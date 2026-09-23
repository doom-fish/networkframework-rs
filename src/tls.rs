#![allow(clippy::missing_errors_doc)]

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::sync::Arc;

use doom_fish_utils::panic_safe::catch_user_panic_result;
use zeroize::Zeroizing;

use crate::context::release_arc;
use crate::error::NetworkError;
use crate::ffi;
use crate::parameters::ConnectionParameters;
use crate::protocol::ProtocolOptions;
use crate::quic_support::{SecurityProtocolMetadata, SecurityProtocolOptions};

pub struct TlsIdentity {
    handle: *mut c_void,
}

unsafe impl Send for TlsIdentity {}
unsafe impl Sync for TlsIdentity {}

impl std::fmt::Debug for TlsIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsIdentity").finish_non_exhaustive()
    }
}

pub(crate) fn secret_c_string(
    value: &str,
    field: &str,
) -> Result<Zeroizing<Vec<u8>>, NetworkError> {
    if value.as_bytes().contains(&0) {
        return Err(NetworkError::InvalidArgument(format!(
            "{field} contains a NUL byte"
        )));
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(value.len() + 1));
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
    Ok(bytes)
}

impl TlsIdentity {
    pub fn from_pkcs12(data: &[u8], password: &str) -> Result<Self, NetworkError> {
        if data.is_empty() {
            return Err(NetworkError::InvalidArgument(
                "PKCS#12 data is empty".into(),
            ));
        }
        let password_bytes = secret_c_string(password, "PKCS#12 password")?;

        let mut status: c_int = ffi::NW_INVALID_ARG;
        let mut os_status: i32 = 0;
        let handle = unsafe {
            ffi::nw_shim_identity_create_from_pkcs12(
                data.as_ptr(),
                data.len(),
                password_bytes.as_ptr().cast::<c_char>(),
                &raw mut status,
                &raw mut os_status,
            )
        };
        if !handle.is_null() {
            return Ok(Self { handle });
        }
        Err(match status {
            ffi::NW_UNSUPPORTED => NetworkError::Unsupported(
                "importing a PKCS#12 identity without the keychain needs macOS 15 or later".into(),
            ),
            ffi::NW_SECURITY_FAILED => NetworkError::Security(os_status),
            _ => NetworkError::InvalidArgument("PKCS#12 data could not be read".into()),
        })
    }

    #[allow(clippy::missing_safety_doc)]
    #[must_use]
    pub unsafe fn from_sec_identity(sec_identity: *mut c_void) -> Option<Self> {
        let handle = unsafe { ffi::nw_shim_identity_create(sec_identity) };
        (!handle.is_null()).then_some(Self { handle })
    }
}

impl Clone for TlsIdentity {
    fn clone(&self) -> Self {
        Self {
            handle: unsafe { ffi::nw_shim_sec_retain(self.handle) },
        }
    }
}

impl Drop for TlsIdentity {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_sec_release(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl TlsVersion {
    const fn as_raw(self) -> u16 {
        match self {
            Self::Tls12 => 0x0303,
            Self::Tls13 => 0x0304,
        }
    }

    const fn from_raw(raw: u16) -> Option<Self> {
        match raw {
            0x0303 => Some(Self::Tls12),
            0x0304 => Some(Self::Tls13),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct TlsPeer<'a> {
    certificates: Vec<&'a [u8]>,
    trusted: bool,
}

impl<'a> TlsPeer<'a> {
    #[must_use]
    pub fn certificate_chain(&self) -> &[&'a [u8]] {
        &self.certificates
    }

    #[must_use]
    pub fn leaf_certificate(&self) -> Option<&'a [u8]> {
        self.certificates.first().copied()
    }

    #[must_use]
    pub fn leaf_certificate_sha256(&self) -> Option<[u8; 32]> {
        self.leaf_certificate().map(certificate_sha256)
    }

    #[must_use]
    pub const fn is_trusted_by_system(&self) -> bool {
        self.trusted
    }
}

#[must_use]
pub fn certificate_sha256(der: &[u8]) -> [u8; 32] {
    let mut digest = [0_u8; 32];
    unsafe { ffi::nw_shim_sha256(der.as_ptr(), der.len(), digest.as_mut_ptr()) };
    digest
}

type VerifyHandler = Box<dyn Fn(&TlsPeer<'_>) -> bool + Send + Sync + 'static>;

unsafe extern "C" fn verify_trampoline(
    certificates: *const *const u8,
    certificate_lengths: *const usize,
    certificate_count: usize,
    trusted: c_int,
    context: *mut c_void,
) -> c_int {
    if context.is_null() {
        return 0;
    }
    let handler = unsafe { &*context.cast::<VerifyHandler>() };
    let certificates =
        if certificates.is_null() || certificate_lengths.is_null() || certificate_count == 0 {
            Vec::new()
        } else {
            let pointers = unsafe { std::slice::from_raw_parts(certificates, certificate_count) };
            let lengths =
                unsafe { std::slice::from_raw_parts(certificate_lengths, certificate_count) };
            pointers
                .iter()
                .zip(lengths)
                .map(|(&pointer, &length)| {
                    if pointer.is_null() || length == 0 {
                        &[][..]
                    } else {
                        unsafe { std::slice::from_raw_parts(pointer, length) }
                    }
                })
                .collect()
        };
    let peer = TlsPeer {
        certificates,
        trusted: trusted != 0,
    };
    catch_user_panic_result("tls_verify_trampoline", || handler(&peer)).map_or(0, c_int::from)
}

fn to_cstring(value: &str, field: &str) -> Result<CString, NetworkError> {
    CString::new(value).map_err(|e| NetworkError::InvalidArgument(format!("{field} NUL byte: {e}")))
}

impl SecurityProtocolOptions {
    pub fn set_local_identity(&mut self, identity: &TlsIdentity) -> &mut Self {
        unsafe { ffi::nw_shim_sec_options_set_local_identity(self.as_ptr(), identity.handle) };
        self
    }

    pub fn set_min_tls_version(&mut self, version: TlsVersion) -> &mut Self {
        unsafe { ffi::nw_shim_sec_options_set_min_tls_version(self.as_ptr(), version.as_raw()) };
        self
    }

    pub fn set_max_tls_version(&mut self, version: TlsVersion) -> &mut Self {
        unsafe { ffi::nw_shim_sec_options_set_max_tls_version(self.as_ptr(), version.as_raw()) };
        self
    }

    pub fn add_application_protocol(&mut self, protocol: &str) -> Result<&mut Self, NetworkError> {
        if protocol.is_empty() || protocol.len() > 255 {
            return Err(NetworkError::InvalidArgument(
                "ALPN protocol names must be 1 to 255 bytes".into(),
            ));
        }
        let protocol = to_cstring(protocol, "application protocol")?;
        unsafe {
            ffi::nw_shim_sec_options_add_application_protocol(self.as_ptr(), protocol.as_ptr());
        };
        Ok(self)
    }

    pub fn set_server_name(&mut self, server_name: &str) -> Result<&mut Self, NetworkError> {
        if server_name.is_empty() {
            return Err(NetworkError::InvalidArgument("server name is empty".into()));
        }
        let server_name = to_cstring(server_name, "server name")?;
        unsafe { ffi::nw_shim_sec_options_set_server_name(self.as_ptr(), server_name.as_ptr()) };
        Ok(self)
    }

    pub fn set_peer_authentication_required(&mut self, required: bool) -> &mut Self {
        unsafe {
            ffi::nw_shim_sec_options_set_peer_authentication_required(
                self.as_ptr(),
                c_int::from(required),
            );
        };
        self
    }

    pub fn set_verify_handler<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(&TlsPeer<'_>) -> bool + Send + Sync + 'static,
    {
        let handler: Arc<VerifyHandler> = Arc::new(Box::new(handler));
        unsafe {
            ffi::nw_shim_sec_options_set_verify_callback(
                self.as_ptr(),
                Some(verify_trampoline),
                Arc::into_raw(handler).cast_mut().cast(),
                Some(release_arc::<VerifyHandler>),
            );
        };
        self
    }

    pub fn pin_peer_certificate_sha256(&mut self, pins: &[[u8; 32]]) -> &mut Self {
        let pins = pins.to_vec();
        self.set_verify_handler(move |peer| {
            peer.leaf_certificate_sha256()
                .is_some_and(|digest| pins.contains(&digest))
        })
    }
}

impl SecurityProtocolMetadata {
    #[must_use]
    pub fn negotiated_tls_version(&self) -> Option<TlsVersion> {
        TlsVersion::from_raw(unsafe {
            ffi::nw_shim_sec_metadata_get_negotiated_tls_version(self.as_ptr())
        })
    }

    #[must_use]
    pub fn negotiated_application_protocol(&self) -> Option<String> {
        let value = unsafe { ffi::nw_shim_sec_metadata_copy_negotiated_protocol(self.as_ptr()) };
        if value.is_null() {
            return None;
        }
        let protocol = unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::nw_shim_free_buffer(value.cast()) };
        Some(protocol)
    }
}

impl ConnectionParameters {
    pub fn tls_tcp_configured<F>(configure: F) -> Result<Self, NetworkError>
    where
        F: FnOnce(&mut SecurityProtocolOptions),
    {
        let tls = ProtocolOptions::tls()?;
        let mut security = tls.tls_security_options().ok_or_else(|| {
            NetworkError::InvalidArgument("TLS options carry no security options".into())
        })?;
        configure(&mut security);
        let mut parameters = Self::tcp()?;
        parameters.prepend_application_protocol(&tls)?;
        Ok(parameters)
    }

    pub fn quic_configured<F>(alpn: &str, configure: F) -> Result<Self, NetworkError>
    where
        F: FnOnce(&mut SecurityProtocolOptions),
    {
        let parameters = Self::quic(alpn)?;
        let transport = parameters
            .default_protocol_stack()
            .and_then(|stack| stack.transport_protocol())
            .filter(ProtocolOptions::is_quic)
            .ok_or_else(|| {
                NetworkError::InvalidArgument("QUIC parameters carry no QUIC transport".into())
            })?;
        let handle = unsafe { ffi::nw_shim_quic_copy_sec_protocol_options(transport.as_ptr()) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "QUIC options carry no security options".into(),
            ));
        }
        let mut security = unsafe { SecurityProtocolOptions::from_raw(handle) };
        configure(&mut security);
        Ok(parameters)
    }
}
