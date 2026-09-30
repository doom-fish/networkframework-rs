//! [`ListenerBuilder`] — pre-start configuration for [`TcpListener`].

use core::ffi::c_int;
use std::ffi::CString;
use std::sync::Mutex;

use doom_fish_utils::callback_context::CallbackContext;

use super::{
    advertised_endpoint_trampoline, new_connection_group_trampoline, AdvertisedEndpointCallback,
    NewConnectionGroupCallback, TcpListener,
};
use crate::advertise_descriptor::AdvertiseDescriptor;
use crate::client::TcpClient;
use crate::connection_group::ConnectionGroup;
use crate::context::Subscription;
use crate::endpoint::Endpoint;
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::ConnectionParameters;

enum Source<'a> {
    Parameters,
    Port(u16),
    Connection(&'a TcpClient),
    LaunchdKey(String),
}

/// Configures a [`TcpListener`] before it starts.
///
/// Created by [`TcpListener::builder`]. Every setting is applied before
/// Network.framework starts the listener, so none of them can race the first
/// connection or the first advertisement.
///
/// ```no_run
/// use networkframework::{AdvertiseDescriptor, ConnectionParameters, TcpListener};
///
/// let mut parameters = ConnectionParameters::tcp()?;
/// parameters.set_include_peer_to_peer(true);
/// let descriptor = AdvertiseDescriptor::bonjour_service(None, "_awdlssh._tcp", None)?;
///
/// let listener = TcpListener::builder(&parameters)
///     .advertise(descriptor)
///     .on_advertised_endpoint(|endpoint, added| {
///         println!("advertised={added} {:?}", endpoint.and_then(|e| e.bonjour_service_name()));
///     })
///     .bind()?;
/// let client = listener.accept()?;
/// # Ok::<(), networkframework::NetworkError>(())
/// ```
#[must_use = "a ListenerBuilder does nothing until `bind` is called"]
pub struct ListenerBuilder<'a> {
    parameters: &'a ConnectionParameters,
    source: Source<'a>,
    advertise: Option<AdvertiseDescriptor>,
    advertised_endpoint: Option<AdvertisedEndpointCallback>,
    new_connection_group: Option<NewConnectionGroupCallback>,
    new_connection_limit: Option<u32>,
}

impl std::fmt::Debug for ListenerBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = match &self.source {
            Source::Parameters => "parameters".to_owned(),
            Source::Port(port) => format!("port {port}"),
            Source::Connection(_) => "connection".to_owned(),
            Source::LaunchdKey(key) => format!("launchd key {key:?}"),
        };
        f.debug_struct("ListenerBuilder")
            .field("source", &source)
            .field("advertise", &self.advertise)
            .field("advertised_endpoint", &self.advertised_endpoint.is_some())
            .field("new_connection_group", &self.new_connection_group.is_some())
            .field("new_connection_limit", &self.new_connection_limit)
            .finish_non_exhaustive()
    }
}

impl<'a> ListenerBuilder<'a> {
    pub(super) const fn new(parameters: &'a ConnectionParameters) -> Self {
        Self {
            parameters,
            source: Source::Parameters,
            advertise: None,
            advertised_endpoint: None,
            new_connection_group: None,
            new_connection_limit: None,
        }
    }

    /// Listen on `port` (`0` lets the system choose). Without this, the port
    /// comes from the parameters' local endpoint, or the system chooses one.
    ///
    /// `port`, [`connection`](Self::connection) and
    /// [`launchd_key`](Self::launchd_key) each pick where the listener comes
    /// from; the last one called wins.
    pub fn port(mut self, port: u16) -> Self {
        self.source = Source::Port(port);
        self
    }

    /// Anchor the listener to an existing connection
    /// (`nw_listener_create_with_connection`).
    pub fn connection(mut self, connection: &'a TcpClient) -> Self {
        self.source = Source::Connection(connection);
        self
    }

    /// Take over the socket launchd registered under `launchd_key`
    /// (`nw_listener_create_with_launchd_key`).
    pub fn launchd_key(mut self, launchd_key: impl Into<String>) -> Self {
        self.source = Source::LaunchdKey(launchd_key.into());
        self
    }

    /// Advertise the listener as a Bonjour or application service. With
    /// [`ConnectionParameters::set_include_peer_to_peer`] enabled, the service
    /// is also advertised over AWDL.
    pub fn advertise(mut self, descriptor: AdvertiseDescriptor) -> Self {
        self.advertise = Some(descriptor);
        self
    }

    /// Receive the endpoint the system registered for the advertisement
    /// (`added == true`) and its removal (`added == false`). Only fires for a
    /// listener that advertises.
    pub fn on_advertised_endpoint<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Option<Endpoint>, bool) + Send + 'static,
    {
        self.advertised_endpoint = Some(Mutex::new(Box::new(callback)));
        self
    }

    /// Deliver inbound connection groups (for example QUIC connections) to
    /// `handler` instead of single connections. [`TcpListener::accept`] then
    /// returns an error.
    pub fn on_new_connection_group<F>(mut self, handler: F) -> Self
    where
        F: FnMut(ConnectionGroup) + Send + 'static,
    {
        self.new_connection_group = Some(Mutex::new(Box::new(handler)));
        self
    }

    /// Cap the number of connections delivered before the limit is raised
    /// again with [`TcpListener::set_new_connection_limit`].
    pub const fn new_connection_limit(mut self, new_connection_limit: u32) -> Self {
        self.new_connection_limit = Some(new_connection_limit);
        self
    }

    /// Create the listener, apply every setting, start it and wait until it
    /// is ready.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidArgument`] if the launchd key contains
    /// a NUL byte, and [`NetworkError::ListenFailed`] if the listener does not
    /// become ready.
    pub fn bind(self) -> Result<TcpListener, NetworkError> {
        let launchd_key = match &self.source {
            Source::LaunchdKey(key) => Some(CString::new(key.as_str()).map_err(|e| {
                NetworkError::InvalidArgument(format!("launchd_key NUL byte: {e}"))
            })?),
            _ => None,
        };

        let parameters = self.parameters.as_ptr();
        let mut status: c_int = 0;
        // SAFETY: every pointer outlives the call; the shim retains what it keeps.
        let handle = unsafe {
            match (&self.source, &launchd_key) {
                (Source::Port(port), _) => {
                    ffi::nw_shim_listener_prepare_with_port(parameters, *port, &raw mut status)
                }
                (Source::Connection(connection), _) => {
                    ffi::nw_shim_listener_prepare_with_connection(
                        connection.as_ptr(),
                        parameters,
                        &raw mut status,
                    )
                }
                (Source::LaunchdKey(_), Some(key)) => {
                    ffi::nw_shim_listener_prepare_with_launchd_key(
                        parameters,
                        key.as_ptr(),
                        &raw mut status,
                    )
                }
                _ => ffi::nw_shim_listener_prepare_direct(parameters, &raw mut status),
            }
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }

        // From here on nothing may return early: a prepared handle is only
        // released by `nw_shim_listener_start_prepared`.
        if let Some(limit) = self.new_connection_limit {
            unsafe { ffi::nw_shim_listener_set_new_connection_limit(handle, limit) };
        }
        if let Some(descriptor) = &self.advertise {
            // The listener retains the descriptor.
            unsafe { ffi::nw_shim_listener_set_advertise_descriptor(handle, descriptor.as_ptr()) };
        }
        let advertised_endpoint = self.advertised_endpoint.and_then(|callback| {
            Subscription::register(callback, |context, retain, release| unsafe {
                ffi::nw_shim_listener_subscribe_advertised_endpoint(
                    handle,
                    Some(advertised_endpoint_trampoline),
                    context,
                    Some(retain),
                    Some(release),
                )
            })
        });
        let new_connection_group = self.new_connection_group.map(|handler| {
            let context: CallbackContext<NewConnectionGroupCallback> =
                CallbackContext::new(handler);
            // Cannot fail: the handle is fresh and has no group handler yet.
            unsafe {
                ffi::nw_shim_listener_set_new_connection_group_handler(
                    handle,
                    Some(new_connection_group_trampoline),
                    context.retained_ptr(),
                    Some(CallbackContext::<NewConnectionGroupCallback>::RETAIN),
                    Some(CallbackContext::<NewConnectionGroupCallback>::RELEASE),
                )
            };
            context
        });

        let status = unsafe { ffi::nw_shim_listener_start_prepared(handle) };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(TcpListener {
            handle,
            advertised_endpoint,
            new_connection_group,
        })
    }
}
