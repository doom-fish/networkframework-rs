//! TXT-record helper APIs.

#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};

use crate::{error::NetworkError, ffi};

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

/// Result of looking up a key in a TXT record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxtRecordFindResult {
    Invalid,
    NotPresent,
    NoValue,
    EmptyValue,
    NonEmptyValue,
    Unknown(i32),
}

impl TxtRecordFindResult {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::NotPresent,
            2 => Self::NoValue,
            3 => Self::EmptyValue,
            4 => Self::NonEmptyValue,
            other => Self::Unknown(other),
        }
    }
}

/// One decoded TXT-record entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtRecordEntry {
    pub key: String,
    pub status: TxtRecordFindResult,
    pub value: Option<Vec<u8>>,
}

/// Result of looking up a single TXT-record key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtRecordLookup {
    pub status: TxtRecordFindResult,
    pub value: Option<Vec<u8>>,
}

/// Wrapper around `nw_txt_record_t`.
#[derive(Debug)]
pub struct TxtRecord {
    handle: *mut c_void,
}

unsafe impl Send for TxtRecord {}
unsafe impl Sync for TxtRecord {}

fn value_from_raw(status: TxtRecordFindResult, bytes: *mut u8, len: usize) -> Option<Vec<u8>> {
    if !bytes.is_null() && len > 0 {
        let value = unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec();
        unsafe { ffi::nw_shim_free_buffer(bytes.cast()) };
        return Some(value);
    }
    if !bytes.is_null() {
        unsafe { ffi::nw_shim_free_buffer(bytes.cast()) };
    }
    matches!(status, TxtRecordFindResult::EmptyValue).then(Vec::new)
}

impl TxtRecord {
    /// Create a TXT record from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NetworkError> {
        if bytes.is_empty() {
            return Err(NetworkError::InvalidArgument(
                "TXT record bytes must not be empty".into(),
            ));
        }
        let handle = unsafe { ffi::nw_shim_txt_record_create_with_bytes(bytes.as_ptr(), bytes.len()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create TXT record from bytes".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create an empty dictionary-backed TXT record.
    pub fn dictionary() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_txt_record_create_dictionary() };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create TXT record dictionary".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// # Safety
    ///
    /// `handle` must be a valid retained TXT-record object handle that remains
    /// alive for the returned wrapper.
    #[must_use]
    pub const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    /// Look up a key in the TXT record.
    pub fn find_key(&self, key: &str) -> Result<TxtRecordFindResult, NetworkError> {
        let key = to_cstring(key, "key")?;
        Ok(TxtRecordFindResult::from_raw(unsafe {
            ffi::nw_shim_txt_record_find_key(self.handle, key.as_ptr())
        }))
    }

    /// Read the current value for a key.
    pub fn lookup(&self, key: &str) -> Result<TxtRecordLookup, NetworkError> {
        let key = to_cstring(key, "key")?;
        let mut value_length = 0_usize;
        let mut found: c_int = 0;
        let value = unsafe {
            ffi::nw_shim_txt_record_copy_value(
                self.handle,
                key.as_ptr(),
                &mut value_length,
                &mut found,
            )
        };
        let status = TxtRecordFindResult::from_raw(found);
        Ok(TxtRecordLookup {
            status,
            value: value_from_raw(status, value, value_length),
        })
    }

    /// Insert or replace a key.
    pub fn set_key(&mut self, key: &str, value: Option<&[u8]>) -> Result<&mut Self, NetworkError> {
        let key = to_cstring(key, "key")?;
        let (value_ptr, value_length) = value
            .map_or((core::ptr::null(), 0), |value| (value.as_ptr(), value.len()));
        let set = unsafe {
            ffi::nw_shim_txt_record_set_key(self.handle, key.as_ptr(), value_ptr, value_length)
        };
        if set == 0 {
            return Err(NetworkError::InvalidArgument(
                "failed to set TXT record key".into(),
            ));
        }
        Ok(self)
    }

    /// Remove a key from the TXT record.
    pub fn remove_key(&mut self, key: &str) -> Result<bool, NetworkError> {
        let key = to_cstring(key, "key")?;
        Ok(unsafe { ffi::nw_shim_txt_record_remove_key(self.handle, key.as_ptr()) != 0 })
    }

    /// Number of keys in the TXT record.
    #[must_use]
    pub fn key_count(&self) -> usize {
        unsafe { ffi::nw_shim_txt_record_get_key_count(self.handle) }
    }

    /// Encode the TXT record back to raw bytes.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut len = 0_usize;
        let ptr = unsafe { ffi::nw_shim_txt_record_copy_bytes(self.handle, &mut len) };
        if ptr.is_null() || len == 0 {
            return Vec::new();
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
        unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
        bytes
    }

    /// Enumerate every entry in the TXT record.
    #[must_use]
    pub fn entries(&self) -> Vec<TxtRecordEntry> {
        let mut entries = Vec::new();
        unsafe {
            ffi::nw_shim_txt_record_apply(
                self.handle,
                Some(collect_entry_trampoline),
                std::ptr::addr_of_mut!(entries).cast(),
            )
        };
        entries
    }

    /// Whether the TXT record is dictionary-backed.
    #[must_use]
    pub fn is_dictionary(&self) -> bool {
        unsafe { ffi::nw_shim_txt_record_is_dictionary(self.handle) != 0 }
    }
}

impl Clone for TxtRecord {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_txt_record_copy(self.handle) };
        Self { handle }
    }
}

impl PartialEq for TxtRecord {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::nw_shim_txt_record_is_equal(self.handle, other.handle) != 0 }
    }
}

impl Eq for TxtRecord {}

impl Drop for TxtRecord {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

unsafe extern "C" fn collect_entry_trampoline(
    key: *const c_char,
    found: c_int,
    value: *const u8,
    value_len: usize,
    user_info: *mut c_void,
) -> c_int {
    if user_info.is_null() {
        return 0;
    }
    let entries = unsafe { &mut *user_info.cast::<Vec<TxtRecordEntry>>() };
    let key = if key.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(key) }
            .to_string_lossy()
            .into_owned()
    };
    let status = TxtRecordFindResult::from_raw(found);
    let value = if value.is_null() {
        matches!(status, TxtRecordFindResult::EmptyValue).then(Vec::new)
    } else {
        Some(unsafe { std::slice::from_raw_parts(value, value_len) }.to_vec())
    };
    entries.push(TxtRecordEntry { key, status, value });
    1
}
