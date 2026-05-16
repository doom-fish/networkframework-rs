//! [`TcpListener`] — synchronous TCP listener via Network.framework.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};

use crate::client::TcpClient;
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::{ConnectionParameters, KeepAlives};

/// Blocking listener wrapper around `nw_listener`. Each accepted
/// connection returns a [`TcpClient`] handle that is already fully
/// ready for reads/writes.
pub struct TcpListener {
    handle: *mut c_void,
    keepalives: KeepAlives,
}

unsafe impl Send for TcpListener {}
unsafe impl Sync for TcpListener {}

impl TcpListener {
    /// Bind a plain TCP listener on `port` (use `0` for an OS-assigned
    /// port).
    ///
    /// For TLS, use [`bind_tls`](Self::bind_tls).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ListenFailed`] if the bind fails.
    pub fn bind(port: u16) -> Result<Self, NetworkError> {
        Self::bind_inner(port, false)
    }

    /// Bind a TLS-wrapped TCP listener on `port`. Uses Apple's default
    /// TLS configuration; the server must be configured with an
    /// identity (out of scope for this crate's `bind_tls` helper — for
    /// real-world use cases plug in `nw_protocol_options_set_identity`
    /// via your own shim).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ListenFailed`] if the bind fails.
    pub fn bind_tls(port: u16) -> Result<Self, NetworkError> {
        Self::bind_inner(port, true)
    }

    /// Bind a listener using explicit [`ConnectionParameters`].
    pub fn bind_with_parameters(
        port: u16,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_listener_create_with_parameters(parameters.as_ptr(), port, &mut status)
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            keepalives: parameters.keepalives(),
        })
    }

    fn bind_inner(port: u16, use_tls: bool) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_listener_create(port, c_int::from(use_tls), &mut status)
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
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
        Ok(unsafe {
            TcpClient::from_raw_with_keepalives(conn_handle, self.keepalives.clone())
        })
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
