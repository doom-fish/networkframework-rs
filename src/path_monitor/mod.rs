//! [`PathMonitor`] — observe network reachability and interface
//! changes via `nw_path_monitor`.

use core::ffi::c_void;
use core::ptr;
use std::sync::Arc;
use std::sync::Mutex;

use doom_fish_utils::panic_safe::catch_user_panic;

use crate::ffi;
use crate::interface::{list_interfaces_for_monitor, NetworkInterface};

pub use crate::interface::InterfaceType;

/// One network-path update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathUpdate {
    /// `true` if the system thinks the network can route traffic.
    pub satisfied: bool,
    pub interface: InterfaceType,
}

type PathCb = Mutex<Box<dyn FnMut(PathUpdate) + Send + 'static>>;
type CancelCb = Mutex<Box<dyn FnMut() + Send + 'static>>;

/// RAII guard for a running `nw_path_monitor`. Drop to stop receiving
/// updates.
#[allow(clippy::type_complexity)]
pub struct PathMonitor {
    handle: *mut c_void,
    callback_raw: *const PathCb,
    cancel_raw: *const CancelCb,
}

// SAFETY: Network.framework serializes monitor callbacks on the monitor queue.
// The raw callback pointers are only dereferenced while the monitor handle is
// live, and `Drop` stops the monitor, drains the queue, and only then reclaims
// the pointers.
unsafe impl Send for PathMonitor {}
// SAFETY: Shared references only forward to the shim. The raw callback
// pointers remain valid until `Drop` stops the monitor and reclaims them after
// the queue is idle.
unsafe impl Sync for PathMonitor {}

fn reclaim_arc_raw<T>(raw: &mut *const T) {
    if !raw.is_null() {
        // SAFETY: `*raw` was produced by `Arc::into_raw`, and the caller only
        // invokes this helper after the shim has stopped using the pointer.
        unsafe {
            drop(Arc::from_raw(*raw));
        }
        *raw = ptr::null();
    }
}

impl PathMonitor {
    /// List the interfaces visible to the most recent path snapshot.
    #[must_use]
    pub fn list_interfaces(&self) -> Vec<NetworkInterface> {
        list_interfaces_for_monitor(self.handle)
    }

    /// Copy the latest path snapshot observed by the monitor.
    #[must_use]
    pub fn current_path(&self) -> Option<crate::path::Path> {
        // SAFETY: `self.handle` is either null (the shim returns null) or a
        // live path-monitor handle produced by the shim.
        let handle = unsafe { ffi::nw_shim_path_monitor_copy_latest_path(self.handle) };
        if handle.is_null() {
            None
        } else {
            // SAFETY: the shim returns a retained `nw_path_t` ownership token
            // for the caller to wrap.
            Some(unsafe { crate::path::Path::from_raw(handle) })
        }
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    /// Prevent the monitor from considering paths that use an interface type.
    pub fn prohibit_interface_type(&mut self, interface_type: InterfaceType) -> &mut Self {
        // SAFETY: `self.handle` is the live monitor handle owned by this
        // `PathMonitor`, and `interface_type` is a valid shim enum value.
        unsafe {
            ffi::nw_shim_path_monitor_prohibit_interface_type(self.handle, interface_type.as_raw());
        }
        self
    }

    /// Receive a callback when the monitor is cancelled.
    pub fn set_cancel_handler<F>(&mut self, callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        if !self.cancel_raw.is_null() {
            if !self.handle.is_null() {
                // SAFETY: `self.handle` is a live monitor handle. Clearing the
                // cancel handler and draining the queue ensures no queued
                // callback can still observe `self.cancel_raw`.
                unsafe {
                    ffi::nw_shim_path_monitor_set_cancel_handler(
                        self.handle,
                        None,
                        ptr::null_mut(),
                    );
                    ffi::nw_shim_path_monitor_drain_queue(self.handle);
                }
            }
            reclaim_arc_raw(&mut self.cancel_raw);
        }

        let callback: Box<dyn FnMut() + Send + 'static> = Box::new(callback);
        let cancel_raw = Arc::into_raw(Arc::new(Mutex::new(callback)));
        if self.handle.is_null() {
            self.cancel_raw = cancel_raw;
            return;
        }

        // SAFETY: `self.handle` is a live monitor handle, and `cancel_raw`
        // points at an `Arc` allocation that stays valid until we clear the
        // handler and reclaim it.
        unsafe {
            ffi::nw_shim_path_monitor_set_cancel_handler(
                self.handle,
                Some(cancel_trampoline),
                cancel_raw.cast::<c_void>().cast_mut(),
            );
        }
        self.cancel_raw = cancel_raw;
    }
}

impl Drop for PathMonitor {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: `self.handle` is the live monitor handle owned by this
            // value. `nw_shim_path_monitor_stop` cancels the monitor and drains
            // its queue before returning, so no more callbacks can fire.
            unsafe {
                ffi::nw_shim_path_monitor_stop(self.handle);
            }
            self.handle = ptr::null_mut();
        }
        reclaim_arc_raw(&mut self.callback_raw);
        reclaim_arc_raw(&mut self.cancel_raw);
    }
}

unsafe extern "C" fn trampoline(satisfied: i32, interface_type: i32, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }

    // SAFETY: `user_info` is the stable pointer created by `Arc::into_raw` in
    // the constructor and remains valid until `Drop` stops the monitor and
    // reclaims it.
    let callback = unsafe { &*user_info.cast::<PathCb>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    catch_user_panic("path_monitor_trampoline", || {
        guard(PathUpdate {
            satisfied: satisfied != 0,
            interface: InterfaceType::from_raw(interface_type),
        });
    });
}

unsafe extern "C" fn cancel_trampoline(user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }

    // SAFETY: `user_info` is the stable pointer created by `Arc::into_raw` in
    // `set_cancel_handler` and remains valid until the handler is cleared and
    // reclaimed.
    let callback = unsafe { &*user_info.cast::<CancelCb>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    catch_user_panic("path_monitor_cancel_trampoline", &mut *guard);
}

/// Start a path monitor. The closure fires whenever Apple reports a
/// network-state change (Wi-Fi connect/disconnect, cellular fallback,
/// airplane mode, etc.).
#[must_use]
pub fn start_path_monitor<F>(callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    let boxed: Box<dyn FnMut(PathUpdate) + Send + 'static> = Box::new(callback);
    let callback_raw = Arc::into_raw(Arc::new(Mutex::new(boxed)));
    // SAFETY: `callback_raw` points at the leaked `Arc` allocation that the
    // shim will hand back to `trampoline` until `Drop` stops the monitor.
    let handle = unsafe {
        ffi::nw_shim_path_monitor_start(trampoline, callback_raw.cast::<c_void>().cast_mut())
    };
    PathMonitor {
        handle,
        callback_raw,
        cancel_raw: ptr::null(),
    }
}

/// Start a path monitor restricted to a specific interface type.
#[must_use]
pub fn start_path_monitor_with_type<F>(interface_type: InterfaceType, callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    let boxed: Box<dyn FnMut(PathUpdate) + Send + 'static> = Box::new(callback);
    let callback_raw = Arc::into_raw(Arc::new(Mutex::new(boxed)));
    // SAFETY: `callback_raw` points at the leaked `Arc` allocation that the
    // shim will hand back to `trampoline` until `Drop` stops the monitor.
    let handle = unsafe {
        ffi::nw_shim_path_monitor_start_with_type(
            interface_type.as_raw(),
            trampoline,
            callback_raw.cast::<c_void>().cast_mut(),
        )
    };
    PathMonitor {
        handle,
        callback_raw,
        cancel_raw: ptr::null(),
    }
}

/// Start a path monitor associated with ethernet-channel reachability.
#[must_use]
pub fn start_path_monitor_for_ethernet_channel<F>(callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    let boxed: Box<dyn FnMut(PathUpdate) + Send + 'static> = Box::new(callback);
    let callback_raw = Arc::into_raw(Arc::new(Mutex::new(boxed)));
    // SAFETY: `callback_raw` points at the leaked `Arc` allocation that the
    // shim will hand back to `trampoline` until `Drop` stops the monitor.
    let handle = unsafe {
        ffi::nw_shim_path_monitor_start_for_ethernet_channel(
            trampoline,
            callback_raw.cast::<c_void>().cast_mut(),
        )
    };
    PathMonitor {
        handle,
        callback_raw,
        cancel_raw: ptr::null(),
    }
}
