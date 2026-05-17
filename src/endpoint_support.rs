#![allow(clippy::semicolon_if_nothing_returned)]

use crate::{endpoint::Endpoint, ffi, txt_record::TxtRecord};

impl Endpoint {
    /// Copy the raw `sockaddr` backing an address endpoint.
    #[must_use]
    pub fn raw_address(&self) -> Option<Vec<u8>> {
        let len =
            unsafe { ffi::nw_shim_endpoint_copy_address(self.as_ptr(), core::ptr::null_mut(), 0) };
        if len == 0 {
            return None;
        }
        let mut bytes = vec![0_u8; len];
        let copied = unsafe {
            ffi::nw_shim_endpoint_copy_address(
                self.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if copied == 0 {
            return None;
        }
        Some(bytes)
    }

    /// Copy the TXT record attached to a Bonjour endpoint, if available.
    #[must_use]
    pub fn txt_record(&self) -> Option<TxtRecord> {
        let handle = unsafe { ffi::nw_shim_endpoint_copy_txt_record(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { TxtRecord::from_raw(handle) })
    }
}
