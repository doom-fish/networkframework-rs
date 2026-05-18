//! Privacy contexts, proxy configuration, and encrypted resolver settings.

#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_char, c_void};
use std::ffi::{CStr, CString};

use crate::endpoint::Endpoint;
use crate::error::NetworkError;
use crate::ffi;
use crate::protocol::ProtocolOptions;

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

unsafe extern "C" fn collect_string_trampoline(value: *const c_char, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let values = unsafe { &mut *user_info.cast::<Vec<String>>() };
    let value = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    values.push(value);
}

/// Shared privacy and cache policy applied through [`crate::ConnectionParameters`].
pub struct PrivacyContext {
    handle: *mut c_void,
}

unsafe impl Send for PrivacyContext {}
unsafe impl Sync for PrivacyContext {}

impl std::fmt::Debug for PrivacyContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivacyContext")
            .field("handle", &self.handle)
            .finish()
    }
}

impl PrivacyContext {
    /// Create a named privacy context.
    pub fn new(description: &str) -> Result<Self, NetworkError> {
        let description = to_cstring(description, "description")?;
        let handle = unsafe { ffi::nw_shim_privacy_context_create(description.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create privacy context".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Copy the process-global default privacy context.
    #[must_use]
    pub fn default_context() -> Self {
        let handle = unsafe { ffi::nw_shim_privacy_context_copy_default() };
        Self { handle }
    }

    /// Flush any caches associated with the context.
    pub fn flush_cache(&self) {
        unsafe { ffi::nw_shim_privacy_context_flush_cache(self.handle) };
    }

    /// Disable Network.framework logging for this context.
    pub fn disable_logging(&self) {
        unsafe { ffi::nw_shim_privacy_context_disable_logging(self.handle) };
    }

    /// Require encrypted DNS resolution, optionally with a fallback encrypted resolver.
    pub fn require_encrypted_name_resolution(
        &self,
        required: bool,
        fallback: Option<&ResolverConfig>,
    ) {
        unsafe {
            ffi::nw_shim_privacy_context_require_encrypted_name_resolution(
                self.handle,
                i32::from(required),
                fallback.map_or(core::ptr::null_mut(), ResolverConfig::as_ptr),
            );
        }
    }

    /// Add a proxy configuration to the privacy context.
    pub fn add_proxy(&self, proxy: &ProxyConfig) {
        unsafe { ffi::nw_shim_privacy_context_add_proxy(self.handle, proxy.handle) };
    }

    /// Clear all proxy configurations from the context.
    pub fn clear_proxies(&self) {
        unsafe { ffi::nw_shim_privacy_context_clear_proxies(self.handle) };
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Clone for PrivacyContext {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for PrivacyContext {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Encrypted DNS resolver configuration.
pub struct ResolverConfig {
    handle: *mut c_void,
}

unsafe impl Send for ResolverConfig {}
unsafe impl Sync for ResolverConfig {}

impl std::fmt::Debug for ResolverConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolverConfig")
            .field("handle", &self.handle)
            .finish()
    }
}

impl ResolverConfig {
    /// Create a DNS-over-HTTPS resolver from a URL template.
    pub fn dns_over_https(url: &str) -> Result<Self, NetworkError> {
        let url = to_cstring(url, "url")?;
        let handle = unsafe { ffi::nw_shim_resolver_config_create_https(url.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create DNS-over-HTTPS resolver".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create a DNS-over-TLS resolver from a host and port.
    pub fn dns_over_tls(host: &str, port: u16) -> Result<Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let handle = unsafe { ffi::nw_shim_resolver_config_create_tls(host.as_ptr(), port) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create DNS-over-TLS resolver".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Add a DNS server address to the resolver configuration.
    pub fn add_server_address(
        &mut self,
        address: &str,
        port: u16,
    ) -> Result<&mut Self, NetworkError> {
        let address = to_cstring(address, "address")?;
        let status = unsafe {
            ffi::nw_shim_resolver_config_add_server_address(self.handle, address.as_ptr(), port)
        };
        if status != ffi::NW_OK {
            return Err(crate::error::from_status(status));
        }
        Ok(self)
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Clone for ResolverConfig {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ResolverConfig {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// A secure relay hop usable for relay and Oblivious HTTP proxy configurations.
pub struct RelayHop {
    handle: *mut c_void,
}

unsafe impl Send for RelayHop {}
unsafe impl Sync for RelayHop {}

impl std::fmt::Debug for RelayHop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayHop")
            .field("handle", &self.handle)
            .finish()
    }
}

impl RelayHop {
    /// Create a relay hop using optional HTTP/3 and HTTP/2 endpoints.
    pub fn new(
        http3_endpoint: Option<&Endpoint>,
        http2_endpoint: Option<&Endpoint>,
        relay_tls_options: Option<&ProtocolOptions>,
    ) -> Result<Self, NetworkError> {
        let handle = unsafe {
            ffi::nw_shim_relay_hop_create(
                http3_endpoint.map_or(core::ptr::null_mut(), Endpoint::as_ptr),
                http2_endpoint.map_or(core::ptr::null_mut(), Endpoint::as_ptr),
                relay_tls_options.map_or(core::ptr::null_mut(), ProtocolOptions::as_ptr),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create relay hop".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Attach an extra HTTP header field to CONNECT requests.
    pub fn add_additional_http_header_field(
        &mut self,
        field_name: &str,
        field_value: &str,
    ) -> Result<&mut Self, NetworkError> {
        let field_name = to_cstring(field_name, "field_name")?;
        let field_value = to_cstring(field_value, "field_value")?;
        unsafe {
            ffi::nw_shim_relay_hop_add_additional_http_header_field(
                self.handle,
                field_name.as_ptr(),
                field_value.as_ptr(),
            );
        };
        Ok(self)
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl Clone for RelayHop {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for RelayHop {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Proxy settings attachable to a [`PrivacyContext`].
pub struct ProxyConfig {
    handle: *mut c_void,
}

/// Minimal wrapper around `NSURLSessionConfiguration`'s Network.framework proxy settings.
pub struct UrlSessionConfiguration {
    handle: *mut c_void,
}

unsafe impl Send for ProxyConfig {}
unsafe impl Sync for ProxyConfig {}

impl std::fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("handle", &self.handle)
            .finish()
    }
}

impl std::fmt::Debug for UrlSessionConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UrlSessionConfiguration")
            .field("handle", &self.handle)
            .finish()
    }
}

impl ProxyConfig {
    /// Create an HTTP CONNECT proxy configuration.
    pub fn http_connect(host: &str, port: u16, use_tls: bool) -> Result<Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let handle = unsafe {
            ffi::nw_shim_proxy_config_create_http_connect(host.as_ptr(), port, i32::from(use_tls))
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create HTTP CONNECT proxy".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create a `SOCKSv5` proxy configuration.
    pub fn socksv5(host: &str, port: u16) -> Result<Self, NetworkError> {
        let host = to_cstring(host, "host")?;
        let handle = unsafe { ffi::nw_shim_proxy_config_create_socksv5(host.as_ptr(), port) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create SOCKSv5 proxy".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create a relay proxy configuration.
    pub fn relay(
        first_hop: &RelayHop,
        second_hop: Option<&RelayHop>,
    ) -> Result<Self, NetworkError> {
        let handle = unsafe {
            ffi::nw_shim_proxy_config_create_relay(
                first_hop.as_ptr(),
                second_hop.map_or(core::ptr::null_mut(), RelayHop::as_ptr),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create relay proxy configuration".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Create an Oblivious HTTP proxy configuration.
    pub fn oblivious_http(
        relay_hop: &RelayHop,
        relay_resource_path: &str,
        gateway_key_config: &[u8],
    ) -> Result<Self, NetworkError> {
        let relay_resource_path = to_cstring(relay_resource_path, "relay_resource_path")?;
        let handle = unsafe {
            ffi::nw_shim_proxy_config_create_oblivious_http(
                relay_hop.as_ptr(),
                relay_resource_path.as_ptr(),
                gateway_key_config.as_ptr(),
                gateway_key_config.len(),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create Oblivious HTTP proxy configuration".into(),
            ));
        }
        Ok(Self { handle })
    }

    /// Configure proxy authentication credentials.
    pub fn set_credentials(
        &mut self,
        username: &str,
        password: Option<&str>,
    ) -> Result<&mut Self, NetworkError> {
        let username = to_cstring(username, "username")?;
        let password = match password {
            Some(password) => Some(to_cstring(password, "password")?),
            None => None,
        };
        unsafe {
            ffi::nw_shim_proxy_config_set_username_password(
                self.handle,
                username.as_ptr(),
                password
                    .as_ref()
                    .map_or(core::ptr::null(), |value| value.as_ptr()),
            );
        }
        Ok(self)
    }

    /// Allow fallback to direct connections if the proxy path fails.
    pub fn set_failover_allowed(&mut self, failover_allowed: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_proxy_config_set_failover_allowed(
                self.handle,
                i32::from(failover_allowed),
            );
        }
        self
    }

    /// Whether proxy failover is currently enabled.
    #[must_use]
    pub fn failover_allowed(&self) -> bool {
        unsafe { ffi::nw_shim_proxy_config_get_failover_allowed(self.handle) != 0 }
    }

    /// Match a hostname suffix that should use this proxy.
    pub fn add_match_domain(&mut self, domain: &str) -> Result<&mut Self, NetworkError> {
        let domain = to_cstring(domain, "domain")?;
        unsafe { ffi::nw_shim_proxy_config_add_match_domain(self.handle, domain.as_ptr()) };
        Ok(self)
    }

    /// Clear hostname suffixes that opt into this proxy.
    pub fn clear_match_domains(&mut self) -> &mut Self {
        unsafe { ffi::nw_shim_proxy_config_clear_match_domains(self.handle) };
        self
    }

    /// Snapshot the currently configured match domains.
    #[must_use]
    pub fn match_domains(&self) -> Vec<String> {
        let mut domains = Vec::new();
        unsafe {
            ffi::nw_shim_proxy_config_enumerate_match_domains(
                self.handle,
                collect_string_trampoline,
                std::ptr::addr_of_mut!(domains).cast(),
            );
        };
        domains
    }

    /// Add a hostname suffix that should bypass this proxy.
    pub fn add_excluded_domain(&mut self, domain: &str) -> Result<&mut Self, NetworkError> {
        let domain = to_cstring(domain, "domain")?;
        unsafe { ffi::nw_shim_proxy_config_add_excluded_domain(self.handle, domain.as_ptr()) };
        Ok(self)
    }

    /// Clear hostname suffixes that bypass this proxy.
    pub fn clear_excluded_domains(&mut self) -> &mut Self {
        unsafe { ffi::nw_shim_proxy_config_clear_excluded_domains(self.handle) };
        self
    }

    /// Snapshot the currently configured excluded domains.
    #[must_use]
    pub fn excluded_domains(&self) -> Vec<String> {
        let mut domains = Vec::new();
        unsafe {
            ffi::nw_shim_proxy_config_enumerate_excluded_domains(
                self.handle,
                collect_string_trampoline,
                std::ptr::addr_of_mut!(domains).cast(),
            );
        };
        domains
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

impl UrlSessionConfiguration {
    /// Create a wrapper around `URLSessionConfiguration.default` when available.
    #[must_use]
    pub fn default_session() -> Option<Self> {
        let handle = unsafe { ffi::nw_shim_url_session_configuration_default() };
        (!handle.is_null()).then_some(Self { handle })
    }

    /// Create a wrapper around `URLSessionConfiguration.ephemeral` when available.
    #[must_use]
    pub fn ephemeral_session() -> Option<Self> {
        let handle = unsafe { ffi::nw_shim_url_session_configuration_ephemeral() };
        (!handle.is_null()).then_some(Self { handle })
    }

    /// Replace the configuration's `proxyConfigurations` array.
    pub fn set_proxy_configurations(&mut self, proxy_configurations: &[ProxyConfig]) -> &mut Self {
        let items: Vec<*mut c_void> = proxy_configurations
            .iter()
            .map(ProxyConfig::as_ptr)
            .collect();
        unsafe {
            ffi::nw_shim_url_session_configuration_set_proxy_configurations(
                self.handle,
                items.as_ptr(),
                items.len(),
            );
        };
        self
    }

    /// Copy the configuration's `proxyConfigurations` array.
    #[must_use]
    pub fn proxy_configurations(&self) -> Vec<ProxyConfig> {
        let mut count = 0_usize;
        let items = unsafe {
            ffi::nw_shim_url_session_configuration_copy_proxy_configurations(
                self.handle,
                &mut count,
            )
        };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let configs = slice
            .iter()
            .filter_map(|handle| (!handle.is_null()).then_some(ProxyConfig { handle: *handle }))
            .collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        configs
    }
}

impl Drop for UrlSessionConfiguration {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_url_session_configuration_release(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl Clone for ProxyConfig {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ProxyConfig {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
