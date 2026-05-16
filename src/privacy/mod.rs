//! Privacy contexts, proxy configuration, and encrypted resolver settings.

#![allow(clippy::missing_errors_doc)]

use core::ffi::c_void;
use std::ffi::CString;

use crate::error::NetworkError;
use crate::ffi;

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value)
        .map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

/// Shared privacy and cache policy applied through [`crate::ConnectionParameters`].
pub struct PrivacyContext {
    handle: *mut c_void,
}

unsafe impl Send for PrivacyContext {}
unsafe impl Sync for PrivacyContext {}

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
    pub fn add_server_address(&mut self, address: &str, port: u16) -> Result<&mut Self, NetworkError> {
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

/// Proxy settings attachable to a [`PrivacyContext`].
pub struct ProxyConfig {
    handle: *mut c_void,
}

unsafe impl Send for ProxyConfig {}
unsafe impl Sync for ProxyConfig {}

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
                password.as_ref().map_or(core::ptr::null(), |value| value.as_ptr()),
            );
        }
        Ok(self)
    }

    /// Allow fallback to direct connections if the proxy path fails.
    pub fn set_failover_allowed(&mut self, failover_allowed: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_proxy_config_set_failover_allowed(self.handle, i32::from(failover_allowed));
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
