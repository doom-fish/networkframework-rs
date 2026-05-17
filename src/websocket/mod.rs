//! [`WebSocket`] — RFC 6455 WebSocket client over Network.framework.

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};

use crate::error::{from_status, NetworkError};
use crate::ffi;

/// WebSocket message kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Text = 1,
    Binary = 2,
    Close = 8,
    Ping = 9,
    Pong = 10,
}

impl Opcode {
    pub(crate) const fn from_raw(v: i32) -> Self {
        match v {
            1 => Self::Text,
            8 => Self::Close,
            9 => Self::Ping,
            10 => Self::Pong,
            _ => Self::Binary,
        }
    }
}

fn copied_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

unsafe extern "C" fn collect_string(value: *const c_char, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let values = unsafe { &mut *user_info.cast::<Vec<String>>() };
    let value = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    values.push(value);
}

unsafe extern "C" fn collect_header(
    name: *const c_char,
    value: *const c_char,
    user_info: *mut c_void,
) -> c_int {
    if user_info.is_null() {
        return 0;
    }
    let headers = unsafe { &mut *user_info.cast::<Vec<(String, String)>>() };
    let name = if name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
    };
    let value = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    headers.push((name, value));
    1
}

/// WebSocket protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsVersion {
    Invalid,
    V13,
    Other(i32),
}

impl WsVersion {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::V13,
            other => Self::Other(other),
        }
    }

    pub(crate) const fn as_raw(self) -> i32 {
        match self {
            Self::Invalid => 0,
            Self::V13 => 1,
            Self::Other(raw) => raw,
        }
    }
}

/// WebSocket close code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsCloseCode {
    NormalClosure,
    GoingAway,
    ProtocolError,
    UnsupportedData,
    NoStatusReceived,
    AbnormalClosure,
    InvalidFramePayloadData,
    PolicyViolation,
    MessageTooBig,
    MandatoryExtension,
    InternalServerError,
    TlsHandshake,
    Other(i32),
}

impl WsCloseCode {
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            1000 => Self::NormalClosure,
            1001 => Self::GoingAway,
            1002 => Self::ProtocolError,
            1003 => Self::UnsupportedData,
            1005 => Self::NoStatusReceived,
            1006 => Self::AbnormalClosure,
            1007 => Self::InvalidFramePayloadData,
            1008 => Self::PolicyViolation,
            1009 => Self::MessageTooBig,
            1010 => Self::MandatoryExtension,
            1011 => Self::InternalServerError,
            1015 => Self::TlsHandshake,
            other => Self::Other(other),
        }
    }

    pub(crate) const fn as_raw(self) -> i32 {
        match self {
            Self::NormalClosure => 1000,
            Self::GoingAway => 1001,
            Self::ProtocolError => 1002,
            Self::UnsupportedData => 1003,
            Self::NoStatusReceived => 1005,
            Self::AbnormalClosure => 1006,
            Self::InvalidFramePayloadData => 1007,
            Self::PolicyViolation => 1008,
            Self::MessageTooBig => 1009,
            Self::MandatoryExtension => 1010,
            Self::InternalServerError => 1011,
            Self::TlsHandshake => 1015,
            Self::Other(raw) => raw,
        }
    }
}

/// Status returned from WebSocket client-request handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsResponseStatus {
    Invalid,
    Accept,
    Reject,
    Other(i32),
}

impl WsResponseStatus {
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::Accept,
            2 => Self::Reject,
            other => Self::Other(other),
        }
    }

    pub(crate) const fn as_raw(self) -> i32 {
        match self {
            Self::Invalid => 0,
            Self::Accept => 1,
            Self::Reject => 2,
            Self::Other(raw) => raw,
        }
    }
}

/// Opaque WebSocket client request object.
pub struct WsRequest {
    handle: *mut c_void,
}

unsafe impl Send for WsRequest {}
unsafe impl Sync for WsRequest {}

impl WsRequest {
    /// # Safety
    ///
    /// `handle` must be a valid retained `nw_ws_request_t` that remains alive
    /// for the lifetime of the returned wrapper.
    #[must_use]
    pub const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Copy the request subprotocol list.
    #[must_use]
    pub fn subprotocols(&self) -> Vec<String> {
        let mut subprotocols = Vec::new();
        unsafe {
            ffi::nw_shim_ws_request_enumerate_subprotocols(
                self.handle,
                Some(collect_string),
                std::ptr::addr_of_mut!(subprotocols).cast(),
            )
        };
        subprotocols
    }

    /// Copy the request headers.
    #[must_use]
    pub fn additional_headers(&self) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        unsafe {
            ffi::nw_shim_ws_request_enumerate_additional_headers(
                self.handle,
                Some(collect_header),
                std::ptr::addr_of_mut!(headers).cast(),
            )
        };
        headers
    }
}

impl Clone for WsRequest {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for WsRequest {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Opaque WebSocket server response object.
pub struct WsResponse {
    handle: *mut c_void,
}

unsafe impl Send for WsResponse {}
unsafe impl Sync for WsResponse {}

impl WsResponse {
    /// Create a WebSocket server response.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidArgument`] if the subprotocol contains an
    /// interior NUL byte or the framework fails to create the response.
    pub fn new(
        status: WsResponseStatus,
        selected_subprotocol: Option<&str>,
    ) -> Result<Self, NetworkError> {
        let selected_subprotocol = selected_subprotocol
            .map(|value| {
                CString::new(value).map_err(|e| {
                    NetworkError::InvalidArgument(format!("selected_subprotocol NUL byte: {e}"))
                })
            })
            .transpose()?;
        let handle = unsafe {
            ffi::nw_shim_ws_response_create(
                status.as_raw(),
                selected_subprotocol
                    .as_ref()
                    .map_or(core::ptr::null(), |value| value.as_ptr()),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create WebSocket response".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// # Safety
    ///
    /// `handle` must be a valid retained `nw_ws_response_t` that remains alive
    /// for the lifetime of the returned wrapper.
    #[must_use]
    pub const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    #[must_use]
    pub fn status(&self) -> WsResponseStatus {
        WsResponseStatus::from_raw(unsafe { ffi::nw_shim_ws_response_get_status(self.handle) })
    }

    #[must_use]
    pub fn selected_subprotocol(&self) -> Option<String> {
        copied_string(unsafe { ffi::nw_shim_ws_response_get_selected_subprotocol(self.handle) })
    }

    /// Add an additional HTTP header to the response.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidArgument`] if either header string
    /// contains an interior NUL byte.
    pub fn add_additional_header(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<&mut Self, NetworkError> {
        let name = CString::new(name)
            .map_err(|e| NetworkError::InvalidArgument(format!("name NUL byte: {e}")))?;
        let value = CString::new(value)
            .map_err(|e| NetworkError::InvalidArgument(format!("value NUL byte: {e}")))?;
        unsafe {
            ffi::nw_shim_ws_response_add_additional_header(
                self.handle,
                name.as_ptr(),
                value.as_ptr(),
            );
        };
        Ok(self)
    }

    #[must_use]
    pub fn additional_headers(&self) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        unsafe {
            ffi::nw_shim_ws_response_enumerate_additional_headers(
                self.handle,
                Some(collect_header),
                std::ptr::addr_of_mut!(headers).cast(),
            )
        };
        headers
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) fn into_raw(mut self) -> *mut c_void {
        let handle = self.handle;
        self.handle = core::ptr::null_mut();
        handle
    }
}

impl Clone for WsResponse {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for WsResponse {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// One received WebSocket message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WsMessage {
    pub opcode: Opcode,
    pub data: Vec<u8>,
}

/// RFC 6455 WebSocket client.
pub struct WebSocket {
    handle: *mut c_void,
}

unsafe impl Send for WebSocket {}
unsafe impl Sync for WebSocket {}

impl WebSocket {
    /// Open a WebSocket connection to `ws://host:port/path` (plain) or
    /// `wss://host:port/path` (TLS).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] on failure.
    pub fn connect(host: &str, port: u16, path: &str, use_tls: bool) -> Result<Self, NetworkError> {
        let host_c = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let path_c = CString::new(path)
            .map_err(|e| NetworkError::InvalidArgument(format!("path NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_ws_connect(
                host_c.as_ptr(),
                port,
                path_c.as_ptr(),
                c_int::from(use_tls),
                &mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self { handle })
    }

    /// Send a UTF-8 text message.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send_text(&self, text: &str) -> Result<(), NetworkError> {
        self.send(text.as_bytes(), Opcode::Text)
    }

    /// Send a binary message.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send_binary(&self, data: &[u8]) -> Result<(), NetworkError> {
        self.send(data, Opcode::Binary)
    }

    /// Send a ping (with optional payload).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send_ping(&self, payload: &[u8]) -> Result<(), NetworkError> {
        self.send(payload, Opcode::Ping)
    }

    /// Generic send.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send(&self, data: &[u8], opcode: Opcode) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_ws_send(self.handle, data.as_ptr(), data.len(), opcode as c_int)
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Receive one WebSocket message (blocking up to `max_len` bytes).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ReceiveFailed`].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn receive(&self, max_len: usize) -> Result<WsMessage, NetworkError> {
        let mut buf = vec![0u8; max_len];
        let mut op: c_int = 0;
        let n = unsafe { ffi::nw_shim_ws_receive(self.handle, buf.as_mut_ptr(), max_len, &mut op) };
        if n < 0 {
            return Err(from_status(n as i32));
        }
        buf.truncate(n as usize);
        Ok(WsMessage {
            opcode: Opcode::from_raw(op),
            data: buf,
        })
    }

    /// Register a pong handler callback for WebSocket messages.
    ///
    /// This registers a callback that will be invoked whenever a pong frame is received.
    /// The callback will receive a [`FrameworkError`] if there was an error processing the pong,
    /// or `None` if the pong was handled successfully.
    ///
    /// # Safety
    ///
    /// - The callback must be safe to call from the networking dispatch queue.
    /// - The provided metadata must be valid WebSocket protocol metadata.
    /// - The metadata pointer must remain valid while the callback is in use.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidArgument`] if metadata is null.
    pub unsafe fn set_pong_handler(
        &self,
        metadata: *mut c_void,
        callback: ffi::WsPongCallback,
    ) -> Result<(), NetworkError> {
        if metadata.is_null() {
            return Err(NetworkError::InvalidArgument(
                "metadata is null".to_string(),
            ));
        }
        unsafe {
            ffi::nw_shim_ws_metadata_set_pong_handler(
                metadata,
                Some(callback),
                core::ptr::null_mut(),
            );
        }
        Ok(())
    }

    /// Register a pong handler with user context.
    ///
    /// Similar to [`Self::set_pong_handler`] but allows passing user context that
    /// will be forwarded to the callback.
    ///
    /// # Safety
    ///
    /// - The callback must be safe to call from the networking dispatch queue.
    /// - The provided metadata must be valid WebSocket protocol metadata.
    /// - The metadata and `user_info` pointers must remain valid while the callback is in use.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::InvalidArgument`] if metadata is null.
    pub unsafe fn set_pong_handler_with_context(
        &self,
        metadata: *mut c_void,
        callback: ffi::WsPongCallback,
        user_info: *mut c_void,
    ) -> Result<(), NetworkError> {
        if metadata.is_null() {
            return Err(NetworkError::InvalidArgument(
                "metadata is null".to_string(),
            ));
        }
        unsafe {
            ffi::nw_shim_ws_metadata_set_pong_handler(metadata, Some(callback), user_info);
        }
        Ok(())
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
