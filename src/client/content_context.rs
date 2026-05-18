//! Explicit `nw_content_context` support for message-oriented sends and receives.

#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::c_void;
use std::ffi::{CStr, CString};

use crate::error::NetworkError;
use crate::ffi;
use crate::protocol::{ProtocolDefinition, ProtocolMetadata};

/// One received payload and its associated content context.
#[derive(Debug, Clone)]
pub struct ReceivedContent {
    pub data: Vec<u8>,
    pub context: Option<ContentContext>,
    pub is_complete: bool,
}

/// Builder and wrapper for `nw_content_context_t`.
pub struct ContentContext {
    handle: *mut c_void,
}

unsafe impl Send for ContentContext {}
unsafe impl Sync for ContentContext {}

impl std::fmt::Debug for ContentContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentContext")
            .field("handle", &self.handle)
            .field("identifier", &self.identifier())
            .field("is_final", &self.is_final())
            .finish()
    }
}

impl ContentContext {
    /// Create a new content context with a descriptive identifier.
    pub fn new(identifier: &str) -> Result<Self, NetworkError> {
        let identifier = CString::new(identifier)
            .map_err(|e| NetworkError::InvalidArgument(format!("identifier NUL byte: {e}")))?;
        let handle = unsafe { ffi::nw_shim_content_context_create(identifier.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create content context".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Return the context identifier.
    #[must_use]
    pub fn identifier(&self) -> String {
        let ptr = unsafe { ffi::nw_shim_content_context_get_identifier(self.handle) };
        if ptr.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    /// Whether this context marks the final message on a connection.
    #[must_use]
    pub fn is_final(&self) -> bool {
        unsafe { ffi::nw_shim_content_context_get_is_final(self.handle) != 0 }
    }

    /// Mark the context as the final message on the connection.
    pub fn set_is_final(&mut self, is_final: bool) -> &mut Self {
        unsafe { ffi::nw_shim_content_context_set_is_final(self.handle, i32::from(is_final)) };
        self
    }

    /// Current expiration in milliseconds.
    #[must_use]
    pub fn expiration_milliseconds(&self) -> u64 {
        unsafe { ffi::nw_shim_content_context_get_expiration_milliseconds(self.handle) }
    }

    /// Set the expiration, in milliseconds, after which queued content may be dropped.
    pub fn set_expiration_milliseconds(&mut self, expiration_milliseconds: u64) -> &mut Self {
        unsafe {
            ffi::nw_shim_content_context_set_expiration_milliseconds(
                self.handle,
                expiration_milliseconds,
            )
        };
        self
    }

    /// Relative priority between `0.0` and `1.0`.
    #[must_use]
    pub fn relative_priority(&self) -> f64 {
        unsafe { ffi::nw_shim_content_context_get_relative_priority(self.handle) }
    }

    /// Set the relative priority between `0.0` and `1.0`.
    pub fn set_relative_priority(&mut self, relative_priority: f64) -> &mut Self {
        unsafe {
            ffi::nw_shim_content_context_set_relative_priority(self.handle, relative_priority)
        };
        self
    }

    /// Make another content context an antecedent for this one.
    pub fn set_antecedent(&mut self, antecedent: Option<&Self>) -> &mut Self {
        unsafe {
            ffi::nw_shim_content_context_set_antecedent(
                self.handle,
                antecedent.map_or(core::ptr::null_mut(), Self::as_ptr),
            )
        };
        self
    }

    /// Copy the antecedent context, if one exists.
    #[must_use]
    pub fn copy_antecedent(&self) -> Option<Self> {
        let handle = unsafe { ffi::nw_shim_content_context_copy_antecedent(self.handle) };
        if handle.is_null() {
            return None;
        }
        Some(Self { handle })
    }

    /// Attach framer metadata to this content context.
    pub fn set_framer_message(&mut self, message: &crate::framer::FramerMessage) -> &mut Self {
        unsafe {
            ffi::nw_shim_content_context_set_protocol_metadata(self.handle, message.as_ptr())
        };
        self
    }

    /// Copy framer metadata associated with this context.
    #[must_use]
    pub fn copy_framer_message(
        &self,
        framer_options: &crate::framer::FramerOptions,
    ) -> Option<crate::framer::FramerMessage> {
        let handle = unsafe {
            ffi::nw_shim_content_context_copy_protocol_metadata_for_options(
                self.handle,
                framer_options.as_ptr(),
            )
        };
        if handle.is_null() {
            return None;
        }
        Some(unsafe { crate::framer::FramerMessage::from_raw(handle) })
    }

    /// Enumerate every protocol metadata object currently attached to this context.
    #[must_use]
    pub fn protocol_metadata_entries(&self) -> Vec<(ProtocolDefinition, ProtocolMetadata)> {
        unsafe extern "C" fn collect(
            definition: *mut c_void,
            metadata: *mut c_void,
            user_info: *mut c_void,
        ) -> i32 {
            if user_info.is_null() || definition.is_null() || metadata.is_null() {
                return 0;
            }
            let entries =
                unsafe { &mut *user_info.cast::<Vec<(ProtocolDefinition, ProtocolMetadata)>>() };
            entries.push((
                unsafe { ProtocolDefinition::from_raw(definition) },
                unsafe { ProtocolMetadata::from_raw(metadata) },
            ));
            1
        }

        let mut entries = Vec::new();
        unsafe {
            ffi::nw_shim_content_context_foreach_protocol_metadata(
                self.handle,
                Some(collect),
                std::ptr::addr_of_mut!(entries).cast(),
            )
        };
        entries
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

impl Clone for ContentContext {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ContentContext {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
