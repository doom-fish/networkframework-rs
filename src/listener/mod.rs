//! [`TcpListener`] — synchronous TCP listener via Network.framework.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::sync::{Arc, Mutex};

use crate::client::TcpClient;
use crate::connection_group::ConnectionGroup;
use crate::endpoint::Endpoint;
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::{ConnectionParameters, KeepAlives};
use doom_fish_utils::panic_safe::catch_user_panic;

type AdvertisedEndpointCallback = Mutex<Box<dyn FnMut(Option<Endpoint>, bool) + Send + 'static>>;

struct NewConnectionGroupCallback {
    keepalives: KeepAlives,
    callback: Mutex<Box<dyn FnMut(ConnectionGroup) + Send + 'static>>,
}

/// Blocking listener wrapper around `nw_listener`. Each accepted
/// connection returns a [`TcpClient`] handle that is already fully
/// ready for reads/writes.
pub struct TcpListener {
    handle: *mut c_void,
    keepalives: KeepAlives,
    advertised_endpoint_callback: Option<Arc<AdvertisedEndpointCallback>>,
    new_connection_group_callback: Option<Arc<NewConnectionGroupCallback>>,
}

unsafe impl Send for TcpListener {}
unsafe impl Sync for TcpListener {}

impl std::fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpListener")
            .field("handle", &self.handle)
            .field("has_advertised_endpoint_callback", &self.advertised_endpoint_callback.is_some())
            .field("has_new_connection_group_callback", &self.new_connection_group_callback.is_some())
            .finish_non_exhaustive()
    }
}

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
            advertised_endpoint_callback: None,
            new_connection_group_callback: None,
        })
    }

    /// Create a listener directly from parameters without binding a specific port first.
    pub fn bind_direct(parameters: &ConnectionParameters) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle =
            unsafe { ffi::nw_shim_listener_create_direct(parameters.as_ptr(), &mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            keepalives: parameters.keepalives(),
            advertised_endpoint_callback: None,
            new_connection_group_callback: None,
        })
    }

    /// Create a listener anchored to an existing connection.
    pub fn bind_with_connection(
        connection: &TcpClient,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_listener_create_with_connection(
                connection.as_ptr(),
                parameters.as_ptr(),
                &mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            keepalives: parameters.keepalives(),
            advertised_endpoint_callback: None,
            new_connection_group_callback: None,
        })
    }

    /// Create a launchd-backed listener from an existing launchd key.
    pub fn bind_with_launchd_key(
        parameters: &ConnectionParameters,
        launchd_key: &str,
    ) -> Result<Self, NetworkError> {
        let launchd_key = CString::new(launchd_key)
            .map_err(|e| NetworkError::InvalidArgument(format!("launchd_key NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_listener_create_with_launchd_key(
                parameters.as_ptr(),
                launchd_key.as_ptr(),
                &mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            keepalives: parameters.keepalives(),
            advertised_endpoint_callback: None,
            new_connection_group_callback: None,
        })
    }

    fn bind_inner(port: u16, use_tls: bool) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle =
            unsafe { ffi::nw_shim_listener_create(port, c_int::from(use_tls), &mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
            advertised_endpoint_callback: None,
            new_connection_group_callback: None,
        })
    }

    /// The port actually bound (useful when `bind(0)` was used).
    #[must_use]
    pub fn local_port(&self) -> u16 {
        unsafe { ffi::nw_shim_listener_port(self.handle) }
    }

    /// Current cap on the number of simultaneously delivered new connections.
    #[must_use]
    pub fn new_connection_limit(&self) -> u32 {
        unsafe { ffi::nw_shim_listener_get_new_connection_limit(self.handle) }
    }

    /// Update the cap on simultaneously delivered new connections.
    pub fn set_new_connection_limit(&mut self, new_connection_limit: u32) -> &mut Self {
        unsafe {
            ffi::nw_shim_listener_set_new_connection_limit(self.handle, new_connection_limit);
        };
        self
    }

    /// Receive callbacks when the listener's advertised endpoint changes.
    pub fn set_advertised_endpoint_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(Option<Endpoint>, bool) + Send + 'static,
    {
        let callback: Box<dyn FnMut(Option<Endpoint>, bool) + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_listener_set_advertised_endpoint_changed_handler(
                self.handle,
                Some(advertised_endpoint_trampoline),
                raw,
            );
        };
        self.advertised_endpoint_callback = Some(arc);
    }

    /// Receive callbacks when the listener creates connection groups.
    pub fn set_new_connection_group_handler<F>(&mut self, callback: F)
    where
        F: FnMut(ConnectionGroup) + Send + 'static,
    {
        let handler = Arc::new(NewConnectionGroupCallback {
            keepalives: self.keepalives.clone(),
            callback: Mutex::new(Box::new(callback)),
        });
        let raw = Arc::into_raw(handler.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_listener_set_new_connection_group_handler(
                self.handle,
                Some(new_connection_group_trampoline),
                raw,
            );
        };
        self.new_connection_group_callback = Some(handler);
    }

    #[cfg(feature = "async")]
    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[cfg(feature = "async")]
    #[must_use]
    pub(crate) fn keepalives(&self) -> KeepAlives {
        self.keepalives.clone()
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
        Ok(unsafe { TcpClient::from_raw_with_keepalives(conn_handle, self.keepalives.clone()) })
    }
}

unsafe extern "C" fn advertised_endpoint_trampoline(
    endpoint: *mut c_void,
    is_added: c_int,
    user_info: *mut c_void,
) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<AdvertisedEndpointCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    let endpoint = (!endpoint.is_null()).then_some(unsafe { Endpoint::from_raw(endpoint) });
    catch_user_panic("listener_advertised_endpoint_trampoline", || {
        guard(endpoint, is_added != 0);
    });
}

unsafe extern "C" fn new_connection_group_trampoline(group: *mut c_void, user_info: *mut c_void) {
    if user_info.is_null() || group.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<NewConnectionGroupCallback>() };
    let Ok(mut guard) = callback.callback.lock() else {
        return;
    };
    let group = unsafe { ConnectionGroup::from_raw(group, callback.keepalives.clone()) };
    catch_user_panic("listener_new_connection_group_trampoline", || {
        guard(group);
    });
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_listener_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
