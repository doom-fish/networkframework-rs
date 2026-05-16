//! QUIC helpers backed by Network.framework.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;

use crate::client::{ContentContext, ReceivedContent};
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::{ConnectionParameters, KeepAlives};
use crate::protocol::{ProtocolDefinition, ProtocolOptions};

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

/// QUIC protocol options attachable to a protocol stack.
pub struct QuicOptions {
    handle: *mut c_void,
}

unsafe impl Send for QuicOptions {}
unsafe impl Sync for QuicOptions {}

impl QuicOptions {
    /// Create a fresh QUIC protocol-options object.
    pub fn new() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_protocol_create_quic_options() };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create QUIC options".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Add an ALPN protocol string to the handshake.
    pub fn add_tls_application_protocol(
        &mut self,
        application_protocol: &str,
    ) -> Result<&mut Self, NetworkError> {
        let application_protocol = to_cstring(application_protocol, "application_protocol")?;
        unsafe {
            ffi::nw_shim_quic_add_tls_application_protocol(
                self.handle,
                application_protocol.as_ptr(),
            );
        };
        Ok(self)
    }

    /// Whether the stream is unidirectional.
    #[must_use]
    pub fn stream_is_unidirectional(&self) -> bool {
        unsafe { ffi::nw_shim_quic_get_stream_is_unidirectional(self.handle) != 0 }
    }

    /// Set whether the stream is unidirectional.
    pub fn set_stream_is_unidirectional(&mut self, is_unidirectional: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_quic_set_stream_is_unidirectional(
                self.handle,
                i32::from(is_unidirectional),
            );
        }
        self
    }

    /// Whether the QUIC options are configured as a datagram flow.
    #[must_use]
    pub fn stream_is_datagram(&self) -> bool {
        unsafe { ffi::nw_shim_quic_get_stream_is_datagram(self.handle) != 0 }
    }

    /// Configure the QUIC stream as a datagram flow.
    pub fn set_stream_is_datagram(&mut self, is_datagram: bool) -> &mut Self {
        unsafe { ffi::nw_shim_quic_set_stream_is_datagram(self.handle, i32::from(is_datagram)) };
        self
    }

    /// Current `initial_max_data` transport parameter.
    #[must_use]
    pub fn initial_max_data(&self) -> u64 {
        unsafe { ffi::nw_shim_quic_get_initial_max_data(self.handle) }
    }

    /// Set the `initial_max_data` transport parameter.
    pub fn set_initial_max_data(&mut self, initial_max_data: u64) -> &mut Self {
        unsafe { ffi::nw_shim_quic_set_initial_max_data(self.handle, initial_max_data) };
        self
    }

    /// Current maximum UDP payload size.
    #[must_use]
    pub fn max_udp_payload_size(&self) -> u16 {
        unsafe { ffi::nw_shim_quic_get_max_udp_payload_size(self.handle) }
    }

    /// Set the maximum UDP payload size.
    pub fn set_max_udp_payload_size(&mut self, max_udp_payload_size: u16) -> &mut Self {
        unsafe { ffi::nw_shim_quic_set_max_udp_payload_size(self.handle, max_udp_payload_size) };
        self
    }

    /// Current idle timeout in milliseconds.
    #[must_use]
    pub fn idle_timeout(&self) -> u32 {
        unsafe { ffi::nw_shim_quic_get_idle_timeout(self.handle) }
    }

    /// Set the QUIC idle timeout in milliseconds.
    pub fn set_idle_timeout(&mut self, idle_timeout: u32) -> &mut Self {
        unsafe { ffi::nw_shim_quic_set_idle_timeout(self.handle, idle_timeout) };
        self
    }

    /// Copy the QUIC protocol definition.
    #[must_use]
    pub fn definition(&self) -> Option<ProtocolDefinition> {
        self.protocol_options().definition()
    }

    /// Borrow these QUIC options as a generic protocol-options wrapper.
    #[must_use]
    pub fn protocol_options(&self) -> ProtocolOptions {
        ProtocolOptions::clone_from_raw(self.handle)
    }
}

impl Clone for QuicOptions {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for QuicOptions {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Single-stream QUIC client. Uses `nw_quic_create_options` to add
/// QUIC framing on top of secure UDP.
pub struct QuicConnection {
    handle: *mut c_void,
    _keepalives: KeepAlives,
}

unsafe impl Send for QuicConnection {}
unsafe impl Sync for QuicConnection {}

impl QuicConnection {
    /// Open a QUIC connection to `host:port`.
    pub fn connect(host: &str, port: u16, alpn: &str) -> Result<Self, NetworkError> {
        let host_c = to_cstring(host, "host")?;
        let alpn_c = to_cstring(alpn, "alpn")?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_quic_connect(host_c.as_ptr(), port, alpn_c.as_ptr(), &mut status)
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            _keepalives: KeepAlives::empty(),
        })
    }

    /// Open a QUIC connection using explicit [`ConnectionParameters`].
    pub fn connect_with_parameters(
        host: &str,
        port: u16,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_connection_create_with_parameters(
                host.as_ptr(),
                port,
                parameters.as_ptr(),
                &mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            _keepalives: parameters.keepalives(),
        })
    }

    /// Send `data` on the single bidirectional stream.
    pub fn send(&self, data: &[u8]) -> Result<(), NetworkError> {
        let status = unsafe { ffi::nw_shim_tcp_send(self.handle, data.as_ptr(), data.len()) };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Send data with an explicit [`ContentContext`].
    pub fn send_with_context(
        &self,
        data: &[u8],
        context: &ContentContext,
    ) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_connection_send_with_context(
                self.handle,
                data.as_ptr(),
                data.len(),
                context.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Receive up to `max_len` bytes from the stream.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn receive(&self, max_len: usize) -> Result<Vec<u8>, NetworkError> {
        let mut buf = vec![0u8; max_len];
        let n = unsafe { ffi::nw_shim_tcp_receive(self.handle, buf.as_mut_ptr(), max_len) };
        if n < 0 {
            return Err(from_status(n as i32));
        }
        buf.truncate(n as usize);
        Ok(buf)
    }

    /// Receive data together with its [`ContentContext`].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn receive_with_context(&self, max_len: usize) -> Result<ReceivedContent, NetworkError> {
        let mut buf = vec![0_u8; max_len];
        let mut context = core::ptr::null_mut();
        let mut is_complete = 0;
        let n = unsafe {
            ffi::nw_shim_connection_receive_with_context(
                self.handle,
                buf.as_mut_ptr(),
                max_len,
                &mut context,
                &mut is_complete,
            )
        };
        if n < 0 {
            return Err(from_status(n as i32));
        }
        buf.truncate(n as usize);
        Ok(ReceivedContent {
            data: buf,
            context: (!context.is_null()).then_some(unsafe { ContentContext::from_raw(context) }),
            is_complete: is_complete != 0,
        })
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Drop for QuicConnection {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
