#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};

use crate::{
    endpoint::Endpoint,
    error::NetworkError,
    ffi,
    interface::{InterfaceType, NetworkInterface},
    interface_support::{network_interface_from_handle, network_interface_from_parts},
    parameters::ConnectionParameters,
    protocol::ProtocolOptions,
};

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

unsafe fn copied_optional_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

/// Connection service class applied to new paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceClass {
    BestEffort,
    Background,
    InteractiveVideo,
    InteractiveVoice,
    ResponsiveData,
    Signaling,
    Unknown(i32),
}

impl ServiceClass {
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::BestEffort,
            1 => Self::Background,
            2 => Self::InteractiveVideo,
            3 => Self::InteractiveVoice,
            4 => Self::ResponsiveData,
            5 => Self::Signaling,
            other => Self::Unknown(other),
        }
    }

    pub(crate) const fn as_raw(self) -> i32 {
        match self {
            Self::BestEffort => 0,
            Self::Background => 1,
            Self::InteractiveVideo => 2,
            Self::InteractiveVoice => 3,
            Self::ResponsiveData => 4,
            Self::Signaling => 5,
            Self::Unknown(raw) => raw,
        }
    }
}

/// Multipath policy applied to new connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipathService {
    Disabled,
    Handover,
    Interactive,
    Aggregate,
    Unknown(i32),
}

impl MultipathService {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Disabled,
            1 => Self::Handover,
            2 => Self::Interactive,
            3 => Self::Aggregate,
            other => Self::Unknown(other),
        }
    }

    const fn as_raw(self) -> i32 {
        match self {
            Self::Disabled => 0,
            Self::Handover => 1,
            Self::Interactive => 2,
            Self::Aggregate => 3,
            Self::Unknown(raw) => raw,
        }
    }
}

/// Policy controlling whether expired DNS answers may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiredDnsBehavior {
    Default,
    Allow,
    Prohibit,
    Persistent,
    Unknown(i32),
}

impl ExpiredDnsBehavior {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Default,
            1 => Self::Allow,
            2 => Self::Prohibit,
            3 => Self::Persistent,
            other => Self::Unknown(other),
        }
    }

    const fn as_raw(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::Allow => 1,
            Self::Prohibit => 2,
            Self::Persistent => 3,
            Self::Unknown(raw) => raw,
        }
    }
}

/// Mutable wrapper around `nw_protocol_stack_t`.
pub struct ProtocolStack {
    handle: *mut c_void,
}

unsafe impl Send for ProtocolStack {}
unsafe impl Sync for ProtocolStack {}

impl ProtocolStack {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Remove every application protocol from the stack.
    pub fn clear_application_protocols(&mut self) -> &mut Self {
        unsafe { ffi::nw_shim_protocol_stack_clear_application_protocols(self.handle) };
        self
    }

    /// Copy the current application-protocol list.
    #[must_use]
    pub fn application_protocols(&self) -> Vec<ProtocolOptions> {
        let mut count = 0_usize;
        let items = unsafe {
            ffi::nw_shim_protocol_stack_copy_application_protocols(self.handle, &mut count)
        };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let protocols = slice
            .iter()
            .filter_map(|handle| (!handle.is_null()).then_some(unsafe { ProtocolOptions::from_raw(*handle) }))
            .collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        protocols
    }

    /// Copy the transport protocol, if one is configured.
    #[must_use]
    pub fn transport_protocol(&self) -> Option<ProtocolOptions> {
        let handle = unsafe { ffi::nw_shim_protocol_stack_copy_transport_protocol(self.handle) };
        (!handle.is_null()).then_some(unsafe { ProtocolOptions::from_raw(handle) })
    }

    /// Replace the current transport protocol.
    pub fn set_transport_protocol(&mut self, protocol: &ProtocolOptions) -> &mut Self {
        unsafe { ffi::nw_shim_protocol_stack_set_transport_protocol(self.handle, protocol.as_ptr()) };
        self
    }

    /// Copy the internet protocol, if one is configured.
    #[must_use]
    pub fn internet_protocol(&self) -> Option<ProtocolOptions> {
        let handle = unsafe { ffi::nw_shim_protocol_stack_copy_internet_protocol(self.handle) };
        (!handle.is_null()).then_some(unsafe { ProtocolOptions::from_raw(handle) })
    }
}

impl Clone for ProtocolStack {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ProtocolStack {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl ConnectionParameters {
    /// Whether the framework should prefer a direct path before trying configured proxies.
    #[must_use]
    pub fn prefer_no_proxy(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_prefer_no_proxy(self.as_ptr()) != 0 }
    }

    /// Require a specific interface for path evaluation, or clear the requirement.
    pub fn require_interface(
        &mut self,
        interface: Option<&NetworkInterface>,
    ) -> Result<&mut Self, NetworkError> {
        match interface {
            Some(interface) => {
                let name = to_cstring(&interface.name, "interface.name")?;
                unsafe {
                    ffi::nw_shim_parameters_require_interface(
                        self.as_ptr(),
                        name.as_ptr(),
                        interface.interface_type.as_raw(),
                        interface.index,
                    )
                };
            }
            None => unsafe {
                ffi::nw_shim_parameters_require_interface(self.as_ptr(), core::ptr::null(), 0, 0)
            },
        }
        Ok(self)
    }

    /// Copy the currently required interface, if any.
    #[must_use]
    pub fn required_interface(&self) -> Option<NetworkInterface> {
        let mut name = core::ptr::null_mut();
        let mut interface_type: c_int = 0;
        let mut index = 0_u32;
        let found = unsafe {
            ffi::nw_shim_parameters_copy_required_interface(
                self.as_ptr(),
                &mut name,
                &mut interface_type,
                &mut index,
            )
        };
        if found == 0 {
            return None;
        }
        let name = unsafe { copied_optional_string(name) }.unwrap_or_default();
        Some(network_interface_from_parts(name, interface_type, index))
    }

    /// Add an interface to the prohibited-interface list.
    pub fn prohibit_interface(
        &mut self,
        interface: &NetworkInterface,
    ) -> Result<&mut Self, NetworkError> {
        let name = to_cstring(&interface.name, "interface.name")?;
        unsafe {
            ffi::nw_shim_parameters_prohibit_interface(
                self.as_ptr(),
                name.as_ptr(),
                interface.interface_type.as_raw(),
                interface.index,
            )
        };
        Ok(self)
    }

    /// Remove every prohibited interface.
    pub fn clear_prohibited_interfaces(&mut self) -> &mut Self {
        unsafe { ffi::nw_shim_parameters_clear_prohibited_interfaces(self.as_ptr()) };
        self
    }

    /// Copy the prohibited-interface list.
    #[must_use]
    pub fn prohibited_interfaces(&self) -> Vec<NetworkInterface> {
        let mut count = 0_usize;
        let items = unsafe { ffi::nw_shim_parameters_copy_prohibited_interfaces(self.as_ptr(), &mut count) };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let interfaces = slice
            .iter()
            .filter_map(|handle| unsafe { network_interface_from_handle(*handle) })
            .collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        interfaces
    }

    /// Add an interface type to the prohibited-interface-type list.
    pub fn prohibit_interface_type(&mut self, interface_type: InterfaceType) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_prohibit_interface_type(self.as_ptr(), interface_type.as_raw())
        };
        self
    }

    /// Remove every prohibited interface type.
    pub fn clear_prohibited_interface_types(&mut self) -> &mut Self {
        unsafe { ffi::nw_shim_parameters_clear_prohibited_interface_types(self.as_ptr()) };
        self
    }

    /// Copy the prohibited interface types.
    #[must_use]
    pub fn prohibited_interface_types(&self) -> Vec<InterfaceType> {
        let mut count = 0_usize;
        let items = unsafe {
            ffi::nw_shim_parameters_copy_prohibited_interface_types(self.as_ptr(), &mut count)
        };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let types = slice.iter().copied().map(InterfaceType::from_raw).collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        types
    }

    /// Reuse the local address when binding.
    pub fn set_reuse_local_address(&mut self, reuse_local_address: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_reuse_local_address(
                self.as_ptr(),
                c_int::from(reuse_local_address),
            )
        };
        self
    }

    /// Whether the local address may be reused when binding.
    #[must_use]
    pub fn reuse_local_address(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_reuse_local_address(self.as_ptr()) != 0 }
    }

    /// Set or clear the local endpoint used for new connections.
    pub fn set_local_endpoint(&mut self, local_endpoint: Option<&Endpoint>) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_local_endpoint(
                self.as_ptr(),
                local_endpoint.map_or(core::ptr::null_mut(), Endpoint::as_ptr),
            )
        };
        self
    }

    /// Copy the currently configured local endpoint.
    #[must_use]
    pub fn local_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_parameters_copy_local_endpoint(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Allow or disallow peer-to-peer transports such as AWDL.
    pub fn set_include_peer_to_peer(&mut self, include_peer_to_peer: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_include_peer_to_peer(
                self.as_ptr(),
                c_int::from(include_peer_to_peer),
            )
        };
        self
    }

    /// Whether peer-to-peer transports are allowed.
    #[must_use]
    pub fn include_peer_to_peer(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_include_peer_to_peer(self.as_ptr()) != 0 }
    }

    /// Enable or disable TCP Fast Open.
    pub fn set_fast_open_enabled(&mut self, fast_open_enabled: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_fast_open_enabled(
                self.as_ptr(),
                c_int::from(fast_open_enabled),
            )
        };
        self
    }

    /// Whether TCP Fast Open is enabled.
    #[must_use]
    pub fn fast_open_enabled(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_fast_open_enabled(self.as_ptr()) != 0 }
    }

    /// Set the service class for new flows.
    pub fn set_service_class(&mut self, service_class: ServiceClass) -> &mut Self {
        unsafe { ffi::nw_shim_parameters_set_service_class(self.as_ptr(), service_class.as_raw()) };
        self
    }

    /// Current service class.
    #[must_use]
    pub fn service_class(&self) -> ServiceClass {
        ServiceClass::from_raw(unsafe { ffi::nw_shim_parameters_get_service_class(self.as_ptr()) })
    }

    /// Set the multipath policy.
    pub fn set_multipath_service(&mut self, multipath_service: MultipathService) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_multipath_service(
                self.as_ptr(),
                multipath_service.as_raw(),
            )
        };
        self
    }

    /// Current multipath policy.
    #[must_use]
    pub fn multipath_service(&self) -> MultipathService {
        MultipathService::from_raw(unsafe {
            ffi::nw_shim_parameters_get_multipath_service(self.as_ptr())
        })
    }

    /// Copy the default protocol stack for these parameters.
    #[must_use]
    pub fn default_protocol_stack(&self) -> Option<ProtocolStack> {
        let handle = unsafe { ffi::nw_shim_parameters_copy_default_protocol_stack(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { ProtocolStack::from_raw(handle) })
    }

    /// Restrict new paths to local interfaces only.
    pub fn set_local_only(&mut self, local_only: bool) -> &mut Self {
        unsafe { ffi::nw_shim_parameters_set_local_only(self.as_ptr(), c_int::from(local_only)) };
        self
    }

    /// Whether new paths are restricted to local interfaces only.
    #[must_use]
    pub fn local_only(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_get_local_only(self.as_ptr()) != 0 }
    }

    /// Control whether expired DNS answers may be reused.
    pub fn set_expired_dns_behavior(&mut self, expired_dns_behavior: ExpiredDnsBehavior) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_expired_dns_behavior(
                self.as_ptr(),
                expired_dns_behavior.as_raw(),
            )
        };
        self
    }

    /// Current expired-DNS policy.
    #[must_use]
    pub fn expired_dns_behavior(&self) -> ExpiredDnsBehavior {
        ExpiredDnsBehavior::from_raw(unsafe {
            ffi::nw_shim_parameters_get_expired_dns_behavior(self.as_ptr())
        })
    }

    /// Require DNSSEC validation for endpoint resolution.
    pub fn set_requires_dnssec_validation(&mut self, requires_dnssec_validation: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_parameters_set_requires_dnssec_validation(
                self.as_ptr(),
                c_int::from(requires_dnssec_validation),
            )
        };
        self
    }

    /// Whether DNSSEC validation is required.
    #[must_use]
    pub fn requires_dnssec_validation(&self) -> bool {
        unsafe { ffi::nw_shim_parameters_requires_dnssec_validation(self.as_ptr()) != 0 }
    }
}
