//! [`Browser`] — Bonjour and application-service discovery via `nw_browser`.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};

use crate::error::NetworkError;
use crate::ffi;
use crate::parameters::ConnectionParameters;

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

/// A descriptor describing what a browser should look for.
pub struct BrowseDescriptor {
    handle: *mut c_void,
}

unsafe impl Send for BrowseDescriptor {}
unsafe impl Sync for BrowseDescriptor {}

fn copied_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

impl BrowseDescriptor {
    /// Create a Bonjour browse descriptor.
    pub fn bonjour_service(service_type: &str, domain: Option<&str>) -> Result<Self, NetworkError> {
        let service_type = to_cstring(service_type, "service_type")?;
        let domain = match domain {
            Some(domain) => Some(to_cstring(domain, "domain")?),
            None => None,
        };
        let handle = unsafe {
            ffi::nw_shim_browse_descriptor_create_bonjour_service(
                service_type.as_ptr(),
                domain
                    .as_ref()
                    .map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create browse descriptor".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create an application-service browse descriptor.
    pub fn application_service(name: &str) -> Result<Self, NetworkError> {
        let name = to_cstring(name, "name")?;
        let handle =
            unsafe { ffi::nw_shim_browse_descriptor_create_application_service(name.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create application-service browse descriptor".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Bonjour service type, if this is a Bonjour browse descriptor.
    #[must_use]
    pub fn bonjour_service_type(&self) -> Option<String> {
        copied_string(unsafe {
            ffi::nw_shim_browse_descriptor_copy_bonjour_service_type(self.handle)
        })
    }

    /// Bonjour service domain, if explicitly configured.
    #[must_use]
    pub fn bonjour_service_domain(&self) -> Option<String> {
        copied_string(unsafe {
            ffi::nw_shim_browse_descriptor_copy_bonjour_service_domain(self.handle)
        })
    }

    /// Enable or disable TXT-record inclusion during browsing.
    pub fn set_include_txt_record(&mut self, include_txt_record: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_browse_descriptor_set_include_txt_record(
                self.handle,
                i32::from(include_txt_record),
            );
        };
        self
    }

    /// Whether TXT-record inclusion is enabled.
    #[must_use]
    pub fn include_txt_record(&self) -> bool {
        unsafe { ffi::nw_shim_browse_descriptor_get_include_txt_record(self.handle) != 0 }
    }

    /// Application-service name, if this is an application-service descriptor.
    #[must_use]
    pub fn application_service_name(&self) -> Option<String> {
        copied_string(unsafe {
            ffi::nw_shim_browse_descriptor_copy_application_service_name(self.handle)
        })
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Clone for BrowseDescriptor {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for BrowseDescriptor {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
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
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// Start browsing with an explicit descriptor and optional parameters.
pub fn start_browser_with_descriptor<F>(
    descriptor: &BrowseDescriptor,
    parameters: Option<&ConnectionParameters>,
    callback: F,
) -> Result<Browser, NetworkError>
where
    F: FnMut(BrowserEvent) + Send + 'static,
{
    let boxed: Box<dyn FnMut(BrowserEvent) + Send + 'static> = Box::new(callback);
    let arc: Arc<Cb> = Arc::new(Mutex::new(boxed));
    let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
    let handle = unsafe {
        ffi::nw_shim_browser_start_with_descriptor(
            descriptor.as_ptr(),
            parameters.map_or(core::ptr::null_mut(), ConnectionParameters::as_ptr),
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

/// Start browsing for Bonjour services of `service_type` in `domain`.
pub fn start_browser<F>(
    service_type: &str,
    domain: Option<&str>,
    callback: F,
) -> Result<Browser, NetworkError>
where
    F: FnMut(BrowserEvent) + Send + 'static,
{
    let descriptor = BrowseDescriptor::bonjour_service(service_type, domain)?;
    start_browser_with_descriptor(&descriptor, None, callback)
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

/// Publish a Bonjour service on the local network.
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
