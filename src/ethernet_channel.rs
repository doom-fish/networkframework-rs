//! Ethernet channel wrappers.

#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::sync::{Arc, Mutex};

use crate::{
    error::{from_status, NetworkError},
    ffi,
    interface::NetworkInterface,
    parameters::{ConnectionParameters, KeepAlives},
};

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

/// Ethernet channel lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthernetChannelState {
    Invalid,
    Waiting,
    Preparing,
    Ready,
    Failed,
    Cancelled,
    Unknown(i32),
}

impl EthernetChannelState {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::Waiting,
            2 => Self::Preparing,
            3 => Self::Ready,
            4 => Self::Failed,
            5 => Self::Cancelled,
            other => Self::Unknown(other),
        }
    }
}

/// One received Ethernet frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthernetFrame {
    pub data: Vec<u8>,
    pub vlan_tag: u16,
    pub local_address: [u8; 6],
    pub remote_address: [u8; 6],
}

type StateCallback = Mutex<Box<dyn FnMut(EthernetChannelState) + Send + 'static>>;
type ReceiveCallback = Mutex<Box<dyn FnMut(EthernetFrame) + Send + 'static>>;

/// A custom `EtherType` data channel.
#[allow(clippy::type_complexity)]
pub struct EthernetChannel {
    handle: *mut c_void,
    state_callback: Option<Arc<StateCallback>>,
    receive_callback: Option<Arc<ReceiveCallback>>,
    _keepalives: KeepAlives,
}

unsafe impl Send for EthernetChannel {}
unsafe impl Sync for EthernetChannel {}

fn copy_mac_address(bytes: *const u8) -> [u8; 6] {
    if bytes.is_null() {
        return [0_u8; 6];
    }
    let mut address = [0_u8; 6];
    address.copy_from_slice(unsafe { std::slice::from_raw_parts(bytes, 6) });
    address
}

impl EthernetChannel {
    /// Create a custom `EtherType` channel on a specific interface.
    pub fn new(ether_type: u16, interface: &NetworkInterface) -> Result<Self, NetworkError> {
        let name = to_cstring(&interface.name, "interface.name")?;
        let handle = unsafe {
            ffi::nw_shim_ethernet_channel_create(
                ether_type,
                name.as_ptr(),
                interface.interface_type.as_raw(),
                interface.index,
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create Ethernet channel".into(),
            ));
        }
        Ok(Self {
            handle,
            state_callback: None,
            receive_callback: None,
            _keepalives: KeepAlives::empty(),
        })
    }

    /// Create a custom `EtherType` channel with explicit connection parameters.
    pub fn with_parameters(
        ether_type: u16,
        interface: &NetworkInterface,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let name = to_cstring(&interface.name, "interface.name")?;
        let handle = unsafe {
            ffi::nw_shim_ethernet_channel_create_with_parameters(
                ether_type,
                name.as_ptr(),
                interface.interface_type.as_raw(),
                interface.index,
                parameters.as_ptr(),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create Ethernet channel with parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            state_callback: None,
            receive_callback: None,
            _keepalives: parameters.keepalives(),
        })
    }

    /// Set a state-change callback. Call before [`start`](Self::start).
    pub fn set_state_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(EthernetChannelState) + Send + 'static,
    {
        let callback: Box<dyn FnMut(EthernetChannelState) + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_ethernet_channel_set_state_changed_handler(
                self.handle,
                Some(state_trampoline),
                raw,
            )
        };
        self.state_callback = Some(arc);
    }

    /// Set the receive callback. Call before [`start`](Self::start).
    pub fn set_receive_handler<F>(&mut self, callback: F)
    where
        F: FnMut(EthernetFrame) + Send + 'static,
    {
        let callback: Box<dyn FnMut(EthernetFrame) + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_ethernet_channel_set_receive_handler(
                self.handle,
                Some(receive_trampoline),
                raw,
            )
        };
        self.receive_callback = Some(arc);
    }

    /// Current maximum payload size for the channel.
    #[must_use]
    pub fn maximum_payload_size(&self) -> u32 {
        unsafe { ffi::nw_shim_ethernet_channel_get_maximum_payload_size(self.handle) }
    }

    /// Start the channel.
    pub fn start(&self) {
        unsafe { ffi::nw_shim_ethernet_channel_start(self.handle) };
    }

    /// Cancel the channel.
    pub fn cancel(&self) {
        unsafe { ffi::nw_shim_ethernet_channel_cancel(self.handle) };
    }

    /// Send one Ethernet frame.
    pub fn send(
        &self,
        data: &[u8],
        vlan_tag: u16,
        remote_address: [u8; 6],
    ) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_ethernet_channel_send(
                self.handle,
                data.as_ptr(),
                data.len(),
                vlan_tag,
                remote_address.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }
}

impl Drop for EthernetChannel {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_ethernet_channel_release(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

unsafe extern "C" fn state_trampoline(state: c_int, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<StateCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    guard(EthernetChannelState::from_raw(state));
}

unsafe extern "C" fn receive_trampoline(
    data: *const u8,
    len: usize,
    vlan_tag: u16,
    local_address: *const u8,
    remote_address: *const u8,
    user_info: *mut c_void,
) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<ReceiveCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    let data = if data.is_null() || len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
    };
    guard(EthernetFrame {
        data,
        vlan_tag,
        local_address: copy_mac_address(local_address),
        remote_address: copy_mac_address(remote_address),
    });
}
