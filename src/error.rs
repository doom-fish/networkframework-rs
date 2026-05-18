//! Errors raised by [`networkframework`](crate).

use core::ffi::c_void;
use core::fmt;
use std::ffi::CStr;

use crate::ffi;

fn copied_string(ptr: *mut i8) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

/// Failure modes from the C shim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    InvalidArgument(String),
    ConnectFailed,
    SendFailed,
    ReceiveFailed,
    ListenFailed,
    Cancelled,
    Timeout,
    Unknown(i32),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            Self::ConnectFailed => write!(f, "connect failed"),
            Self::SendFailed => write!(f, "send failed"),
            Self::ReceiveFailed => write!(f, "receive failed"),
            Self::ListenFailed => write!(f, "listen failed"),
            Self::Cancelled => write!(f, "operation cancelled"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::Unknown(c) => write!(f, "unknown shim status {c}"),
        }
    }
}

impl std::error::Error for NetworkError {}

/// Network.framework error domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorDomain {
    Posix,
    Dns,
    Tls,
    WifiAware,
    Unknown(i32),
}

impl ErrorDomain {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Posix,
            2 => Self::Dns,
            3 => Self::Tls,
            4 => Self::WifiAware,
            other => Self::Unknown(other),
        }
    }

    #[must_use]
    pub fn name(self) -> Option<String> {
        match self {
            Self::Posix => copied_string(unsafe { ffi::nw_shim_error_copy_posix_domain() }),
            Self::Dns => copied_string(unsafe { ffi::nw_shim_error_copy_dns_domain() }),
            Self::Tls => copied_string(unsafe { ffi::nw_shim_error_copy_tls_domain() }),
            Self::WifiAware => {
                copied_string(unsafe { ffi::nw_shim_error_copy_wifi_aware_domain() })
            }
            Self::Unknown(_) => None,
        }
    }
}

/// Opaque `nw_error_t` wrapper for advanced error inspection.
pub struct FrameworkError {
    handle: *mut c_void,
}

unsafe impl Send for FrameworkError {}
unsafe impl Sync for FrameworkError {}

impl fmt::Debug for FrameworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameworkError")
            .field("handle", &self.handle)
            .field("domain", &self.domain())
            .field("code", &self.code())
            .finish()
    }
}

impl FrameworkError {
    /// # Safety
    ///
    /// `handle` must be a valid retained `nw_error_t` that remains alive for the
    /// lifetime of the returned wrapper.
    #[must_use]
    pub const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    #[must_use]
    pub fn domain(&self) -> ErrorDomain {
        ErrorDomain::from_raw(unsafe { ffi::nw_shim_error_get_domain(self.handle) })
    }

    #[must_use]
    pub fn code(&self) -> i32 {
        unsafe { ffi::nw_shim_error_get_code(self.handle) }
    }

    #[must_use]
    pub fn cf_error_domain(&self) -> Option<String> {
        copied_string(unsafe { ffi::nw_shim_error_copy_cf_error_domain(self.handle) })
    }

    #[must_use]
    pub fn cf_error_description(&self) -> Option<String> {
        copied_string(unsafe { ffi::nw_shim_error_copy_cf_error_description(self.handle) })
    }

    /// Get the underlying Core Foundation [`CFError`](apple_cf::cf::CFError) from this framework error.
    ///
    /// Returns `Some(CFError)` if a valid `CFError` is available, `None` otherwise.
    /// The returned `CFError` is owned and will be automatically released when dropped.
    #[must_use]
    pub fn copy_cf_error(&self) -> Option<apple_cf::cf::CFError> {
        let cf_error_ptr = unsafe { ffi::nw_shim_error_copy_cf_error(self.handle) };
        // nw_error_copy_cf_error returns a retained CFErrorRef, so we use from_raw_retained
        unsafe { apple_cf::cf::CFError::from_raw_retained(cf_error_ptr) }
    }
}

impl Clone for FrameworkError {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for FrameworkError {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

#[must_use]
pub(crate) fn from_status(code: i32) -> NetworkError {
    use crate::ffi::{
        NW_CANCELLED, NW_CONNECT_FAILED, NW_INVALID_ARG, NW_LISTEN_FAILED, NW_RECV_FAILED,
        NW_SEND_FAILED, NW_TIMEOUT,
    };
    match code {
        NW_INVALID_ARG => NetworkError::InvalidArgument("shim".into()),
        NW_CONNECT_FAILED => NetworkError::ConnectFailed,
        NW_SEND_FAILED => NetworkError::SendFailed,
        NW_RECV_FAILED => NetworkError::ReceiveFailed,
        NW_LISTEN_FAILED => NetworkError::ListenFailed,
        NW_CANCELLED => NetworkError::Cancelled,
        NW_TIMEOUT => NetworkError::Timeout,
        other => NetworkError::Unknown(other),
    }
}
