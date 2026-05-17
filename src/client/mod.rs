//! [`TcpClient`] — synchronous outbound TCP connection via Network.framework.

#![allow(clippy::missing_errors_doc)]

mod content_context;

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};

pub use content_context::{ContentContext, ReceivedContent};

use crate::error::{from_status, NetworkError};
use crate::ffi;
use crate::parameters::{ConnectionParameters, KeepAlives};
use crate::path::Path;
use crate::protocol::{ProtocolDefinition, ProtocolMetadata};

/// Blocking client wrapper around `nw_connection`.
///
/// The connection is fully established (`nw_connection_state_ready`)
/// before [`connect`] returns.
type BooleanCallback = Mutex<Box<dyn FnMut(bool) + Send + 'static>>;
type PathChangedCallback = Mutex<Box<dyn FnMut(Option<Path>) + Send + 'static>>;

pub struct TcpClient {
    handle: *mut c_void,
    _keepalives: KeepAlives,
    viability_callback: Option<Arc<BooleanCallback>>,
    better_path_callback: Option<Arc<BooleanCallback>>,
    path_callback: Option<Arc<PathChangedCallback>>,
}

// Network.framework manages its own thread-safety; the shim wraps
// each call in a dispatch queue. The handle is never observed from
// Rust except as a pointer.
unsafe impl Send for TcpClient {}
unsafe impl Sync for TcpClient {}

fn copied_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::nw_shim_free_buffer(ptr.cast()) };
    Some(value)
}

impl TcpClient {
    /// Open a plain TCP connection to `host:port`. Blocks up to 30 s
    /// waiting for the connection to become ready.
    ///
    /// For TLS, use [`connect_tls`](Self::connect_tls).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] / [`NetworkError::Timeout`]
    /// on failure.
    pub fn connect(host: &str, port: u16) -> Result<Self, NetworkError> {
        Self::connect_inner(host, port, false)
    }

    /// Open a TLS-wrapped TCP connection to `host:port`. Server-name
    /// indication and Apple's default trust evaluation are used; the
    /// connection only becomes ready once the TLS handshake completes
    /// successfully.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ConnectFailed`] / [`NetworkError::Timeout`]
    /// on TCP or TLS failure (incl. invalid certificate / hostname).
    pub fn connect_tls(host: &str, port: u16) -> Result<Self, NetworkError> {
        Self::connect_inner(host, port, true)
    }

    /// Open a TCP connection using explicit [`ConnectionParameters`].
    pub fn connect_with_parameters(
        host: &str,
        port: u16,
        parameters: &ConnectionParameters,
    ) -> Result<Self, NetworkError> {
        let host = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_connection_create_with_parameters(
                host.as_ptr(),
                port,
                parameters.as_ptr(),
                &mut status,
            )
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            _keepalives: parameters.keepalives(),
            viability_callback: None,
            better_path_callback: None,
            path_callback: None,
        })
    }

    fn connect_inner(host: &str, port: u16, use_tls: bool) -> Result<Self, NetworkError> {
        let host_c = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let mut status: c_int = 0;
        let handle = unsafe {
            ffi::nw_shim_tcp_connect(host_c.as_ptr(), port, c_int::from(use_tls), &mut status)
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            _keepalives: KeepAlives::empty(),
            viability_callback: None,
            better_path_callback: None,
            path_callback: None,
        })
    }

    /// Wrap a raw `nw_conn_handle*` (produced by the listener shim).
    ///
    /// # Safety
    ///
    /// `handle` must be a live pointer returned by the shim's accept
    /// path. Ownership is transferred to the returned [`TcpClient`].
    #[must_use]
    pub(crate) const unsafe fn from_raw_with_keepalives(
        handle: *mut c_void,
        keepalives: KeepAlives,
    ) -> Self {
        Self {
            handle,
            _keepalives: keepalives,
            viability_callback: None,
            better_path_callback: None,
            path_callback: None,
        }
    }

    /// Copy the remote endpoint of the connection.
    #[must_use]
    pub fn endpoint(&self) -> Option<crate::endpoint::Endpoint> {
        let handle = unsafe { ffi::nw_shim_connection_copy_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { crate::endpoint::Endpoint::from_raw(handle) })
    }

    /// Copy the connection's parameters snapshot.
    #[must_use]
    pub fn parameters(&self) -> Option<ConnectionParameters> {
        let handle = unsafe { ffi::nw_shim_connection_copy_parameters(self.handle) };
        (!handle.is_null()).then_some(unsafe { ConnectionParameters::from_raw(handle) })
    }

    /// Copy the connection's current network path, if available.
    #[must_use]
    pub fn current_path(&self) -> Option<crate::path::Path> {
        let handle = unsafe { ffi::nw_shim_connection_copy_current_path(self.handle) };
        (!handle.is_null()).then_some(unsafe { crate::path::Path::from_raw(handle) })
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    /// Restart the connection's path and protocol selection.
    pub fn restart(&self) {
        unsafe { ffi::nw_shim_connection_restart(self.handle) };
    }

    /// Force immediate cancellation without graceful teardown.
    pub fn force_cancel(&self) {
        unsafe { ffi::nw_shim_connection_force_cancel(self.handle) };
    }

    /// Cancel the current endpoint and force path fallback when possible.
    pub fn cancel_current_endpoint(&self) {
        unsafe { ffi::nw_shim_connection_cancel_current_endpoint(self.handle) };
    }

    /// Execute several operations in a single Network.framework batch.
    pub fn batch<F>(&self, mut callback: F)
    where
        F: FnMut(),
    {
        unsafe extern "C" fn invoke(user_info: *mut c_void) {
            if user_info.is_null() {
                return;
            }
            let callback = unsafe { &mut *user_info.cast::<&mut dyn FnMut()>() };
            callback();
        }

        let mut callback_ref: &mut dyn FnMut() = &mut callback;
        unsafe {
            ffi::nw_shim_connection_batch(
                self.handle,
                Some(invoke),
                std::ptr::addr_of_mut!(callback_ref).cast(),
            );
        };
    }

    /// Human-readable description from Network.framework.
    #[must_use]
    pub fn description(&self) -> Option<String> {
        copied_string(unsafe { ffi::nw_shim_connection_copy_description(self.handle) })
    }

    /// Maximum datagram size currently allowed by the transport.
    #[must_use]
    pub fn maximum_datagram_size(&self) -> u32 {
        unsafe { ffi::nw_shim_connection_get_maximum_datagram_size(self.handle) }
    }

    /// Copy protocol metadata associated with the connection for a specific protocol definition.
    #[must_use]
    pub fn protocol_metadata(&self, definition: &ProtocolDefinition) -> Option<ProtocolMetadata> {
        let handle = unsafe { ffi::nw_shim_connection_copy_protocol_metadata(self.handle, definition.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { ProtocolMetadata::from_raw(handle) })
    }

    /// Receive viability updates for the connection.
    pub fn set_viability_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        let callback: Box<dyn FnMut(bool) + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_connection_set_viability_changed_handler(
                self.handle,
                Some(boolean_trampoline),
                raw,
            );
        };
        self.viability_callback = Some(arc);
    }

    /// Receive updates when a better network path becomes available.
    pub fn set_better_path_available_handler<F>(&mut self, callback: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        let callback: Box<dyn FnMut(bool) + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_connection_set_better_path_available_handler(
                self.handle,
                Some(boolean_trampoline),
                raw,
            );
        };
        self.better_path_callback = Some(arc);
    }

    /// Receive path snapshots whenever Network.framework changes the active path.
    pub fn set_path_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(Option<Path>) + Send + 'static,
    {
        let callback: Box<dyn FnMut(Option<Path>) + Send + 'static> = Box::new(callback);
        let arc = Arc::new(Mutex::new(callback));
        let raw = Arc::into_raw(arc.clone()).cast::<c_void>().cast_mut();
        unsafe {
            ffi::nw_shim_connection_set_path_changed_handler(
                self.handle,
                Some(path_trampoline),
                raw,
            );
        };
        self.path_callback = Some(arc);
    }

    /// Send `data` over the connection. Blocks until the framework has
    /// acknowledged the buffer.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send(&self, data: &[u8]) -> Result<(), NetworkError> {
        let status = unsafe { ffi::nw_shim_tcp_send(self.handle, data.as_ptr(), data.len()) };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Send `data` with an explicit [`ContentContext`].
    pub fn send_with_context(
        &self,
        data: &[u8],
        context: &ContentContext,
    ) -> Result<(), NetworkError> {
        let status = unsafe {
            ffi::nw_shim_connection_send_with_context(
                self.handle,
                data.as_ptr(),
                data.len(),
                context.as_ptr(),
            )
        };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Read up to `max_len` bytes from the connection. Blocks until at
    /// least one byte is available (or the connection ends).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::ReceiveFailed`].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn receive(&self, max_len: usize) -> Result<Vec<u8>, NetworkError> {
        let mut buf = vec![0u8; max_len];
        let n = unsafe { ffi::nw_shim_tcp_receive(self.handle, buf.as_mut_ptr(), max_len) };
        if n < 0 {
            return Err(from_status(n as i32));
        }
        buf.truncate(n as usize);
        Ok(buf)
    }

    /// Receive data together with its [`ContentContext`].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn receive_with_context(&self, max_len: usize) -> Result<ReceivedContent, NetworkError> {
        let mut buf = vec![0_u8; max_len];
        let mut context = core::ptr::null_mut();
        let mut is_complete = 0;
        let n = unsafe {
            ffi::nw_shim_connection_receive_with_context(
                self.handle,
                buf.as_mut_ptr(),
                max_len,
                &mut context,
                &mut is_complete,
            )
        };
        if n < 0 {
            return Err(from_status(n as i32));
        }
        buf.truncate(n as usize);
        Ok(ReceivedContent {
            data: buf,
            context: (!context.is_null()).then_some(unsafe { ContentContext::from_raw(context) }),
            is_complete: is_complete != 0,
        })
    }
}

unsafe extern "C" fn boolean_trampoline(value: c_int, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<BooleanCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    guard(value != 0);
}

unsafe extern "C" fn path_trampoline(path: *mut c_void, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }
    let callback = unsafe { &*user_info.cast::<PathChangedCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    let path = (!path.is_null()).then_some(unsafe { Path::from_raw(path) });
    guard(path);
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_tcp_close(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}
