//! [`TcpListener`] — synchronous TCP listener via Network.framework.

use core::ffi::{c_int, c_void};

use crate::client::TcpClient;
use crate::error::{from_status, NetworkError};
use crate::ffi;

/// Blocking listener wrapper around `nw_listener`. Each accepted
/// connection returns a [`TcpClient`] handle that is already fully
/// ready for reads/writes.
pub struct TcpListener {
    handle: *mut c_void,
}

unsafe impl Send for TcpListener {}
unsafe impl Sync for TcpListener {}

impl TcpListener {
    /// Bind a TCP listener on `port` (use `0` for an OS-assigned port).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ListenFailed`] if the bind fails.
    pub fn bind(port: u16) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle = unsafe { ffi::nw_shim_listener_create(port, &mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self { handle })
    }

    /// The port actually bound (useful when `bind(0)` was used).
    #[must_use]
    pub fn local_port(&self) -> u16 {
        unsafe { ffi::nw_shim_listener_port(self.handle) }
    }

    /// Block until a new connection arrives, then return it as a
    /// ready-to-use [`TcpClient`].
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] if the accepted
    /// connection couldn't reach the ready state.
    pub fn accept(&self) -> Result<TcpClient, NetworkError> {
        let mut status: c_int = 0;
        let conn_handle = unsafe { ffi::nw_shim_listener_accept(self.handle, &mut status) };
        if status != ffi::NW_OK || conn_handle.is_null() {
            return Err(from_status(status));
        }
        // SAFETY: nw_shim_listener_accept returns the same shape as
        // nw_shim_tcp_connect — a `nw_conn_handle*`. We hand it to
        // TcpClient via a private constructor below.
        Ok(unsafe { TcpClient::from_raw(conn_handle) })
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_listener_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
