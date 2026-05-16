#![allow(
    clippy::missing_const_for_fn,
    clippy::redundant_pub_crate,
    clippy::semicolon_if_nothing_returned
)]

use core::ffi::{c_char, c_void};
use std::ffi::CStr;

use crate::{
    ffi,
    interface::{InterfaceType, NetworkInterface},
};

/// Radio technology reported for an interface path sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceRadioType {
    Unknown,
    WiFiB,
    WiFiA,
    WiFiG,
    WiFiN,
    WiFiAc,
    WiFiAx,
    CellLte,
    CellEndcSub6,
    CellEndcMmw,
    CellNrSaSub6,
    CellNrSaMmw,
    CellWcdma,
    CellGsm,
    CellCdma,
    CellEvdo,
    Other(i32),
}

impl InterfaceRadioType {
    #[must_use]
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::WiFiB,
            2 => Self::WiFiA,
            3 => Self::WiFiG,
            4 => Self::WiFiN,
            5 => Self::WiFiAc,
            6 => Self::WiFiAx,
            0x80 => Self::CellLte,
            0x81 => Self::CellEndcSub6,
            0x82 => Self::CellEndcMmw,
            0x83 => Self::CellNrSaSub6,
            0x84 => Self::CellNrSaMmw,
            0x85 => Self::CellWcdma,
            0x86 => Self::CellGsm,
            0x87 => Self::CellCdma,
            0x88 => Self::CellEvdo,
            other => Self::Other(other),
        }
    }
}

impl InterfaceType {
    #[must_use]
    pub(crate) const fn as_raw(self) -> i32 {
        match self {
            Self::Other => 0,
            Self::WiFi => 1,
            Self::Cellular => 2,
            Self::Wired => 3,
            Self::Loopback => 4,
        }
    }
}

fn copied_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    value
}

#[must_use]
pub(crate) fn network_interface_from_parts(
    name: String,
    interface_type_raw: i32,
    index: u32,
) -> NetworkInterface {
    NetworkInterface {
        name,
        interface_type: InterfaceType::from_raw(interface_type_raw),
        index,
    }
}

#[must_use]
pub(crate) unsafe fn network_interface_from_handle(handle: *mut c_void) -> Option<NetworkInterface> {
    if handle.is_null() {
        return None;
    }
    let name = copied_string(unsafe { ffi::nw_shim_interface_copy_name(handle) });
    let interface_type = unsafe { ffi::nw_shim_interface_get_type(handle) };
    let index = unsafe { ffi::nw_shim_interface_get_index(handle) };
    unsafe { ffi::nw_shim_release_object(handle) };
    Some(network_interface_from_parts(name, interface_type, index))
}
