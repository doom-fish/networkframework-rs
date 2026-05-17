//! [`PathMonitor`] — observe network reachability and interface
//! changes via `nw_path_monitor`.

use core::ffi::c_void;
use std::sync::Arc;
use std::sync::Mutex;

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
    _callback: Arc<PathCb>,
    cancel_callback: Option<Arc<CancelCb>>,
}

unsafe impl Send for PathMonitor {}
unsafe impl Sync for PathMonitor {}

impl PathMonitor {
    /// List the interfaces visible to the most recent path snapshot.
    #[must_use]
    pub fn list_interfaces(&self) -> Vec<NetworkInterface> {
        list_interfaces_for_monitor(self.handle)
    }

    /// Copy the latest path snapshot observed by the monitor.
    #[must_use]
    pub fn current_path(&self) -> Option<crate::path::Path> {
        let handle = unsafe { ffi::nw_shim_path_monitor_copy_latest_path(self.handle) };
        (!handle.is_null()).then_some(unsafe { crate::path::Path::from_raw(handle) })
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    /// Prevent the monitor from considering paths that use an interface type.
    pub fn prohibit_interface_type(&mut self, interface_type: InterfaceType) -> &mut Self {
        unsafe {
            ffi::nw_shim_path_monitor_prohibit_interface_type(self.handle, interface_type.as_raw());
        };
        self
    }

    /// Receive a callback when the monitor is cancelled.
    pub fn set_cancel_handler<F>(&mut self, callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        let callback: Box<dyn FnMut() + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_path_monitor_set_cancel_handler(self.handle, Some(cancel_trampoline), raw);
        };
        self.cancel_callback = Some(arc);
    }
}

impl Drop for PathMonitor {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_path_monitor_stop(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

unsafe extern "C" fn trampoline(satisfied: i32, interface_type: i32, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let arc_ptr = user_info.cast::<PathCb>();
    let Ok(mut guard) = (unsafe { &*arc_ptr }).lock() else {
        return;
    };
    guard(PathUpdate {
        satisfied: satisfied != 0,
        interface: InterfaceType::from_raw(interface_type),
    });
}

unsafe extern "C" fn cancel_trampoline(user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<CancelCb>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    guard();
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
    let arc: Arc<PathCb> = Arc::new(Mutex::new(boxed));
    let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
    let handle = unsafe { ffi::nw_shim_path_monitor_start(trampoline, raw) };
    PathMonitor {
        handle,
        _callback: arc,
        cancel_callback: None,
    }
}

/// Start a path monitor restricted to a specific interface type.
#[must_use]
pub fn start_path_monitor_with_type<F>(interface_type: InterfaceType, callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    let boxed: Box<dyn FnMut(PathUpdate) + Send + 'static> = Box::new(callback);
    let arc: Arc<PathCb> = Arc::new(Mutex::new(boxed));
    let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
    let handle = unsafe {
        ffi::nw_shim_path_monitor_start_with_type(interface_type.as_raw(), trampoline, raw)
    };
    PathMonitor {
        handle,
        _callback: arc,
        cancel_callback: None,
    }
}

/// Start a path monitor associated with ethernet-channel reachability.
#[must_use]
pub fn start_path_monitor_for_ethernet_channel<F>(callback: F) -> PathMonitor
where
    F: FnMut(PathUpdate) + Send + 'static,
{
    let boxed: Box<dyn FnMut(PathUpdate) + Send + 'static> = Box::new(callback);
    let arc: Arc<PathCb> = Arc::new(Mutex::new(boxed));
    let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
    let handle = unsafe { ffi::nw_shim_path_monitor_start_for_ethernet_channel(trampoline, raw) };
    PathMonitor {
        handle,
        _callback: arc,
        cancel_callback: None,
    }
}
