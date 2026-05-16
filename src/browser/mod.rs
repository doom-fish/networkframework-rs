//! [`Browser`] — Bonjour service discovery via `nw_browser`.

use core::ffi::{c_char, c_void};
use std::ffi::CString;
use std::sync::{Arc, Mutex};

use crate::error::NetworkError;
use crate::ffi;

/// One Bonjour service that the browser has observed appearing or
/// disappearing on the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredService {
    /// Instance name, e.g. `"Living Room Apple TV"`.
    pub name: String,
    /// Service type with protocol, e.g. `"_airplay._tcp"`.
    pub service_type: String,
    /// DNS domain — usually `"local"`.
    pub domain: String,
}

/// Discovery event delivered to the browser closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserEvent {
    Found(DiscoveredService),
    Lost(DiscoveredService),
}

type Cb = Mutex<Box<dyn FnMut(BrowserEvent) + Send + 'static>>;

/// RAII guard for a running `nw_browser`. Drop to stop receiving
/// discovery callbacks.
#[allow(clippy::type_complexity)]
pub struct Browser {
    handle: *mut c_void,
    _callback: Arc<Cb>,
}

unsafe impl Send for Browser {}
unsafe impl Sync for Browser {}

impl Drop for Browser {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_browser_stop(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

unsafe extern "C" fn found_trampoline(
    name: *const c_char,
    service_type: *const c_char,
    domain: *const c_char,
    user_info: *mut c_void,
) {
    invoke(user_info, name, service_type, domain, true);
}

unsafe extern "C" fn lost_trampoline(
    name: *const c_char,
    service_type: *const c_char,
    domain: *const c_char,
    user_info: *mut c_void,
) {
    invoke(user_info, name, service_type, domain, false);
}

unsafe fn invoke(
    user_info: *mut c_void,
    name: *const c_char,
    service_type: *const c_char,
    domain: *const c_char,
    is_found: bool,
) {
    if user_info.is_null() {
        return;
    }
    let arc_ptr = user_info.cast::<Cb>();
    let svc = DiscoveredService {
        name: cstr_to_string(name),
        service_type: cstr_to_string(service_type),
        domain: cstr_to_string(domain),
    };
    let event = if is_found {
        BrowserEvent::Found(svc)
    } else {
        BrowserEvent::Lost(svc)
    };
    let Ok(mut guard) = (unsafe { &*arc_ptr }).lock() else {
        return;
    };
    guard(event);
}

unsafe fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { core::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

/// Start browsing for Bonjour services of `service_type` (e.g.
/// `"_airplay._tcp"`, `"_http._tcp"`, `"_ipp._tcp"`, …) in `domain`
/// (use `None` for the default `"local."`).
///
/// The closure fires on the browser's internal queue for each new or
/// disappearing service.
///
/// # Errors
///
/// Returns [`NetworkError::InvalidArgument`] if `service_type`
/// contains a NUL byte, [`NetworkError::ListenFailed`] if Apple
/// refuses to create the browser.
pub fn start_browser<F>(
    service_type: &str,
    domain: Option<&str>,
    callback: F,
) -> Result<Browser, NetworkError>
where
    F: FnMut(BrowserEvent) + Send + 'static,
{
    let svc = CString::new(service_type)
        .map_err(|e| NetworkError::InvalidArgument(format!("service_type NUL byte: {e}")))?;
    let dom = match domain {
        Some(d) => Some(
            CString::new(d)
                .map_err(|e| NetworkError::InvalidArgument(format!("domain NUL byte: {e}")))?,
        ),
        None => None,
    };

    let boxed: Box<dyn FnMut(BrowserEvent) + Send + 'static> = Box::new(callback);
    let arc: Arc<Cb> = Arc::new(Mutex::new(boxed));
    let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
    let handle = unsafe {
        ffi::nw_shim_browser_start(
            svc.as_ptr(),
            dom.as_ref().map_or(core::ptr::null(), |c| c.as_ptr()),
            found_trampoline,
            lost_trampoline,
            raw,
        )
    };
    if handle.is_null() {
        unsafe { Arc::from_raw(raw.cast::<Cb>()) };
        return Err(NetworkError::ListenFailed);
    }
    Ok(Browser {
        handle,
        _callback: arc,
    })
}

/// RAII guard for a running Bonjour service advertisement. Drop to
/// stop publishing the service to the local network.
pub struct BonjourAdvertiser {
    handle: *mut c_void,
}

unsafe impl Send for BonjourAdvertiser {}
unsafe impl Sync for BonjourAdvertiser {}

impl Drop for BonjourAdvertiser {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_bonjour_advertise_stop(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Publish a Bonjour service on the local network. The OS announces
/// `service_name` of type `service_type` (e.g. `"_http._tcp"`) on
/// the given TCP `port`.
///
/// Returned [`BonjourAdvertiser`] guard keeps the service alive
/// until dropped. The listener accepts no connections — this is
/// publish-only (use [`crate::TcpListener`] alongside for the
/// accepting side).
///
/// # Errors
///
/// Returns [`NetworkError::InvalidArgument`] on NUL bytes in any
/// string, or [`NetworkError::ListenFailed`] if Apple refuses.
pub fn advertise_bonjour_service(
    service_type: &str,
    service_name: &str,
    domain: Option<&str>,
    port: u16,
) -> Result<BonjourAdvertiser, NetworkError> {
    let svc_type = CString::new(service_type)
        .map_err(|e| NetworkError::InvalidArgument(format!("service_type NUL: {e}")))?;
    let svc_name = CString::new(service_name)
        .map_err(|e| NetworkError::InvalidArgument(format!("service_name NUL: {e}")))?;
    let dom = match domain {
        Some(d) => Some(
            CString::new(d)
                .map_err(|e| NetworkError::InvalidArgument(format!("domain NUL: {e}")))?,
        ),
        None => None,
    };
    let mut status: core::ffi::c_int = 0;
    let handle = unsafe {
        ffi::nw_shim_bonjour_advertise_start(
            svc_type.as_ptr(),
            svc_name.as_ptr(),
            dom.as_ref().map_or(core::ptr::null(), |c| c.as_ptr()),
            port,
            &mut status,
        )
    };
    if status != ffi::NW_OK || handle.is_null() {
        return Err(crate::error::from_status(status));
    }
    Ok(BonjourAdvertiser { handle })
}
