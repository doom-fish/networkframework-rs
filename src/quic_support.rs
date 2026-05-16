#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::c_void;
use std::ffi::{CStr, CString};

use crate::{
    client::ContentContext,
    error::NetworkError,
    ffi,
    quic::{QuicConnection, QuicOptions},
};

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

unsafe fn copied_string(ptr: *mut i8) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

/// Opaque `sec_protocol_options_t` extracted from QUIC options.
pub struct SecurityProtocolOptions {
    handle: *mut c_void,
}

unsafe impl Send for SecurityProtocolOptions {}
unsafe impl Sync for SecurityProtocolOptions {}

impl SecurityProtocolOptions {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Clone for SecurityProtocolOptions {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_sec_retain(self.handle) };
        Self { handle }
    }
}

impl Drop for SecurityProtocolOptions {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_sec_release(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Opaque `sec_protocol_metadata_t` extracted from QUIC metadata.
pub struct SecurityProtocolMetadata {
    handle: *mut c_void,
}

unsafe impl Send for SecurityProtocolMetadata {}
unsafe impl Sync for SecurityProtocolMetadata {}

impl SecurityProtocolMetadata {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Clone for SecurityProtocolMetadata {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_sec_retain(self.handle) };
        Self { handle }
    }
}

impl Drop for SecurityProtocolMetadata {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_sec_release(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// QUIC stream direction or datagram mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicStreamType {
    Unknown,
    Bidirectional,
    Unidirectional,
    Datagram,
    Other(i32),
}

impl QuicStreamType {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::Bidirectional,
            2 => Self::Unidirectional,
            3 => Self::Datagram,
            other => Self::Other(other),
        }
    }
}

/// QUIC protocol metadata attached to a connection or content context.
pub struct QuicMetadata {
    handle: *mut c_void,
}

unsafe impl Send for QuicMetadata {}
unsafe impl Sync for QuicMetadata {}

impl QuicMetadata {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    /// Whether this metadata object represents QUIC protocol metadata.
    #[must_use]
    pub fn is_quic(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_metadata_is_quic(self.handle) != 0 }
    }

    /// Copy the associated security metadata.
    #[must_use]
    pub fn security_metadata(&self) -> Option<SecurityProtocolMetadata> {
        let handle = unsafe { ffi::nw_shim_quic_copy_sec_protocol_metadata(self.handle) };
        (!handle.is_null()).then_some(unsafe { SecurityProtocolMetadata::from_raw(handle) })
    }

    /// QUIC stream identifier, if present.
    #[must_use]
    pub fn stream_id(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_stream_id(self.handle) }
    }

    /// QUIC stream direction or datagram mode.
    #[must_use]
    pub fn stream_type(&self) -> QuicStreamType {
        QuicStreamType::from_raw(unsafe { ffi::nw_shim_quic_get_stream_type(self.handle) })
    }

    /// Application error associated with the current stream, if any.
    #[must_use]
    pub fn stream_application_error(&self) -> Option<u64> {
        let error = unsafe { ffi::nw_shim_quic_get_stream_application_error(self.handle) };
        (error != u64::MAX).then_some(error)
    }

    /// Set the current stream application's error code.
    pub fn set_stream_application_error(&mut self, application_error: u64) -> &mut Self {
        unsafe { ffi::nw_shim_quic_set_stream_application_error(self.handle, application_error) };
        self
    }

    /// Local bidirectional stream limit advertised to the peer.
    #[must_use]
    pub fn local_max_streams_bidirectional(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_local_max_streams_bidirectional(self.handle) }
    }

    /// Update the local bidirectional stream limit.
    pub fn set_local_max_streams_bidirectional(&mut self, max_streams_bidirectional: u64) -> &mut Self {
        unsafe {
            ffi::nw_shim_quic_set_local_max_streams_bidirectional(
                self.handle,
                max_streams_bidirectional,
            )
        };
        self
    }

    /// Local unidirectional stream limit advertised to the peer.
    #[must_use]
    pub fn local_max_streams_unidirectional(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_local_max_streams_unidirectional(self.handle) }
    }

    /// Update the local unidirectional stream limit.
    pub fn set_local_max_streams_unidirectional(&mut self, max_streams_unidirectional: u64) -> &mut Self {
        unsafe {
            ffi::nw_shim_quic_set_local_max_streams_unidirectional(
                self.handle,
                max_streams_unidirectional,
            )
        };
        self
    }

    /// Remote bidirectional stream limit.
    #[must_use]
    pub fn remote_max_streams_bidirectional(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_remote_max_streams_bidirectional(self.handle) }
    }

    /// Remote unidirectional stream limit.
    #[must_use]
    pub fn remote_max_streams_unidirectional(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_remote_max_streams_unidirectional(self.handle) }
    }

    /// Maximum usable QUIC datagram frame size.
    #[must_use]
    pub fn stream_usable_datagram_frame_size(&self) -> u16 {
        unsafe { ffi::nw_shim_quic_get_stream_usable_datagram_frame_size(self.handle) }
    }

    /// Connection application error, if any.
    #[must_use]
    pub fn application_error(&self) -> Option<u64> {
        let error = unsafe { ffi::nw_shim_quic_get_application_error(self.handle) };
        (error != u64::MAX).then_some(error)
    }

    /// Human-readable application error reason, if one was attached.
    #[must_use]
    pub fn application_error_reason(&self) -> Option<String> {
        unsafe { copied_string(ffi::nw_shim_quic_copy_application_error_reason(self.handle)) }
    }

    /// Set the connection application error code and optional reason.
    pub fn set_application_error(
        &mut self,
        application_error: u64,
        reason: Option<&str>,
    ) -> Result<&mut Self, NetworkError> {
        let reason = reason.map(|reason| to_cstring(reason, "reason")).transpose()?;
        unsafe {
            ffi::nw_shim_quic_set_application_error(
                self.handle,
                application_error,
                reason.as_ref().map_or(core::ptr::null(), |reason| reason.as_ptr()),
            )
        };
        Ok(self)
    }

    /// Current QUIC keepalive interval in seconds.
    #[must_use]
    pub fn keepalive_interval(&self) -> u16 {
        unsafe { ffi::nw_shim_quic_get_keepalive_interval(self.handle) }
    }

    /// Update the QUIC keepalive interval in seconds.
    pub fn set_keepalive_interval(&mut self, keepalive_interval: u16) -> &mut Self {
        unsafe { ffi::nw_shim_quic_set_keepalive_interval(self.handle, keepalive_interval) };
        self
    }

    /// Remote idle timeout in milliseconds.
    #[must_use]
    pub fn remote_idle_timeout(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_remote_idle_timeout(self.handle) }
    }
}

impl Clone for QuicMetadata {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for QuicMetadata {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl QuicOptions {
    /// Current `initial_max_streams_bidi` transport parameter.
    #[must_use]
    pub fn initial_max_streams_bidirectional(&self) -> u64 {
        let options = self.protocol_options();
        unsafe { ffi::nw_shim_quic_get_initial_max_streams_bidirectional(options.as_ptr()) }
    }

    /// Set the `initial_max_streams_bidi` transport parameter.
    pub fn set_initial_max_streams_bidirectional(
        &mut self,
        initial_max_streams_bidirectional: u64,
    ) -> &mut Self {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_set_initial_max_streams_bidirectional(
                options.as_ptr(),
                initial_max_streams_bidirectional,
            )
        };
        self
    }

    /// Current `initial_max_streams_uni` transport parameter.
    #[must_use]
    pub fn initial_max_streams_unidirectional(&self) -> u64 {
        let options = self.protocol_options();
        unsafe { ffi::nw_shim_quic_get_initial_max_streams_unidirectional(options.as_ptr()) }
    }

    /// Set the `initial_max_streams_uni` transport parameter.
    pub fn set_initial_max_streams_unidirectional(
        &mut self,
        initial_max_streams_unidirectional: u64,
    ) -> &mut Self {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_set_initial_max_streams_unidirectional(
                options.as_ptr(),
                initial_max_streams_unidirectional,
            )
        };
        self
    }

    /// Current local bidirectional stream receive window.
    #[must_use]
    pub fn initial_max_stream_data_bidirectional_local(&self) -> u64 {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_get_initial_max_stream_data_bidirectional_local(options.as_ptr())
        }
    }

    /// Set the local bidirectional stream receive window.
    pub fn set_initial_max_stream_data_bidirectional_local(
        &mut self,
        initial_max_stream_data_bidirectional_local: u64,
    ) -> &mut Self {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_set_initial_max_stream_data_bidirectional_local(
                options.as_ptr(),
                initial_max_stream_data_bidirectional_local,
            )
        };
        self
    }

    /// Current remote bidirectional stream receive window.
    #[must_use]
    pub fn initial_max_stream_data_bidirectional_remote(&self) -> u64 {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_get_initial_max_stream_data_bidirectional_remote(options.as_ptr())
        }
    }

    /// Set the remote bidirectional stream receive window.
    pub fn set_initial_max_stream_data_bidirectional_remote(
        &mut self,
        initial_max_stream_data_bidirectional_remote: u64,
    ) -> &mut Self {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_set_initial_max_stream_data_bidirectional_remote(
                options.as_ptr(),
                initial_max_stream_data_bidirectional_remote,
            )
        };
        self
    }

    /// Current unidirectional stream receive window.
    #[must_use]
    pub fn initial_max_stream_data_unidirectional(&self) -> u64 {
        let options = self.protocol_options();
        unsafe { ffi::nw_shim_quic_get_initial_max_stream_data_unidirectional(options.as_ptr()) }
    }

    /// Set the unidirectional stream receive window.
    pub fn set_initial_max_stream_data_unidirectional(
        &mut self,
        initial_max_stream_data_unidirectional: u64,
    ) -> &mut Self {
        let options = self.protocol_options();
        unsafe {
            ffi::nw_shim_quic_set_initial_max_stream_data_unidirectional(
                options.as_ptr(),
                initial_max_stream_data_unidirectional,
            )
        };
        self
    }

    /// Current maximum QUIC datagram frame size.
    #[must_use]
    pub fn max_datagram_frame_size(&self) -> u16 {
        let options = self.protocol_options();
        unsafe { ffi::nw_shim_quic_get_max_datagram_frame_size(options.as_ptr()) }
    }

    /// Set the maximum QUIC datagram frame size.
    pub fn set_max_datagram_frame_size(&mut self, max_datagram_frame_size: u16) -> &mut Self {
        let options = self.protocol_options();
        unsafe { ffi::nw_shim_quic_set_max_datagram_frame_size(options.as_ptr(), max_datagram_frame_size) };
        self
    }

    /// Copy the underlying security-options object.
    #[must_use]
    pub fn security_options(&self) -> Option<SecurityProtocolOptions> {
        let options = self.protocol_options();
        let handle = unsafe { ffi::nw_shim_quic_copy_sec_protocol_options(options.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { SecurityProtocolOptions::from_raw(handle) })
    }
}

impl QuicConnection {
    /// Copy the connection-level QUIC metadata snapshot.
    #[must_use]
    pub fn metadata(&self) -> Option<QuicMetadata> {
        let handle = unsafe { ffi::nw_shim_connection_copy_quic_metadata(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { QuicMetadata::from_raw(handle) })
    }
}

impl ContentContext {
    /// Attach QUIC metadata to this content context.
    pub fn set_quic_metadata(&mut self, metadata: &QuicMetadata) -> &mut Self {
        unsafe { ffi::nw_shim_content_context_set_protocol_metadata(self.as_ptr(), metadata.as_ptr()) };
        self
    }

    /// Copy QUIC metadata from this content context, if present.
    #[must_use]
    pub fn copy_quic_metadata(&self) -> Option<QuicMetadata> {
        let handle = unsafe { ffi::nw_shim_content_context_copy_quic_metadata(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { QuicMetadata::from_raw(handle) })
    }
}
