//! Path snapshots backed by `nw_path_t`.

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CStr;

use crate::endpoint::Endpoint;
use crate::ffi;
use crate::interface::{InterfaceType, NetworkInterface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    Invalid,
    Satisfied,
    Unsatisfied,
    Satisfiable,
    Unknown(i32),
}

impl PathStatus {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::Satisfied,
            2 => Self::Unsatisfied,
            3 => Self::Satisfiable,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathUnsatisfiedReason {
    NotAvailable,
    CellularDenied,
    WiFiDenied,
    LocalNetworkDenied,
    VpnInactive,
    Unknown(i32),
}

impl PathUnsatisfiedReason {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::NotAvailable,
            1 => Self::CellularDenied,
            2 => Self::WiFiDenied,
            3 => Self::LocalNetworkDenied,
            4 => Self::VpnInactive,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkQuality {
    Unknown,
    Minimal,
    Moderate,
    Good,
    Other(i32),
}

impl LinkQuality {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            10 => Self::Minimal,
            20 => Self::Moderate,
            30 => Self::Good,
            other => Self::Other(other),
        }
    }
}

pub struct Path {
    handle: *mut c_void,
}

unsafe impl Send for Path {}
unsafe impl Sync for Path {}

unsafe extern "C" fn collect_interface_trampoline(
    name: *const c_char,
    interface_type: c_int,
    index: u32,
    user_info: *mut c_void,
) -> c_int {
    if user_info.is_null() {
        return 0;
    }
    let interfaces = unsafe { &mut *user_info.cast::<Vec<NetworkInterface>>() };
    let name = if name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
    };
    interfaces.push(NetworkInterface {
        name,
        interface_type: InterfaceType::from_raw(interface_type),
        index,
    });
    1
}

impl Path {
    #[must_use]
    pub fn status(&self) -> PathStatus {
        PathStatus::from_raw(unsafe { ffi::nw_shim_path_get_status(self.handle) })
    }

    #[must_use]
    pub fn unsatisfied_reason(&self) -> PathUnsatisfiedReason {
        PathUnsatisfiedReason::from_raw(unsafe {
            ffi::nw_shim_path_get_unsatisfied_reason(self.handle)
        })
    }

    #[must_use]
    pub fn is_expensive(&self) -> bool {
        unsafe { ffi::nw_shim_path_is_expensive(self.handle) != 0 }
    }

    #[must_use]
    pub fn is_constrained(&self) -> bool {
        unsafe { ffi::nw_shim_path_is_constrained(self.handle) != 0 }
    }

    #[must_use]
    pub fn is_ultra_constrained(&self) -> bool {
        unsafe { ffi::nw_shim_path_is_ultra_constrained(self.handle) != 0 }
    }

    #[must_use]
    pub fn has_ipv4(&self) -> bool {
        unsafe { ffi::nw_shim_path_has_ipv4(self.handle) != 0 }
    }

    #[must_use]
    pub fn has_ipv6(&self) -> bool {
        unsafe { ffi::nw_shim_path_has_ipv6(self.handle) != 0 }
    }

    #[must_use]
    pub fn has_dns(&self) -> bool {
        unsafe { ffi::nw_shim_path_has_dns(self.handle) != 0 }
    }

    #[must_use]
    pub fn uses_interface_type(&self, interface_type: InterfaceType) -> bool {
        let raw = match interface_type {
            InterfaceType::Other => 0,
            InterfaceType::WiFi => 1,
            InterfaceType::Cellular => 2,
            InterfaceType::Wired => 3,
            InterfaceType::Loopback => 4,
        };
        unsafe { ffi::nw_shim_path_uses_interface_type(self.handle, raw) != 0 }
    }

    #[must_use]
    pub fn effective_local_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_path_copy_effective_local_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    #[must_use]
    pub fn effective_remote_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_path_copy_effective_remote_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    #[must_use]
    pub fn link_quality(&self) -> LinkQuality {
        LinkQuality::from_raw(unsafe { ffi::nw_shim_path_get_link_quality(self.handle) })
    }

    #[must_use]
    pub fn interfaces(&self) -> Vec<NetworkInterface> {
        let mut interfaces = Vec::new();
        unsafe {
            ffi::nw_shim_path_enumerate_interfaces(
                self.handle,
                collect_interface_trampoline,
                std::ptr::addr_of_mut!(interfaces).cast(),
            )
        };
        interfaces
    }

    /// Enumerate the gateways attached to this path snapshot.
    #[must_use]
    pub fn gateways(&self) -> Vec<Endpoint> {
        unsafe extern "C" fn collect_gateway(endpoint: *mut c_void, user_info: *mut c_void) -> c_int {
            if user_info.is_null() || endpoint.is_null() {
                return 0;
            }
            let endpoints = unsafe { &mut *user_info.cast::<Vec<Endpoint>>() };
            endpoints.push(unsafe { Endpoint::from_raw(endpoint) });
            1
        }

        let mut endpoints = Vec::new();
        unsafe {
            ffi::nw_shim_path_enumerate_gateways(
                self.handle,
                Some(collect_gateway),
                std::ptr::addr_of_mut!(endpoints).cast(),
            )
        };
        endpoints
    }

    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Clone for Path {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::nw_shim_path_is_equal(self.handle, other.handle) != 0 }
    }
}

impl Eq for Path {}

impl Drop for Path {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
