//! Endpoint helpers backed by `nw_endpoint_t`.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_char, c_void};
use std::ffi::{CStr, CString};

use crate::error::NetworkError;
use crate::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    Invalid,
    Address,
    Host,
    BonjourService,
    Url,
    Unknown(i32),
}

impl EndpointType {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::Address,
            2 => Self::Host,
            3 => Self::BonjourService,
            4 => Self::Url,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug)]
pub struct Endpoint {
    handle: *mut c_void,
}

unsafe impl Send for Endpoint {}
unsafe impl Sync for Endpoint {}

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

unsafe fn copied_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

impl Endpoint {
    pub fn host(host: &str, port: u16) -> Result<Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let handle = unsafe { ffi::nw_shim_endpoint_create_host(host.as_ptr(), port) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create host endpoint".into(),
            ));
        }
        Ok(Self { handle })
    }

    pub fn address(address: &str, port: u16) -> Result<Self, NetworkError> {
        let address = to_cstring(address, "address")?;
        let handle = unsafe { ffi::nw_shim_endpoint_create_address(address.as_ptr(), port) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create address endpoint".into(),
            ));
        }
        Ok(Self { handle })
    }

    pub fn bonjour_service(
        name: Option<&str>,
        service_type: &str,
        domain: Option<&str>,
    ) -> Result<Self, NetworkError> {
        let service_type = to_cstring(service_type, "service_type")?;
        let name = match name {
            Some(value) => Some(to_cstring(value, "name")?),
            None => None,
        };
        let domain = match domain {
            Some(value) => Some(to_cstring(value, "domain")?),
            None => None,
        };
        let handle = unsafe {
            ffi::nw_shim_endpoint_create_bonjour_service(
                name.as_ref()
                    .map_or(core::ptr::null(), |value| value.as_ptr()),
                service_type.as_ptr(),
                domain
                    .as_ref()
                    .map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create bonjour service endpoint".into(),
            ));
        }
        Ok(Self { handle })
    }

    pub fn url(url: &str) -> Result<Self, NetworkError> {
        let url = to_cstring(url, "url")?;
        let handle = unsafe { ffi::nw_shim_endpoint_create_url(url.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create URL endpoint".into(),
            ));
        }
        Ok(Self { handle })
    }

    #[must_use]
    pub fn endpoint_type(&self) -> EndpointType {
        EndpointType::from_raw(unsafe { ffi::nw_shim_endpoint_get_type(self.handle) })
    }

    #[must_use]
    pub fn hostname(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_endpoint_copy_hostname(self.handle)) }
    }

    #[must_use]
    pub fn port_string(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_endpoint_copy_port_string(self.handle)) }
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        unsafe { ffi::nw_shim_endpoint_get_port(self.handle) }
    }

    #[must_use]
    pub fn address_string(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_endpoint_copy_address_string(self.handle)) }
    }

    #[must_use]
    pub fn bonjour_service_name(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_endpoint_copy_bonjour_service_name(self.handle)) }
    }

    #[must_use]
    pub fn bonjour_service_type(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_endpoint_copy_bonjour_service_type(self.handle)) }
    }

    #[must_use]
    pub fn bonjour_service_domain(&self) -> Option<String> {
        unsafe {
            copied_string(ffi::nw_shim_endpoint_copy_bonjour_service_domain(
                self.handle,
            ))
        }
    }

    #[must_use]
    pub fn url_string(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_endpoint_copy_url(self.handle)) }
    }

    #[must_use]
    pub fn signature(&self) -> Option<Vec<u8>> {
        let mut len = 0_usize;
        let ptr = unsafe { ffi::nw_shim_endpoint_copy_signature(self.handle, &mut len) };
        if ptr.is_null() || len == 0 {
            return None;
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
        unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
        Some(bytes)
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Clone for Endpoint {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
