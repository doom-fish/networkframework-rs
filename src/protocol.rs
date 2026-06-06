//! Protocol definitions and options.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::sync::{Arc, Mutex};

use doom_fish_utils::panic_safe::catch_user_panic;

use crate::error::{FrameworkError, NetworkError};
use crate::ffi;
use crate::parameters_support::ServiceClass;
use crate::quic_support::{SecurityProtocolMetadata, SecurityProtocolOptions};
use crate::websocket::{Opcode, WsCloseCode, WsRequest, WsResponse, WsVersion};

fn handle_result(handle: *mut c_void, context: &str) -> Result<*mut c_void, NetworkError> {
    if handle.is_null() {
        return Err(NetworkError::InvalidArgument(format!(
            "failed to create {context}"
        )));
    }
    Ok(handle)
}

#[derive(Debug)]
pub struct ProtocolDefinition {
    handle: *mut c_void,
}

unsafe impl Send for ProtocolDefinition {}
unsafe impl Sync for ProtocolDefinition {}

impl ProtocolDefinition {
    pub fn tcp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_copy_tcp_definition() },
                "TCP definition",
            )?,
        })
    }

    pub fn udp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_copy_udp_definition() },
                "UDP definition",
            )?,
        })
    }

    pub fn tls() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_copy_tls_definition() },
                "TLS definition",
            )?,
        })
    }

    pub fn ip() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_copy_ip_definition() },
                "IP definition",
            )?,
        })
    }

    pub fn websocket() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_copy_ws_definition() },
                "WebSocket definition",
            )?,
        })
    }

    pub fn quic() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_copy_quic_definition() },
                "QUIC definition",
            )?,
        })
    }

    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Clone for ProtocolDefinition {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl PartialEq for ProtocolDefinition {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::nw_shim_protocol_definition_is_equal(self.handle, other.handle) != 0 }
    }
}

impl Eq for ProtocolDefinition {}

impl Drop for ProtocolDefinition {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

type WsClientRequestHandlerCallback =
    Mutex<Box<dyn FnMut(WsRequest) -> Option<WsResponse> + Send + 'static>>;
type WsPongHandlerCallback =
    Mutex<Box<dyn FnMut(Option<FrameworkError>) + Send + Sync + 'static>>;

pub struct ProtocolOptions {
    handle: *mut c_void,
    ws_client_request_callback: Option<Arc<WsClientRequestHandlerCallback>>,
}

unsafe impl Send for ProtocolOptions {}
unsafe impl Sync for ProtocolOptions {}

impl std::fmt::Debug for ProtocolOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtocolOptions")
            .field("handle", &self.handle)
            .field("has_ws_client_request_callback", &self.ws_client_request_callback.is_some())
            .finish_non_exhaustive()
    }
}

impl ProtocolOptions {
    pub fn tcp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_tcp_options() },
                "TCP options",
            )?,
            ws_client_request_callback: None,
        })
    }

    pub fn udp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_udp_options() },
                "UDP options",
            )?,
            ws_client_request_callback: None,
        })
    }

    pub fn tls() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_tls_options() },
                "TLS options",
            )?,
            ws_client_request_callback: None,
        })
    }

    pub fn ip() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ip_options() },
                "IP options",
            )?,
            ws_client_request_callback: None,
        })
    }

    pub fn websocket() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ws_options() },
                "WebSocket options",
            )?,
            ws_client_request_callback: None,
        })
    }

    pub fn quic() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_quic_options() },
                "QUIC options",
            )?,
            ws_client_request_callback: None,
        })
    }

    #[must_use]
    pub fn definition(&self) -> Option<ProtocolDefinition> {
        let handle = unsafe { ffi::nw_shim_protocol_options_copy_definition(self.handle) };
        (!handle.is_null()).then_some(unsafe { ProtocolDefinition::from_raw(handle) })
    }

    #[must_use]
    pub fn is_quic(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_options_is_quic(self.handle) != 0 }
    }

    /// Configure a WebSocket client request handler on these options.
    pub fn set_ws_client_request_handler<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(WsRequest) -> Option<WsResponse> + Send + 'static,
    {
        let callback: Box<dyn FnMut(WsRequest) -> Option<WsResponse> + Send + 'static> =
            Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_ws_options_set_client_request_handler(
                self.handle,
                Some(ws_client_request_trampoline),
                raw,
            );
        };
        self.ws_client_request_callback = Some(arc);
        self
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self {
            handle,
            ws_client_request_callback: None,
        }
    }

    #[must_use]
    pub(crate) fn clone_from_raw(handle: *mut c_void) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(handle) };
        Self {
            handle,
            ws_client_request_callback: None,
        }
    }
}

impl Clone for ProtocolOptions {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self {
            handle,
            ws_client_request_callback: None,
        }
    }
}

unsafe extern "C" fn ws_client_request_trampoline(
    request: *mut c_void,
    user_info: *mut c_void,
) -> *mut c_void {
    if user_info.is_null() || request.is_null() {
        return core::ptr::null_mut();
    }
    let callback = unsafe { &*user_info.cast::<WsClientRequestHandlerCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return core::ptr::null_mut();
    };
    let request = unsafe { WsRequest::from_raw(request) };
    let mut response = core::ptr::null_mut();
    catch_user_panic("ws_client_request_trampoline", || {
        response = guard(request).map_or(core::ptr::null_mut(), WsResponse::into_raw);
    });
    response
}

impl Drop for ProtocolOptions {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Internet Protocol version preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    Any,
    V4,
    V6,
    Other(i32),
}

impl IpVersion {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Any,
            4 => Self::V4,
            6 => Self::V6,
            other => Self::Other(other),
        }
    }

    const fn as_raw(self) -> i32 {
        match self {
            Self::Any => 0,
            Self::V4 => 4,
            Self::V6 => 6,
            Self::Other(raw) => raw,
        }
    }
}

/// Local-address selection preference for IP connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpLocalAddressPreference {
    Default,
    Temporary,
    Stable,
    Other(i32),
}

impl IpLocalAddressPreference {
    const fn as_raw(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::Temporary => 1,
            Self::Stable => 2,
            Self::Other(raw) => raw,
        }
    }
}

/// Explicit congestion notification flags for IP metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpEcnFlag {
    NonEct,
    Ect0,
    Ect1,
    CongestionExperienced,
    Other(i32),
}

impl IpEcnFlag {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::NonEct,
            2 => Self::Ect0,
            1 => Self::Ect1,
            3 => Self::CongestionExperienced,
            other => Self::Other(other),
        }
    }

    const fn as_raw(self) -> i32 {
        match self {
            Self::NonEct => 0,
            Self::Ect0 => 2,
            Self::Ect1 => 1,
            Self::CongestionExperienced => 3,
            Self::Other(raw) => raw,
        }
    }
}

/// Forced MPTCP version for TCP options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpMultipathVersion {
    Unspecified,
    V0,
    V1,
    Other(i32),
}

impl TcpMultipathVersion {
    const fn as_raw(self) -> i32 {
        match self {
            Self::Unspecified => -1,
            Self::V0 => 0,
            Self::V1 => 1,
            Self::Other(raw) => raw,
        }
    }
}

/// Generic protocol metadata wrapper.
pub struct ProtocolMetadata {
    handle: *mut c_void,
    ws_pong_handler_callback: Option<Arc<WsPongHandlerCallback>>,
}

unsafe impl Send for ProtocolMetadata {}
unsafe impl Sync for ProtocolMetadata {}

impl std::fmt::Debug for ProtocolMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtocolMetadata")
            .field("handle", &self.handle)
            .field(
                "has_ws_pong_handler_callback",
                &self.ws_pong_handler_callback.is_some(),
            )
            .finish_non_exhaustive()
    }
}

impl ProtocolMetadata {
    /// Create IP metadata.
    pub fn ip() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ip_metadata() },
                "IP metadata",
            )?,
            ws_pong_handler_callback: None,
        })
    }

    /// Create UDP metadata.
    pub fn udp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_udp_metadata() },
                "UDP metadata",
            )?,
            ws_pong_handler_callback: None,
        })
    }

    /// Create WebSocket metadata for an opcode.
    pub fn websocket(opcode: Opcode) -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ws_metadata(opcode as c_int) },
                "WebSocket metadata",
            )?,
            ws_pong_handler_callback: None,
        })
    }

    /// # Safety
    ///
    /// `handle` must be a valid retained `nw_protocol_metadata_t` owned by the
    /// caller and remain alive for the returned wrapper.
    #[must_use]
    pub const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self {
            handle,
            ws_pong_handler_callback: None,
        }
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub fn is_ip(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_metadata_is_ip(self.handle) != 0 }
    }

    #[must_use]
    pub fn is_tcp(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_metadata_is_tcp(self.handle) != 0 }
    }

    #[must_use]
    pub fn is_tls(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_metadata_is_tls(self.handle) != 0 }
    }

    #[must_use]
    pub fn is_udp(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_metadata_is_udp(self.handle) != 0 }
    }

    #[must_use]
    pub fn is_websocket(&self) -> bool {
        unsafe { ffi::nw_shim_protocol_metadata_is_ws(self.handle) != 0 }
    }

    #[must_use]
    pub fn set_ip_ecn_flag(&mut self, ecn_flag: IpEcnFlag) -> &mut Self {
        unsafe { ffi::nw_shim_ip_metadata_set_ecn_flag(self.handle, ecn_flag.as_raw()) };
        self
    }

    #[must_use]
    pub fn ip_ecn_flag(&self) -> IpEcnFlag {
        IpEcnFlag::from_raw(unsafe { ffi::nw_shim_ip_metadata_get_ecn_flag(self.handle) })
    }

    pub fn set_ip_service_class(&mut self, service_class: ServiceClass) -> &mut Self {
        unsafe { ffi::nw_shim_ip_metadata_set_service_class(self.handle, service_class.as_raw()) };
        self
    }

    #[must_use]
    pub fn ip_service_class(&self) -> ServiceClass {
        ServiceClass::from_raw(unsafe { ffi::nw_shim_ip_metadata_get_service_class(self.handle) })
    }

    #[must_use]
    pub fn ip_receive_time(&self) -> u64 {
        unsafe { ffi::nw_shim_ip_metadata_get_receive_time(self.handle) }
    }

    #[must_use]
    pub fn tcp_available_receive_buffer(&self) -> u32 {
        unsafe { ffi::nw_shim_tcp_get_available_receive_buffer(self.handle) }
    }

    #[must_use]
    pub fn tcp_available_send_buffer(&self) -> u32 {
        unsafe { ffi::nw_shim_tcp_get_available_send_buffer(self.handle) }
    }

    pub fn set_ws_close_code(&mut self, close_code: WsCloseCode) -> &mut Self {
        unsafe { ffi::nw_shim_ws_metadata_set_close_code(self.handle, close_code.as_raw()) };
        self
    }

    #[must_use]
    pub fn ws_close_code(&self) -> WsCloseCode {
        WsCloseCode::from_raw(unsafe { ffi::nw_shim_ws_metadata_get_close_code(self.handle) })
    }

    #[must_use]
    pub fn ws_server_response(&self) -> Option<WsResponse> {
        let handle = unsafe { ffi::nw_shim_ws_metadata_copy_server_response(self.handle) };
        (!handle.is_null()).then_some(unsafe { WsResponse::from_raw(handle) })
    }

    /// Register a handler that fires when a pong is received for this ping metadata.
    ///
    /// Wraps `nw_ws_metadata_set_pong_handler`.
    pub fn set_pong_handler<F>(&mut self, handler: F) -> &mut Self
    where
        F: FnMut(Option<FrameworkError>) + Send + Sync + 'static,
    {
        let handler: Box<dyn FnMut(Option<FrameworkError>) + Send + Sync + 'static> =
            Box::new(handler);
        let arc = Arc::new(Mutex::new(handler));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_ws_metadata_set_pong_handler(
                self.handle,
                Some(ws_pong_trampoline),
                raw,
            );
        }
        self.ws_pong_handler_callback = Some(arc);
        self
    }

    #[must_use]
    pub fn tls_security_metadata(&self) -> Option<SecurityProtocolMetadata> {
        let handle = unsafe { ffi::nw_shim_tls_copy_sec_protocol_metadata(self.handle) };
        (!handle.is_null()).then_some(unsafe { SecurityProtocolMetadata::from_raw(handle) })
    }
}

impl Clone for ProtocolMetadata {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self {
            handle,
            ws_pong_handler_callback: None,
        }
    }
}

unsafe extern "C" fn ws_pong_trampoline(error: *mut c_void, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<WsPongHandlerCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    let error = (!error.is_null()).then_some(unsafe { FrameworkError::from_raw(error) });
    catch_user_panic("ws_pong_trampoline", || guard(error));
}

impl Drop for ProtocolMetadata {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl ProtocolOptions {
    /// Create WebSocket options with an explicit protocol version.
    pub fn websocket_with_version(version: WsVersion) -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ws_options_with_version(version.as_raw()) },
                "WebSocket options",
            )?,
            ws_client_request_callback: None,
        })
    }

    /// Copy the associated TLS security options.
    #[must_use]
    pub fn tls_security_options(&self) -> Option<SecurityProtocolOptions> {
        let handle = unsafe { ffi::nw_shim_tls_copy_sec_protocol_options(self.handle) };
        (!handle.is_null()).then_some(unsafe { SecurityProtocolOptions::from_raw(handle) })
    }

    pub fn set_ip_version(&mut self, version: IpVersion) -> &mut Self {
        unsafe { ffi::nw_shim_ip_options_set_version(self.handle, version.as_raw()) };
        self
    }

    pub fn set_ip_hop_limit(&mut self, hop_limit: u8) -> &mut Self {
        unsafe { ffi::nw_shim_ip_options_set_hop_limit(self.handle, hop_limit) };
        self
    }

    pub fn set_ip_use_minimum_mtu(&mut self, use_minimum_mtu: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_ip_options_set_use_minimum_mtu(self.handle, i32::from(use_minimum_mtu));
        };
        self
    }

    pub fn set_ip_disable_fragmentation(&mut self, disable_fragmentation: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_ip_options_set_disable_fragmentation(
                self.handle,
                i32::from(disable_fragmentation),
            );
        };
        self
    }

    pub fn set_ip_calculate_receive_time(&mut self, calculate_receive_time: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_ip_options_set_calculate_receive_time(
                self.handle,
                i32::from(calculate_receive_time),
            );
        };
        self
    }

    pub fn set_ip_local_address_preference(
        &mut self,
        preference: IpLocalAddressPreference,
    ) -> &mut Self {
        unsafe {
            ffi::nw_shim_ip_options_set_local_address_preference(self.handle, preference.as_raw());
        };
        self
    }

    pub fn set_ip_disable_multicast_loopback(
        &mut self,
        disable_multicast_loopback: bool,
    ) -> &mut Self {
        unsafe {
            ffi::nw_shim_ip_options_set_disable_multicast_loopback(
                self.handle,
                i32::from(disable_multicast_loopback),
            );
        };
        self
    }

    pub fn set_tcp_no_delay(&mut self, no_delay: bool) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_no_delay(self.handle, i32::from(no_delay)) };
        self
    }

    pub fn set_tcp_no_push(&mut self, no_push: bool) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_no_push(self.handle, i32::from(no_push)) };
        self
    }

    pub fn set_tcp_no_options(&mut self, no_options: bool) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_no_options(self.handle, i32::from(no_options)) };
        self
    }

    pub fn set_tcp_enable_keepalive(&mut self, enable_keepalive: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_enable_keepalive(self.handle, i32::from(enable_keepalive));
        };
        self
    }

    pub fn set_tcp_keepalive_count(&mut self, keepalive_count: u32) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_keepalive_count(self.handle, keepalive_count) };
        self
    }

    pub fn set_tcp_keepalive_idle_time(&mut self, keepalive_idle_time: u32) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_keepalive_idle_time(self.handle, keepalive_idle_time);
        };
        self
    }

    pub fn set_tcp_keepalive_interval(&mut self, keepalive_interval: u32) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_keepalive_interval(self.handle, keepalive_interval) };
        self
    }

    pub fn set_tcp_maximum_segment_size(&mut self, maximum_segment_size: u32) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_maximum_segment_size(self.handle, maximum_segment_size);
        };
        self
    }

    pub fn set_tcp_connection_timeout(&mut self, connection_timeout: u32) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_connection_timeout(self.handle, connection_timeout) };
        self
    }

    pub fn set_tcp_persist_timeout(&mut self, persist_timeout: u32) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_persist_timeout(self.handle, persist_timeout) };
        self
    }

    pub fn set_tcp_retransmit_connection_drop_time(&mut self, value: u32) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_retransmit_connection_drop_time(self.handle, value) };
        self
    }

    pub fn set_tcp_retransmit_fin_drop(&mut self, retransmit_fin_drop: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_retransmit_fin_drop(
                self.handle,
                i32::from(retransmit_fin_drop),
            );
        };
        self
    }

    pub fn set_tcp_disable_ack_stretching(&mut self, disable_ack_stretching: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_disable_ack_stretching(
                self.handle,
                i32::from(disable_ack_stretching),
            );
        };
        self
    }

    pub fn set_tcp_enable_fast_open(&mut self, enable_fast_open: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_enable_fast_open(self.handle, i32::from(enable_fast_open));
        };
        self
    }

    pub fn set_tcp_disable_ecn(&mut self, disable_ecn: bool) -> &mut Self {
        unsafe { ffi::nw_shim_tcp_options_set_disable_ecn(self.handle, i32::from(disable_ecn)) };
        self
    }

    pub fn set_tcp_multipath_force_version(
        &mut self,
        multipath_force_version: TcpMultipathVersion,
    ) -> &mut Self {
        unsafe {
            ffi::nw_shim_tcp_options_set_multipath_force_version(
                self.handle,
                multipath_force_version.as_raw(),
            );
        };
        self
    }

    pub fn set_udp_prefer_no_checksum(&mut self, prefer_no_checksum: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_udp_options_set_prefer_no_checksum(
                self.handle,
                i32::from(prefer_no_checksum),
            );
        };
        self
    }

    pub fn add_ws_additional_header(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<&mut Self, NetworkError> {
        let name = CString::new(name)
            .map_err(|e| NetworkError::InvalidArgument(format!("name NUL byte: {e}")))?;
        let value = CString::new(value)
            .map_err(|e| NetworkError::InvalidArgument(format!("value NUL byte: {e}")))?;
        unsafe {
            ffi::nw_shim_ws_options_add_additional_header(
                self.handle,
                name.as_ptr(),
                value.as_ptr(),
            );
        };
        Ok(self)
    }

    pub fn add_ws_subprotocol(&mut self, subprotocol: &str) -> Result<&mut Self, NetworkError> {
        let subprotocol = CString::new(subprotocol)
            .map_err(|e| NetworkError::InvalidArgument(format!("subprotocol NUL byte: {e}")))?;
        unsafe { ffi::nw_shim_ws_options_add_subprotocol(self.handle, subprotocol.as_ptr()) };
        Ok(self)
    }

    pub fn set_ws_auto_reply_ping(&mut self, auto_reply_ping: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_ws_options_set_auto_reply_ping(self.handle, i32::from(auto_reply_ping));
        };
        self
    }

    pub fn set_ws_skip_handshake(&mut self, skip_handshake: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_ws_options_set_skip_handshake(self.handle, i32::from(skip_handshake));
        };
        self
    }

    pub fn set_ws_maximum_message_size(&mut self, maximum_message_size: usize) -> &mut Self {
        unsafe {
            ffi::nw_shim_ws_options_set_maximum_message_size(self.handle, maximum_message_size);
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::{Opcode, ProtocolMetadata, ProtocolOptions};
    use crate::client::ContentContext;
    use crate::parameters::ConnectionParameters;
    use crate::websocket::{WebSocket, WsResponse, WsResponseStatus};
    use crate::{ffi, TcpListener};

    #[test]
    fn websocket_pong_handler_runs_for_ping_metadata() -> Result<(), crate::error::NetworkError> {
        let mut server_parameters = ConnectionParameters::tcp()?;
        let mut server_ws_options = ProtocolOptions::websocket()?;
        server_ws_options
            .set_ws_auto_reply_ping(false)
            .set_ws_client_request_handler(|_request| {
                Some(WsResponse::new(WsResponseStatus::Accept, None).expect("accept WebSocket"))
            });
        server_parameters.prepend_application_protocol(&server_ws_options)?;

        let listener = TcpListener::bind_with_parameters(0, &server_parameters)?;
        let port = listener.local_port();
        let server = thread::spawn(move || -> Result<(), crate::error::NetworkError> {
            let connection = listener.accept()?;
            let ping = connection.receive_with_context(1024)?;
            assert_eq!(ping.data, b"doomfish-ping");

            let pong_metadata = ProtocolMetadata::websocket(Opcode::Pong)?;
            let pong_context = ContentContext::new("ws-pong")?;
            unsafe {
                ffi::nw_shim_content_context_set_protocol_metadata(
                    pong_context.as_ptr(),
                    pong_metadata.as_ptr(),
                );
            }
            connection.send_with_context(&ping.data, &pong_context)?;
            Ok(())
        });

        let client = WebSocket::connect("127.0.0.1", port, "/", false)?;

        let (tx, rx) = mpsc::channel();
        let mut ping_metadata = ProtocolMetadata::websocket(Opcode::Ping)?;
        ping_metadata.set_pong_handler(move |error| {
            tx.send(error.map(|error| (error.domain(), error.code())))
                .expect("pong callback send");
        });

        let ping_context = ContentContext::new("ws-ping")?;
        unsafe {
            ffi::nw_shim_content_context_set_protocol_metadata(
                ping_context.as_ptr(),
                ping_metadata.as_ptr(),
            );
        }
        let status = unsafe {
            ffi::nw_shim_connection_send_with_context(
                client.as_ptr(),
                b"doomfish-ping".as_ptr(),
                b"doomfish-ping".len(),
                ping_context.as_ptr(),
            )
        };
        assert_eq!(status, ffi::NW_OK, "failed to send ping metadata over WebSocket");

        let callback_error = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("receive pong callback");
        assert!(
            callback_error.is_none(),
            "unexpected pong callback error: {callback_error:?}"
        );

        server.join().expect("server thread")?;
        Ok(())
    }
}
