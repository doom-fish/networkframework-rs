//! Network interface enumeration helpers.

use core::ffi::{c_char, c_int, c_void};
use std::{collections::HashSet, ffi::CStr};

use crate::ffi;

/// High-level classification for a network interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceType {
    Other,
    WiFi,
    Cellular,
    Wired,
    Loopback,
}

impl InterfaceType {
    #[must_use]
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::WiFi,
            2 => Self::Cellular,
            3 => Self::Wired,
            4 => Self::Loopback,
            _ => Self::Other,
        }
    }
}

/// One visible network interface.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: InterfaceType,
    pub index: u32,
}

/// List interfaces visible to the default path monitor.
#[must_use]
pub fn list_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    unsafe {
        ffi::nw_shim_list_interfaces(
            collect_interface_trampoline,
            std::ptr::addr_of_mut!(interfaces).cast::<c_void>(),
        )
    };
    dedupe_interfaces(&mut interfaces);
    interfaces
}

#[must_use]
pub(crate) fn list_interfaces_for_monitor(handle: *mut c_void) -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    unsafe {
        ffi::nw_shim_path_monitor_enumerate_interfaces(
            handle,
            collect_interface_trampoline,
            std::ptr::addr_of_mut!(interfaces).cast::<c_void>(),
        )
    };
    dedupe_interfaces(&mut interfaces);
    interfaces
}

fn dedupe_interfaces(interfaces: &mut Vec<NetworkInterface>) {
    let mut seen = HashSet::new();
    interfaces.retain(|interface| seen.insert(interface.clone()));
}

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
