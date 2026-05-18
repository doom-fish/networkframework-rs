//! Advertise-descriptor helpers.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};

use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::txt_record::TxtRecord;

pub struct AdvertiseDescriptor {
    handle: *mut c_void,
    bonjour_name: Option<String>,
    bonjour_type: Option<String>,
    bonjour_domain: Option<String>,
    application_service_name: Option<String>,
}

unsafe impl Send for AdvertiseDescriptor {}
unsafe impl Sync for AdvertiseDescriptor {}

impl std::fmt::Debug for AdvertiseDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdvertiseDescriptor")
            .field("handle", &self.handle)
            .field("bonjour_name", &self.bonjour_name)
            .field("bonjour_type", &self.bonjour_type)
            .field("bonjour_domain", &self.bonjour_domain)
            .field("application_service_name", &self.application_service_name)
            .finish()
    }
}

pub struct Advertiser {
    handle: *mut c_void,
}

unsafe impl Send for Advertiser {}
unsafe impl Sync for Advertiser {}

impl std::fmt::Debug for Advertiser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Advertiser")
            .field("handle", &self.handle)
            .finish()
    }
}

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

impl AdvertiseDescriptor {
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
            ffi::nw_shim_advertise_descriptor_create_bonjour_service(
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
                "failed to create advertise descriptor".into(),
            ));
        }
        Ok(Self {
            handle,
            bonjour_name: name.map(|value| value.to_string_lossy().into_owned()),
            bonjour_type: Some(service_type.to_string_lossy().into_owned()),
            bonjour_domain: domain.map(|value| value.to_string_lossy().into_owned()),
            application_service_name: None,
        })
    }

    pub fn application_service(application_service_name: &str) -> Result<Self, NetworkError> {
        let application_service_name =
            to_cstring(application_service_name, "application_service_name")?;
        let handle = unsafe {
            ffi::nw_shim_advertise_descriptor_create_application_service(
                application_service_name.as_ptr(),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create application-service advertise descriptor".into(),
            ));
        }
        Ok(Self {
            handle,
            bonjour_name: None,
            bonjour_type: None,
            bonjour_domain: None,
            application_service_name: Some(application_service_name.to_string_lossy().into_owned()),
        })
    }

    #[must_use]
    pub fn service_name(&self) -> Option<&str> {
        self.bonjour_name.as_deref()
    }

    #[must_use]
    pub fn service_type(&self) -> Option<&str> {
        self.bonjour_type.as_deref()
    }

    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.bonjour_domain.as_deref()
    }

    #[must_use]
    pub fn application_service_name(&self) -> Option<String> {
        if self.application_service_name.is_some() {
            return self.application_service_name.clone();
        }
        unsafe {
            copied_string(
                ffi::nw_shim_advertise_descriptor_copy_application_service_name(self.handle),
            )
        }
    }

    pub fn set_txt_record(&mut self, txt_record: &[u8]) -> &mut Self {
        unsafe {
            ffi::nw_shim_advertise_descriptor_set_txt_record(
                self.handle,
                txt_record.as_ptr(),
                txt_record.len(),
            );
        };
        self
    }

    /// Attach a structured TXT-record object.
    pub fn set_txt_record_object(&mut self, txt_record: &TxtRecord) -> &mut Self {
        unsafe {
            ffi::nw_shim_advertise_descriptor_set_txt_record_object(
                self.handle,
                txt_record.as_ptr(),
            );
        };
        self
    }

    /// Copy the current structured TXT-record object.
    #[must_use]
    pub fn txt_record_object(&self) -> Option<TxtRecord> {
        let handle =
            unsafe { ffi::nw_shim_advertise_descriptor_copy_txt_record_object(self.handle) };
        (!handle.is_null()).then_some(unsafe { TxtRecord::from_raw(handle) })
    }

    pub fn set_no_auto_rename(&mut self, no_auto_rename: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_advertise_descriptor_set_no_auto_rename(
                self.handle,
                c_int::from(no_auto_rename),
            );
        };
        self
    }

    #[must_use]
    pub fn no_auto_rename(&self) -> bool {
        unsafe { ffi::nw_shim_advertise_descriptor_get_no_auto_rename(self.handle) != 0 }
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

pub fn advertise_with_descriptor(
    descriptor: &AdvertiseDescriptor,
    port: u16,
) -> Result<Advertiser, NetworkError> {
    let mut status = 0;
    let handle = if let (Some(service_type), Some(service_name)) = (
        descriptor.bonjour_type.as_deref(),
        descriptor.bonjour_name.as_deref(),
    ) {
        let service_type = to_cstring(service_type, "service_type")?;
        let service_name = to_cstring(service_name, "service_name")?;
        let domain = descriptor
            .bonjour_domain
            .as_deref()
            .map(|value| to_cstring(value, "domain"))
            .transpose()?;
        unsafe {
            ffi::nw_shim_bonjour_advertise_start(
                service_type.as_ptr(),
                service_name.as_ptr(),
                domain
                    .as_ref()
                    .map_or(core::ptr::null(), |value| value.as_ptr()),
                port,
                &mut status,
            )
        }
    } else {
        unsafe {
            ffi::nw_shim_bonjour_advertise_start_with_descriptor(
                descriptor.as_ptr(),
                port,
                &mut status,
            )
        }
    };
    if status != ffi::NW_OK || handle.is_null() {
        return Err(from_status(status));
    }
    Ok(Advertiser { handle })
}

impl Drop for Advertiser {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_bonjour_advertise_stop(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl Clone for AdvertiseDescriptor {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self {
            handle,
            bonjour_name: self.bonjour_name.clone(),
            bonjour_type: self.bonjour_type.clone(),
            bonjour_domain: self.bonjour_domain.clone(),
            application_service_name: self.application_service_name.clone(),
        }
    }
}

impl Drop for AdvertiseDescriptor {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
