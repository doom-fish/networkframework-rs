//! [`UdpClient`] — connected-mode UDP via Network.framework.

use core::ffi::{c_int, c_void};
use std::ffi::CString;

use crate::error::{from_status, NetworkError};
use crate::ffi;

/// Connected-mode UDP "client". Bound to a single peer `host:port`.
pub struct UdpClient {
    handle: *mut c_void,
}

unsafe impl Send for UdpClient {}
unsafe impl Sync for UdpClient {}

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
}

impl Drop for UdpClient {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
