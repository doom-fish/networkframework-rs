//! [`TcpClient`] — synchronous outbound TCP connection via Network.framework.

use core::ffi::{c_int, c_void};
use std::ffi::CString;

use crate::error::{from_status, NetworkError};
use crate::ffi;

/// Blocking client wrapper around `nw_connection`.
///
/// The connection is fully established (`nw_connection_state_ready`)
/// before [`connect`] returns.
pub struct TcpClient {
    handle: *mut c_void,
}

// Network.framework manages its own thread-safety; the shim wraps
// each call in a dispatch queue. The handle is never observed from
// Rust except as a pointer.
unsafe impl Send for TcpClient {}
unsafe impl Sync for TcpClient {}

impl TcpClient {
    /// Open a plain TCP connection to `host:port`. Blocks up to 30 s
    /// waiting for the connection to become ready.
    ///
    /// For TLS, use [`connect_tls`](Self::connect_tls).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] / [`NetworkError::Timeout`]
    /// on failure.
    pub fn connect(host: &str, port: u16) -> Result<Self, NetworkError> {
        Self::connect_inner(host, port, false)
    }

    /// Open a TLS-wrapped TCP connection to `host:port`. Server-name
    /// indication and Apple's default trust evaluation are used; the
    /// connection only becomes ready once the TLS handshake completes
    /// successfully.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] / [`NetworkError::Timeout`]
    /// on TCP or TLS failure (incl. invalid certificate / hostname).
    pub fn connect_tls(host: &str, port: u16) -> Result<Self, NetworkError> {
        Self::connect_inner(host, port, true)
    }

    fn connect_inner(host: &str, port: u16, use_tls: bool) -> Result<Self, NetworkError> {
        let host_c = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_tcp_connect(
                host_c.as_ptr(),
                port,
                c_int::from(use_tls),
                &mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self { handle })
    }

    /// Wrap a raw `nw_conn_handle*` (produced by the listener shim).
    ///
    /// # Safety
    ///
    /// `handle` must be a live pointer returned by the shim's accept
    /// path. Ownership is transferred to the returned [`TcpClient`].
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Send `data` over the connection. Blocks until the framework has
    /// acknowledged the buffer.
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

    /// Read up to `max_len` bytes from the connection. Blocks until at
    /// least one byte is available (or the connection ends).
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
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
