//! Raw FFI declarations matching `src/c-shim/network_shim.c`.

#![allow(missing_docs)]

use core::ffi::{c_int, c_void};

pub const NW_OK: c_int = 0;
pub const NW_INVALID_ARG: c_int = -1;
pub const NW_CONNECT_FAILED: c_int = -2;
pub const NW_SEND_FAILED: c_int = -3;
pub const NW_RECV_FAILED: c_int = -4;
pub const NW_LISTEN_FAILED: c_int = -5;
pub const NW_CANCELLED: c_int = -6;
pub const NW_TIMEOUT: c_int = -7;

extern "C" {
    pub fn nw_shim_tcp_connect(
        host: *const core::ffi::c_char,
        port: u16,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_tcp_send(handle: *mut c_void, data: *const u8, len: usize) -> c_int;

    pub fn nw_shim_tcp_receive(handle: *mut c_void, out_buf: *mut u8, max_len: usize) -> isize;

    pub fn nw_shim_tcp_close(handle: *mut c_void);

    pub fn nw_shim_listener_create(
        port: u16,
        use_tls: c_int,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_listener_port(handle: *mut c_void) -> u16;

    pub fn nw_shim_listener_accept(handle: *mut c_void, out_status: *mut c_int) -> *mut c_void;

    pub fn nw_shim_listener_close(handle: *mut c_void);

    pub fn nw_shim_udp_connect(
        host: *const core::ffi::c_char,
        port: u16,
        out_status: *mut c_int,
    ) -> *mut c_void;

    pub fn nw_shim_path_monitor_start(
        callback: PathMonitorCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;

    pub fn nw_shim_path_monitor_stop(handle: *mut c_void);
}

pub type PathMonitorCallback = unsafe extern "C" fn(
    satisfied: c_int,
    interface_type: c_int,
    user_info: *mut c_void,
);
