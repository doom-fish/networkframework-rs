//! [`TcpListener`] — synchronous TCP listener via Network.framework.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::sync::Mutex;

use doom_fish_utils::callback_context::CallbackContext;

use crate::client::TcpClient;
use crate::connection_group::ConnectionGroup;
use crate::context::Subscription;
use crate::endpoint::Endpoint;
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::interface::InterfaceType;
use crate::parameters::ConnectionParameters;
use crate::tls::{TlsIdentity, TlsVersion};

type AdvertisedEndpointCallback = Mutex<Box<dyn FnMut(Option<Endpoint>, bool) + Send + 'static>>;
type NewConnectionGroupCallback = Mutex<Box<dyn FnMut(ConnectionGroup) + Send + 'static>>;

/// Blocking listener wrapper around `nw_listener`. Each accepted
/// connection returns a [`TcpClient`] handle that is already fully
/// ready for reads/writes.
pub struct TcpListener {
    handle: *mut c_void,
    advertised_endpoint: Option<Subscription<AdvertisedEndpointCallback>>,
    new_connection_group: Option<CallbackContext<NewConnectionGroupCallback>>,
}

unsafe impl Send for TcpListener {}
unsafe impl Sync for TcpListener {}

impl std::fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpListener")
            .field("handle", &self.handle)
            .field("advertised_endpoint", &self.advertised_endpoint)
            .field("new_connection_group", &self.new_connection_group)
            .finish_non_exhaustive()
    }
}

impl TcpListener {
    const fn from_handle(handle: *mut c_void) -> Self {
        Self {
            handle,
            advertised_endpoint: None,
            new_connection_group: None,
        }
    }

    /// Bind a plain TCP listener on `port` (use `0` for an OS-assigned
    /// port) on every local interface, reachable from the network.
    ///
    /// For TLS, use [`bind_tls`](Self::bind_tls); to accept only local
    /// connections, use [`bind_loopback`](Self::bind_loopback).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ListenFailed`] if the bind fails.
    pub fn bind(port: u16) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle = unsafe { ffi::nw_shim_listener_create(port, 0, &raw mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self::from_handle(handle))
    }

    pub fn bind_loopback(port: u16) -> Result<Self, NetworkError> {
        let mut parameters = ConnectionParameters::tcp()?;
        parameters
            .set_required_interface_type(InterfaceType::Loopback)
            .set_local_endpoint(Some(&Endpoint::address("127.0.0.1", 0)?));
        Self::bind_with_parameters(port, &parameters)
    }

    /// Bind a TLS listener on `port` on every local interface. It presents
    /// `identity` and requires TLS 1.2 or newer.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ListenFailed`] if the bind fails.
    pub fn bind_tls(port: u16, identity: &TlsIdentity) -> Result<Self, NetworkError> {
        let parameters = ConnectionParameters::tls_tcp_configured(|tls| {
            tls.set_local_identity(identity)
                .set_min_tls_version(TlsVersion::Tls12);
        })?;
        Self::bind_with_parameters(port, &parameters)
    }

    /// Bind a listener using explicit [`ConnectionParameters`].
    pub fn bind_with_parameters(
        port: u16,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_listener_create_with_parameters(parameters.as_ptr(), port, &raw mut status)
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self::from_handle(handle))
    }

    pub fn bind_with_group_handler<F>(
        port: u16,
        parameters: &ConnectionParameters,
        handler: F,
    ) -> Result<Self, NetworkError>
    where
        F: FnMut(ConnectionGroup) + Send + 'static,
    {
        let handler: Box<dyn FnMut(ConnectionGroup) + Send + 'static> = Box::new(handler);
        let context: CallbackContext<NewConnectionGroupCallback> =
            CallbackContext::new(Mutex::new(handler));
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_listener_create_for_groups(
                parameters.as_ptr(),
                port,
                Some(new_connection_group_trampoline),
                context.retained_ptr(),
                Some(CallbackContext::<NewConnectionGroupCallback>::RETAIN),
                Some(CallbackContext::<NewConnectionGroupCallback>::RELEASE),
                &raw mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            advertised_endpoint: None,
            new_connection_group: Some(context),
        })
    }

    /// Create a listener directly from parameters without binding a specific port first.
    pub fn bind_direct(parameters: &ConnectionParameters) -> Result<Self, NetworkError> {
        let mut status: c_int = 0;
        let handle =
            unsafe { ffi::nw_shim_listener_create_direct(parameters.as_ptr(), &raw mut status) };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self::from_handle(handle))
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
                &raw mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self::from_handle(handle))
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
                &raw mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self::from_handle(handle))
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
        if let Some(previous) = self.advertised_endpoint.take() {
            previous.deactivate();
            unsafe { ffi::nw_shim_listener_unsubscribe(self.handle, previous.token) };
        }
        let callback: Box<dyn FnMut(Option<Endpoint>, bool) + Send + 'static> = Box::new(callback);
        let handle = self.handle;
        self.advertised_endpoint =
            Subscription::register(Mutex::new(callback), |context, retain, release| unsafe {
                ffi::nw_shim_listener_subscribe_advertised_endpoint(
                    handle,
                    Some(advertised_endpoint_trampoline),
                    context,
                    Some(retain),
                    Some(release),
                )
            });
    }

    #[cfg(feature = "async")]
    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    /// Block until a new connection is ready, then return it as a
    /// ready-to-use [`TcpClient`].
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::Cancelled`] once the listener has failed or
    /// been cancelled and no ready connection is left.
    pub fn accept(&self) -> Result<TcpClient, NetworkError> {
        if self.new_connection_group.is_some() {
            return Err(NetworkError::InvalidArgument(
                "a listener bound with a group handler delivers connection groups".into(),
            ));
        }
        let mut status: c_int = 0;
        let conn_handle = unsafe { ffi::nw_shim_listener_accept(self.handle, &raw mut status) };
        if status != ffi::NW_OK || conn_handle.is_null() {
            return Err(from_status(status));
        }
        // SAFETY: nw_shim_listener_accept returns the same shape as
        // nw_shim_tcp_connect — a `nw_conn_handle*`. We hand it to
        // TcpClient via a private constructor below.
        Ok(unsafe { TcpClient::from_raw(conn_handle) })
    }
}

unsafe extern "C" fn advertised_endpoint_trampoline(
    endpoint: *mut c_void,
    is_added: c_int,
    context: *mut c_void,
) {
    let endpoint = (!endpoint.is_null()).then(|| unsafe { Endpoint::from_raw(endpoint) });
    unsafe {
        CallbackContext::<AdvertisedEndpointCallback>::with(
            context,
            "listener_advertised_endpoint_trampoline",
            move |callback| {
                if let Ok(mut callback) = callback.lock() {
                    callback(endpoint, is_added != 0);
                }
            },
        )
    };
}

unsafe extern "C" fn new_connection_group_trampoline(group: *mut c_void, context: *mut c_void) {
    if group.is_null() {
        return;
    }
    let group = unsafe { ConnectionGroup::from_raw(group) };
    unsafe {
        CallbackContext::<NewConnectionGroupCallback>::with(
            context,
            "listener_new_connection_group_trampoline",
            move |callback| {
                if let Ok(mut callback) = callback.lock() {
                    callback(group);
                }
            },
        )
    };
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        self.advertised_endpoint = None;
        self.new_connection_group = None;
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_listener_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
