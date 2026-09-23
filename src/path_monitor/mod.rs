//! [`PathMonitor`] — observe network reachability and interface
//! changes via `nw_path_monitor`.

use core::ffi::c_void;
use core::ptr;
use std::sync::Arc;
use std::sync::Mutex;

use doom_fish_utils::callback_context::CallbackContext;
use doom_fish_utils::panic_safe::catch_user_panic;

use crate::context::{release_arc, retain_arc};
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
pub struct PathMonitor {
    handle: *mut c_void,
    updates: CallbackContext<PathCb>,
    cancel_token: u64,
}

// SAFETY: the shim handle is reference counted and stays alive until the
// monitor's cancel handler, its final event, has run. Callback contexts are
// retained by the shim for as long as it can invoke them.
unsafe impl Send for PathMonitor {}
// SAFETY: shared references only read the latest path under the shim's lock
// or forward to Network.framework; handler replacement requires `&mut self`.
unsafe impl Sync for PathMonitor {}

impl std::fmt::Debug for PathMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathMonitor")
            .field("handle", &self.handle)
            .field("updates", &self.updates)
            .field("has_cancel_handler", &(self.cancel_token != 0))
            .finish_non_exhaustive()
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

    #[cfg(feature = "async")]
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
        if self.cancel_token != 0 {
            unsafe { ffi::nw_shim_path_monitor_unsubscribe(self.handle, self.cancel_token) };
            self.cancel_token = 0;
        }
        let callback: Box<dyn FnMut() + Send + 'static> = Box::new(callback);
        let callback: Arc<CancelCb> = Arc::new(Mutex::new(callback));
        self.cancel_token = unsafe {
            ffi::nw_shim_path_monitor_subscribe_cancel(
                self.handle,
                Some(cancel_trampoline),
                Arc::into_raw(callback).cast_mut().cast(),
                Some(retain_arc::<CancelCb>),
                Some(release_arc::<CancelCb>),
            )
        };
    }
}

impl Drop for PathMonitor {
    fn drop(&mut self) {
        self.updates.deactivate();
        if !self.handle.is_null() {
            // SAFETY: `self.handle` is the live monitor handle owned by this
            // value. Stopping cancels the monitor and releases this value's
            // reference; the shim frees the handle after the cancel handler.
            unsafe {
                ffi::nw_shim_path_monitor_stop(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}

unsafe extern "C" fn trampoline(satisfied: i32, interface_type: i32, context: *mut c_void) {
    let update = PathUpdate {
        satisfied: satisfied != 0,
        interface: InterfaceType::from_raw(interface_type),
    };
    // SAFETY: `context` is the retained callback context registered when the
    // monitor started; the shim keeps it alive while it can call this.
    unsafe {
        CallbackContext::<PathCb>::with(context, "path_monitor_trampoline", |callback| {
            if let Ok(mut callback) = callback.lock() {
                callback(update);
            }
        })
    };
}

unsafe extern "C" fn cancel_trampoline(context: *mut c_void) {
    if context.is_null() {
        return;
    }
    // SAFETY: `context` is the `Arc` handed to the shim in
    // `set_cancel_handler`; the shim holds a reference while calling this.
    let callback = unsafe { &*context.cast::<CancelCb>() };
    catch_user_panic("path_monitor_cancel_trampoline", || {
        if let Ok(mut callback) = callback.lock() {
            callback();
        }
    });
}

fn start_monitor(
    callback: Box<dyn FnMut(PathUpdate) + Send + 'static>,
    start: impl FnOnce(*mut c_void) -> *mut c_void,
) -> PathMonitor {
    let updates: CallbackContext<PathCb> = CallbackContext::new(Mutex::new(callback));
    let handle = start(updates.retained_ptr());
    PathMonitor {
        handle,
        updates,
        cancel_token: 0,
    }
}

/// Start a path monitor. The closure fires whenever Apple reports a
/// network-state change (Wi-Fi connect/disconnect, cellular fallback,
/// airplane mode, etc.).
#[must_use]
pub fn start_path_monitor<F>(callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    start_monitor(Box::new(callback), |context| unsafe {
        ffi::nw_shim_path_monitor_start(
            Some(trampoline),
            context,
            Some(CallbackContext::<PathCb>::RETAIN),
            Some(CallbackContext::<PathCb>::RELEASE),
        )
    })
}

/// Start a path monitor restricted to a specific interface type.
#[must_use]
pub fn start_path_monitor_with_type<F>(interface_type: InterfaceType, callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    start_monitor(Box::new(callback), |context| unsafe {
        ffi::nw_shim_path_monitor_start_with_type(
            interface_type.as_raw(),
            Some(trampoline),
            context,
            Some(CallbackContext::<PathCb>::RETAIN),
            Some(CallbackContext::<PathCb>::RELEASE),
        )
    })
}

/// Start a path monitor associated with ethernet-channel reachability.
#[must_use]
pub fn start_path_monitor_for_ethernet_channel<F>(callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    start_monitor(Box::new(callback), |context| unsafe {
        ffi::nw_shim_path_monitor_start_for_ethernet_channel(
            Some(trampoline),
            context,
            Some(CallbackContext::<PathCb>::RETAIN),
            Some(CallbackContext::<PathCb>::RELEASE),
        )
    })
}
