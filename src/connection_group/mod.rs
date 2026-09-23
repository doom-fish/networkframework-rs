//! Connection groups built on `nw_connection_group_*`.

#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::sync::Mutex;

use doom_fish_utils::callback_context::CallbackContext;

use crate::client::{ContentContext, TcpClient};
use crate::context::Subscription;
use crate::endpoint::Endpoint;
use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::ConnectionParameters;
use crate::path::Path;
use crate::protocol::{ProtocolDefinition, ProtocolMetadata, ProtocolOptions};

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

/// A group descriptor for multicast or multiplex connection groups.
pub struct ConnectionGroupDescriptor {
    handle: *mut c_void,
}

unsafe impl Send for ConnectionGroupDescriptor {}

impl std::fmt::Debug for ConnectionGroupDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionGroupDescriptor")
            .field("handle", &self.handle)
            .finish()
    }
}

impl ConnectionGroupDescriptor {
    /// Create a multiplex group descriptor for a remote endpoint.
    pub fn multiplex(host: &str, port: u16) -> Result<Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let handle = unsafe { ffi::nw_shim_group_descriptor_create_multiplex(host.as_ptr(), port) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create multiplex group descriptor".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create a multicast group descriptor from an IP multicast address.
    pub fn multicast(group_address: &str, port: u16) -> Result<Self, NetworkError> {
        let group_address = to_cstring(group_address, "group_address")?;
        let handle =
            unsafe { ffi::nw_shim_group_descriptor_create_multicast(group_address.as_ptr(), port) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create multicast group descriptor".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Add another endpoint to the descriptor.
    pub fn add_endpoint(&mut self, host: &str, port: u16) -> Result<&mut Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let added =
            unsafe { ffi::nw_shim_group_descriptor_add_endpoint(self.handle, host.as_ptr(), port) };
        if added == 0 {
            return Err(NetworkError::InvalidArgument(
                "failed to add endpoint to group descriptor".into(),
            ));
        }
        Ok(self)
    }

    /// Enumerate the endpoints described by this connection group.
    #[must_use]
    pub fn endpoints(&self) -> Vec<Endpoint> {
        unsafe extern "C" fn collect(endpoint: *mut c_void, user_info: *mut c_void) -> c_int {
            if user_info.is_null() || endpoint.is_null() {
                return 0;
            }
            let endpoints = unsafe { &mut *user_info.cast::<Vec<Endpoint>>() };
            endpoints.push(unsafe { Endpoint::from_raw(endpoint) });
            1
        }

        let mut endpoints = Vec::new();
        unsafe {
            ffi::nw_shim_group_descriptor_enumerate_endpoints(
                self.handle,
                Some(collect),
                std::ptr::addr_of_mut!(endpoints).cast(),
            )
        };
        endpoints
    }

    /// Restrict multicast traffic to a specific source endpoint.
    pub fn set_specific_source(&mut self, endpoint: &Endpoint) -> &mut Self {
        unsafe {
            ffi::nw_shim_multicast_group_descriptor_set_specific_source(
                self.handle,
                endpoint.as_ptr(),
            )
        };
        self
    }

    /// Whether unicast traffic is disabled for multicast descriptors.
    #[must_use]
    pub fn disable_unicast_traffic(&self) -> bool {
        unsafe {
            ffi::nw_shim_multicast_group_descriptor_get_disable_unicast_traffic(self.handle) != 0
        }
    }

    /// Enable or disable unicast traffic for multicast descriptors.
    pub fn set_disable_unicast_traffic(&mut self, disable_unicast_traffic: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_multicast_group_descriptor_set_disable_unicast_traffic(
                self.handle,
                c_int::from(disable_unicast_traffic),
            )
        };
        self
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Clone for ConnectionGroupDescriptor {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ConnectionGroupDescriptor {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Connection group lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionGroupState {
    Invalid,
    Waiting,
    Ready,
    Failed,
    Cancelled,
}

impl ConnectionGroupState {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Waiting,
            2 => Self::Ready,
            3 => Self::Failed,
            4 => Self::Cancelled,
            _ => Self::Invalid,
        }
    }
}

/// An inbound connection-group message.
#[derive(Debug, Clone)]
pub struct ConnectionGroupMessage {
    pub data: Vec<u8>,
    pub context: Option<ContentContext>,
    pub is_complete: bool,
}

type StateCallback = Mutex<Box<dyn FnMut(ConnectionGroupState) + Send + 'static>>;
type ReceiveCallback = Mutex<Box<dyn FnMut(ConnectionGroupMessage) + Send + 'static>>;
type NewConnectionCallback = Mutex<Box<dyn FnMut(TcpClient) + Send + 'static>>;

/// A running connection group.
pub struct ConnectionGroup {
    handle: *mut c_void,
    state: Option<Subscription<StateCallback>>,
    receive: Option<Subscription<ReceiveCallback>>,
    new_connection: Option<Subscription<NewConnectionCallback>>,
}

unsafe impl Send for ConnectionGroup {}
unsafe impl Sync for ConnectionGroup {}

impl std::fmt::Debug for ConnectionGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionGroup")
            .field("handle", &self.handle)
            .field("state", &self.state)
            .field("receive", &self.receive)
            .field("new_connection", &self.new_connection)
            .finish_non_exhaustive()
    }
}

impl ConnectionGroup {
    /// Create a connection group from a descriptor and parameters.
    pub fn new(
        descriptor: &ConnectionGroupDescriptor,
        parameters: &crate::ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let handle = unsafe {
            ffi::nw_shim_connection_group_create(descriptor.as_ptr(), parameters.as_ptr())
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create connection group".into(),
            ));
        }
        Ok(unsafe { Self::from_raw(handle) })
    }

    fn unsubscribe<T: Send + Sync + 'static>(&self, subscription: Option<Subscription<T>>) {
        if let Some(subscription) = subscription {
            subscription.deactivate();
            unsafe { ffi::nw_shim_connection_group_unsubscribe(self.handle, subscription.token) };
        }
    }

    /// Set a state-change callback. Call before [`start`](Self::start).
    pub fn set_state_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(ConnectionGroupState) + Send + 'static,
    {
        let previous = self.state.take();
        self.unsubscribe(previous);
        let callback: Box<dyn FnMut(ConnectionGroupState) + Send + 'static> = Box::new(callback);
        let handle = self.handle;
        self.state =
            Subscription::register(Mutex::new(callback), |context, retain, release| unsafe {
                ffi::nw_shim_connection_group_subscribe_state(
                    handle,
                    Some(state_trampoline),
                    context,
                    Some(retain),
                    Some(release),
                )
            });
    }

    /// Set the receive callback. Call before [`start`](Self::start).
    pub fn set_receive_handler<F>(
        &mut self,
        maximum_message_size: u32,
        reject_oversized_messages: bool,
        callback: F,
    ) -> Result<(), NetworkError>
    where
        F: FnMut(ConnectionGroupMessage) + Send + 'static,
    {
        let callback: Box<dyn FnMut(ConnectionGroupMessage) + Send + 'static> = Box::new(callback);
        let handle = self.handle;
        let subscription =
            Subscription::register(Mutex::new(callback), |context, retain, release| unsafe {
                ffi::nw_shim_connection_group_subscribe_receive(
                    handle,
                    maximum_message_size,
                    c_int::from(reject_oversized_messages),
                    Some(receive_trampoline),
                    context,
                    Some(retain),
                    Some(release),
                )
            })
            .ok_or_else(|| {
                NetworkError::InvalidArgument(
                    "the receive handler must be set before the group starts".into(),
                )
            })?;
        let previous = self.receive.replace(subscription);
        self.unsubscribe(previous);
        Ok(())
    }

    /// # Safety
    ///
    /// `handle` must be a valid retained connection-group handle owned by the
    /// caller and remain alive for the returned wrapper.
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self {
            handle,
            state: None,
            receive: None,
            new_connection: None,
        }
    }

    /// Start the connection group and wait for the initial state update.
    pub fn start(&self) -> Result<(), NetworkError> {
        let status = unsafe { ffi::nw_shim_connection_group_start(self.handle) };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Send a message using the group's default destination semantics.
    pub fn send(&self, data: &[u8], context: &ContentContext) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_connection_group_send(
                self.handle,
                data.as_ptr(),
                data.len(),
                core::ptr::null(),
                0,
                context.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Send a message to a specific endpoint.
    pub fn send_to(
        &self,
        host: &str,
        port: u16,
        data: &[u8],
        context: &ContentContext,
    ) -> Result<(), NetworkError> {
        let host = to_cstring(host, "host")?;
        let status = unsafe {
            ffi::nw_shim_connection_group_send(
                self.handle,
                data.as_ptr(),
                data.len(),
                host.as_ptr(),
                port,
                context.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Copy the underlying group descriptor.
    #[must_use]
    pub fn descriptor(&self) -> Option<ConnectionGroupDescriptor> {
        let handle = unsafe { ffi::nw_shim_connection_group_copy_descriptor(self.handle) };
        (!handle.is_null()).then_some(ConnectionGroupDescriptor { handle })
    }

    /// Copy the group's parameters snapshot.
    #[must_use]
    pub fn parameters(&self) -> Option<ConnectionParameters> {
        let handle = unsafe { ffi::nw_shim_connection_group_copy_parameters(self.handle) };
        (!handle.is_null()).then_some(unsafe { ConnectionParameters::from_raw(handle) })
    }

    /// Copy the remote endpoint associated with a received message.
    #[must_use]
    pub fn remote_endpoint_for_message(&self, context: &ContentContext) -> Option<Endpoint> {
        let handle = unsafe {
            ffi::nw_shim_connection_group_copy_remote_endpoint_for_message(
                self.handle,
                context.as_ptr(),
            )
        };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Copy the local endpoint associated with a received message.
    #[must_use]
    pub fn local_endpoint_for_message(&self, context: &ContentContext) -> Option<Endpoint> {
        let handle = unsafe {
            ffi::nw_shim_connection_group_copy_local_endpoint_for_message(
                self.handle,
                context.as_ptr(),
            )
        };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Copy the path associated with a received message.
    #[must_use]
    pub fn path_for_message(&self, context: &ContentContext) -> Option<Path> {
        let handle = unsafe {
            ffi::nw_shim_connection_group_copy_path_for_message(self.handle, context.as_ptr())
        };
        (!handle.is_null()).then_some(unsafe { Path::from_raw(handle) })
    }

    /// Copy group-wide protocol metadata for a specific protocol definition.
    #[must_use]
    pub fn protocol_metadata(&self, definition: &ProtocolDefinition) -> Option<ProtocolMetadata> {
        let handle = unsafe {
            ffi::nw_shim_connection_group_copy_protocol_metadata(self.handle, definition.as_ptr())
        };
        (!handle.is_null()).then_some(unsafe { ProtocolMetadata::from_raw(handle) })
    }

    /// Copy per-message protocol metadata for a specific protocol definition.
    #[must_use]
    pub fn protocol_metadata_for_message(
        &self,
        context: &ContentContext,
        definition: &ProtocolDefinition,
    ) -> Option<ProtocolMetadata> {
        let handle = unsafe {
            ffi::nw_shim_connection_group_copy_protocol_metadata_for_message(
                self.handle,
                context.as_ptr(),
                definition.as_ptr(),
            )
        };
        (!handle.is_null()).then_some(unsafe { ProtocolMetadata::from_raw(handle) })
    }

    /// Extract a new connection corresponding to a received message.
    pub fn extract_connection_for_message(
        &self,
        context: &ContentContext,
    ) -> Result<TcpClient, NetworkError> {
        let mut status = ffi::NW_OK;
        let handle = unsafe {
            ffi::nw_shim_connection_group_extract_connection_for_message(
                self.handle,
                context.as_ptr(),
                &raw mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(unsafe { TcpClient::from_raw(handle) })
    }

    /// Extract a connection for a specific remote endpoint and protocol options.
    pub fn extract_connection(
        &self,
        endpoint: &Endpoint,
        protocol_options: &ProtocolOptions,
    ) -> Result<TcpClient, NetworkError> {
        let mut status = ffi::NW_OK;
        let handle = unsafe {
            ffi::nw_shim_connection_group_extract_connection(
                self.handle,
                endpoint.as_ptr(),
                protocol_options.as_ptr(),
                &raw mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(unsafe { TcpClient::from_raw(handle) })
    }

    /// Receive callbacks for new connections accepted by the group.
    pub fn set_new_connection_handler<F>(&mut self, callback: F) -> Result<(), NetworkError>
    where
        F: FnMut(TcpClient) + Send + 'static,
    {
        let callback: Box<dyn FnMut(TcpClient) + Send + 'static> = Box::new(callback);
        let handle = self.handle;
        let subscription =
            Subscription::register(Mutex::new(callback), |context, retain, release| unsafe {
                ffi::nw_shim_connection_group_subscribe_new_connection(
                    handle,
                    Some(new_connection_trampoline),
                    context,
                    Some(retain),
                    Some(release),
                )
            })
            .ok_or_else(|| {
                NetworkError::InvalidArgument(
                    "the new-connection handler must be set before the group starts".into(),
                )
            })?;
        let previous = self.new_connection.replace(subscription);
        self.unsubscribe(previous);
        Ok(())
    }

    /// Reinsert an extracted connection back into the group.
    pub fn reinsert_extracted_connection(&self, connection: TcpClient) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_connection_group_reinsert_extracted_connection(
                self.handle,
                connection.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        unsafe { ffi::nw_shim_connection_release_without_cancel(connection.into_raw()) };
        Ok(())
    }

    /// Reply to an inbound message using the group's reply path.
    pub fn reply(
        &self,
        inbound_message: &ContentContext,
        outbound_message: Option<&ContentContext>,
        data: &[u8],
    ) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_connection_group_reply(
                self.handle,
                inbound_message.as_ptr(),
                outbound_message.map_or(core::ptr::null_mut(), ContentContext::as_ptr),
                data.as_ptr(),
                data.len(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Cancel the connection group.
    pub fn cancel(&self) {
        unsafe { ffi::nw_shim_connection_group_cancel(self.handle) };
    }
}

impl Drop for ConnectionGroup {
    fn drop(&mut self) {
        self.state = None;
        self.receive = None;
        self.new_connection = None;
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_connection_group_release(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

unsafe extern "C" fn state_trampoline(state: c_int, context: *mut c_void) {
    let state = ConnectionGroupState::from_raw(state);
    unsafe {
        CallbackContext::<StateCallback>::with(
            context,
            "connection_group_state_trampoline",
            |callback| {
                if let Ok(mut callback) = callback.lock() {
                    callback(state);
                }
            },
        )
    };
}

unsafe extern "C" fn new_connection_trampoline(connection: *mut c_void, context: *mut c_void) {
    if connection.is_null() {
        return;
    }
    let client = unsafe { TcpClient::from_raw(connection) };
    unsafe {
        CallbackContext::<NewConnectionCallback>::with(
            context,
            "connection_group_new_connection_trampoline",
            move |callback| {
                if let Ok(mut callback) = callback.lock() {
                    callback(client);
                }
            },
        )
    };
}

unsafe extern "C" fn receive_trampoline(
    data: *const u8,
    len: usize,
    message_context: *mut c_void,
    is_complete: c_int,
    context: *mut c_void,
) {
    let content_context =
        (!message_context.is_null()).then(|| unsafe { ContentContext::from_raw(message_context) });
    let bytes = if data.is_null() || len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
    };
    let message = ConnectionGroupMessage {
        data: bytes,
        context: content_context,
        is_complete: is_complete != 0,
    };
    unsafe {
        CallbackContext::<ReceiveCallback>::with(
            context,
            "connection_group_receive_trampoline",
            move |callback| {
                if let Ok(mut callback) = callback.lock() {
                    callback(message);
                }
            },
        )
    };
}
