//! Configurable `nw_parameters` wrappers for advanced connections.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::sync::Arc;

use crate::error::NetworkError;
use crate::ffi;

pub(crate) type KeepAlive = Arc<dyn Send + Sync>;

#[derive(Clone, Default)]
pub(crate) struct KeepAlives(Vec<KeepAlive>);

impl KeepAlives {
    pub(crate) fn add<T>(&mut self, value: Arc<T>)
    where
        T: Send + Sync + 'static,
    {
        self.0.push(value);
    }

    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::default()
    }
}

/// Builder for advanced `nw_parameters_t` configuration.
pub struct ConnectionParameters {
    handle: *mut c_void,
    keepalives: KeepAlives,
}

unsafe impl Send for ConnectionParameters {}
unsafe impl Sync for ConnectionParameters {}

impl Clone for ConnectionParameters {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_parameters_copy(self.handle) };
        Self {
            handle,
            keepalives: self.keepalives.clone(),
        }
    }
}

impl ConnectionParameters {
    /// Create TCP parameters without TLS.
    pub fn tcp() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_parameters_create_tcp(0) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create TCP parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
    }

    /// Create TCP parameters with the system TLS configuration enabled.
    pub fn tls_tcp() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_parameters_create_tcp(1) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create TLS TCP parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
    }

    /// Create UDP parameters without DTLS.
    pub fn udp() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_parameters_create_udp() };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create UDP parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
    }

    /// Create QUIC parameters with the supplied ALPN string.
    pub fn quic(alpn: &str) -> Result<Self, NetworkError> {
        let alpn = CString::new(alpn)
            .map_err(|e| NetworkError::InvalidArgument(format!("alpn NUL byte: {e}")))?;
        let handle = unsafe { ffi::nw_shim_parameters_create_quic(alpn.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create QUIC parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
    }

    /// Attempt direct connections before trying configured proxies.
    pub fn set_prefer_no_proxy(&mut self, prefer_no_proxy: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_prefer_no_proxy(self.handle, c_int::from(prefer_no_proxy));
        }
        self
    }

    /// Attach a privacy context to these parameters.
    pub fn set_privacy_context(
        &mut self,
        privacy_context: &crate::privacy::PrivacyContext,
    ) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_privacy_context(self.handle, privacy_context.as_ptr());
        }
        self
    }

    /// Prepend a custom framer onto the application protocol stack.
    pub fn prepend_framer(
        &mut self,
        framer_options: &crate::framer::FramerOptions,
    ) -> Result<&mut Self, NetworkError> {
        let status = unsafe {
            ffi::nw_shim_parameters_prepend_application_protocol(
                self.handle,
                framer_options.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(crate::error::from_status(status));
        }
        self.keepalives.add(framer_options.keepalive());
        Ok(self)
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) fn keepalives(&self) -> KeepAlives {
        self.keepalives.clone()
    }
}

impl Drop for ConnectionParameters {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
