//! Protocol definitions and options.

#![allow(clippy::missing_errors_doc)]

use core::ffi::c_void;

use crate::error::NetworkError;
use crate::ffi;

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

pub struct ProtocolOptions {
    handle: *mut c_void,
}

unsafe impl Send for ProtocolOptions {}
unsafe impl Sync for ProtocolOptions {}

impl ProtocolOptions {
    pub fn tcp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_tcp_options() },
                "TCP options",
            )?,
        })
    }

    pub fn udp() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_udp_options() },
                "UDP options",
            )?,
        })
    }

    pub fn tls() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_tls_options() },
                "TLS options",
            )?,
        })
    }

    pub fn ip() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ip_options() },
                "IP options",
            )?,
        })
    }

    pub fn websocket() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_ws_options() },
                "WebSocket options",
            )?,
        })
    }

    pub fn quic() -> Result<Self, NetworkError> {
        Ok(Self {
            handle: handle_result(
                unsafe { ffi::nw_shim_protocol_create_quic_options() },
                "QUIC options",
            )?,
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

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) fn clone_from_raw(handle: *mut c_void) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(handle) };
        Self { handle }
    }
}

impl Clone for ProtocolOptions {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ProtocolOptions {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
