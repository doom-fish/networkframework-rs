//! [`UdpClient`] — connected-mode UDP via Network.framework.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;

use crate::client::{receive_content, ContentContext, ReceivedContent};
use crate::error::{from_status, receive_error, NetworkError};
use crate::ffi;
use crate::parameters::ConnectionParameters;

/// Connected-mode UDP "client". Bound to a single peer `host:port`.
pub struct UdpClient {
    handle: *mut c_void,
}

unsafe impl Send for UdpClient {}
unsafe impl Sync for UdpClient {}

impl std::fmt::Debug for UdpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpClient")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl UdpClient {
    /// Open a connected UDP socket to `host:port`.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] on failure.
    pub fn connect(host: &str, port: u16) -> Result<Self, NetworkError> {
        let host_c = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe { ffi::nw_shim_udp_connect(host_c.as_ptr(), port, &raw mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self { handle })
    }

    /// Open a UDP connection using explicit [`ConnectionParameters`].
    pub fn connect_with_parameters(
        host: &str,
        port: u16,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let host = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_connection_create_with_parameters(
                host.as_ptr(),
                port,
                parameters.as_ptr(),
                &raw mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self { handle })
    }

    /// Send `data` as a single UDP datagram.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send(&self, data: &[u8]) -> Result<(), NetworkError> {
        let status = unsafe { ffi::nw_shim_tcp_send(self.handle, data.as_ptr(), data.len()) };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Send a UDP datagram with an explicit [`ContentContext`].
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

    /// Receive the next inbound datagram, which must fit in `max_len` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::MessageTooLarge`] when the datagram is longer
    /// than `max_len` (the datagram is consumed; later calls receive the next
    /// one), or [`NetworkError::ReceiveFailed`].
    #[allow(clippy::cast_sign_loss)]
    pub fn receive(&self, max_len: usize) -> Result<Vec<u8>, NetworkError> {
        let mut buf = vec![0u8; max_len];
        let mut size = 0_usize;
        let n = unsafe {
            ffi::nw_shim_connection_receive_message(
                self.handle,
                buf.as_mut_ptr(),
                max_len,
                &raw mut size,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };
        if n < 0 {
            return Err(receive_error(n, size, max_len));
        }
        buf.truncate(n as usize);
        Ok(buf)
    }

    /// Receive a UDP datagram together with its [`ContentContext`].
    pub fn receive_with_context(&self, max_len: usize) -> Result<ReceivedContent, NetworkError> {
        receive_content(self.handle, max_len, true)
    }
}

impl Drop for UdpClient {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
