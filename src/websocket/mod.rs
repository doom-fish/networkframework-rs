//! [`WebSocket`] — RFC 6455 WebSocket client over Network.framework.

use core::ffi::{c_int, c_void};
use std::ffi::CString;

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
    const fn from_raw(v: i32) -> Self {
        match v {
            1 => Self::Text,
            8 => Self::Close,
            9 => Self::Ping,
            10 => Self::Pong,
            _ => Self::Binary,
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
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
