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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParametersAttribution {
    Developer = 1,
    User = 2,
}

impl ParametersAttribution {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            2 => Self::User,
            _ => Self::Developer,
        }
    }
}

/// Builder for advanced `nw_parameters_t` configuration.
pub struct ConnectionParameters {
    handle: *mut c_void,
    keepalives: KeepAlives,
}

unsafe impl Send for ConnectionParameters {}
unsafe impl Sync for ConnectionParameters {}

impl std::fmt::Debug for ConnectionParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionParameters")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

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
    /// Create generic parameters with an empty protocol stack.
    pub fn generic() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_parameters_create() };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create generic parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
    }

    /// Create parameters configured for application services.
    pub fn application_service() -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_parameters_create_application_service() };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create application-service parameters".into(),
            ));
        }
        Ok(Self {
            handle,
            keepalives: KeepAlives::empty(),
        })
    }

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

    /// Create custom-IP parameters for entitlement-gated transports.
    pub fn custom_ip(protocol_number: u8) -> Result<Self, NetworkError> {
        let handle = unsafe { ffi::nw_shim_parameters_create_custom_ip(protocol_number) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create custom-IP parameters".into(),
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

    /// Attribute the traffic created with these parameters to the developer or user.
    pub fn set_attribution(&mut self, attribution: ParametersAttribution) -> &mut Self {
        unsafe { ffi::nw_shim_parameters_set_attribution(self.handle, attribution as c_int) };
        self
    }

    /// Current content-attribution mode.
    #[must_use]
    pub fn attribution(&self) -> ParametersAttribution {
        ParametersAttribution::from_raw(unsafe {
            ffi::nw_shim_parameters_get_attribution(self.handle)
        })
    }

    /// Require the path to use a specific interface type.
    pub fn set_required_interface_type(
        &mut self,
        interface_type: crate::interface::InterfaceType,
    ) -> &mut Self {
        let raw = match interface_type {
            crate::interface::InterfaceType::Other => 0,
            crate::interface::InterfaceType::WiFi => 1,
            crate::interface::InterfaceType::Cellular => 2,
            crate::interface::InterfaceType::Wired => 3,
            crate::interface::InterfaceType::Loopback => 4,
        };
        unsafe { ffi::nw_shim_parameters_set_required_interface_type(self.handle, raw) };
        self
    }

    /// Required interface type, if any.
    #[must_use]
    pub fn required_interface_type(&self) -> crate::interface::InterfaceType {
        crate::interface::InterfaceType::from_raw(unsafe {
            ffi::nw_shim_parameters_get_required_interface_type(self.handle)
        })
    }

    /// Prohibit expensive interfaces such as cellular links.
    pub fn set_prohibit_expensive(&mut self, prohibit_expensive: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_prohibit_expensive(
                self.handle,
                c_int::from(prohibit_expensive),
            );
        }
        self
    }

    /// Whether expensive interfaces are prohibited.
    #[must_use]
    pub fn prohibit_expensive(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_prohibit_expensive(self.handle) != 0 }
    }

    /// Prohibit constrained interfaces.
    pub fn set_prohibit_constrained(&mut self, prohibit_constrained: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_prohibit_constrained(
                self.handle,
                c_int::from(prohibit_constrained),
            );
        }
        self
    }

    /// Whether constrained interfaces are prohibited.
    #[must_use]
    pub fn prohibit_constrained(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_prohibit_constrained(self.handle) != 0 }
    }

    /// Explicitly allow ultra-constrained interfaces when the OS supports them.
    pub fn set_allow_ultra_constrained(&mut self, allow_ultra_constrained: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_allow_ultra_constrained(
                self.handle,
                c_int::from(allow_ultra_constrained),
            );
        };
        self
    }

    /// Whether ultra-constrained interfaces are allowed.
    #[must_use]
    pub fn allow_ultra_constrained(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_allow_ultra_constrained(self.handle) != 0 }
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

    /// Prepend protocol options onto the application protocol stack.
    pub fn prepend_application_protocol(
        &mut self,
        protocol_options: &crate::protocol::ProtocolOptions,
    ) -> Result<&mut Self, NetworkError> {
        let status = unsafe {
            ffi::nw_shim_parameters_prepend_application_protocol(
                self.handle,
                protocol_options.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(crate::error::from_status(status));
        }
        Ok(self)
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
    pub(crate) unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self {
            handle,
            keepalives: KeepAlives::empty(),
        }
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
