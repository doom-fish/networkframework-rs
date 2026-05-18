//! [`UdpClient`] — connected-mode UDP via Network.framework.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;

use crate::client::{ContentContext, ReceivedContent};
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::{ConnectionParameters, KeepAlives};

/// Connected-mode UDP "client". Bound to a single peer `host:port`.
pub struct UdpClient {
    handle: *mut c_void,
    _keepalives: KeepAlives,
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
        let handle = unsafe { ffi::nw_shim_udp_connect(host_c.as_ptr(), port, &mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            _keepalives: KeepAlives::empty(),
        })
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

    /// Read up to `max_len` bytes from the next inbound datagram.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ReceiveFailed`].
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

    /// Receive a UDP datagram together with its [`ContentContext`].
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
}

impl Drop for UdpClient {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
