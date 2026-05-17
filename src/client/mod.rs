//! [`TcpClient`] — synchronous outbound TCP connection via Network.framework.

#![allow(clippy::missing_errors_doc)]

mod content_context;

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};

use doom_fish_utils::panic_safe::catch_user_panic;

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
    viability_raw: *const BooleanCallback,
    better_path_raw: *const BooleanCallback,
    path_raw: *const PathChangedCallback,
}

// SAFETY: Network.framework serializes connection callbacks on its own queue,
// and the raw callback pointers are only dereferenced by that queue while the
// connection handle is live. `Drop` closes the connection, waits for the queue
// to drain, and only then reclaims the raw callback pointers.
unsafe impl Send for TcpClient {}
// SAFETY: Shared references only forward to the shim. The raw callback
// pointers remain valid until `Drop` closes the connection and reclaims them
// after the serial queue has finished running callbacks.
unsafe impl Sync for TcpClient {}

fn reclaim_arc_raw<T>(raw: &mut *const T) {
    if !raw.is_null() {
        // SAFETY: `*raw` was produced by `Arc::into_raw`, and the caller only
        // invokes this helper after the native side has stopped using it.
        unsafe {
            drop(Arc::from_raw(*raw));
        }
        *raw = ptr::null();
    }
}

fn copied_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: `ptr` is a valid NUL-terminated buffer returned by the shim and
    // remains alive until we free it with `nw_shim_free_buffer` below.
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: `ptr` came from the shim's string-allocation helper and must be
    // released with `nw_shim_free_buffer` exactly once.
    unsafe {
        ffi::nw_shim_free_buffer(ptr.cast());
    }
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
        // SAFETY: `host`, `parameters`, and `status` all outlive the call, and
        // `parameters.as_ptr()` is a live shim-owned parameters handle.
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
            viability_raw: ptr::null(),
            better_path_raw: ptr::null(),
            path_raw: ptr::null(),
        })
    }

    fn connect_inner(host: &str, port: u16, use_tls: bool) -> Result<Self, NetworkError> {
        let host_c = CString::new(host)
            .map_err(|e| NetworkError::InvalidArgument(format!("host NUL byte: {e}")))?;
        let mut status: c_int = 0;
        // SAFETY: `host_c` and `status` outlive the call, and the boolean flag
        // is represented exactly as the shim expects.
        let handle = unsafe {
            ffi::nw_shim_tcp_connect(host_c.as_ptr(), port, c_int::from(use_tls), &mut status)
        };
        if status != ffi::NW_OK || handle.is_null() {
            return Err(from_status(status));
        }
        Ok(Self {
            handle,
            _keepalives: KeepAlives::empty(),
            viability_raw: ptr::null(),
            better_path_raw: ptr::null(),
            path_raw: ptr::null(),
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
            viability_raw: ptr::null(),
            better_path_raw: ptr::null(),
            path_raw: ptr::null(),
        }
    }

    /// Copy the remote endpoint of the connection.
    #[must_use]
    pub fn endpoint(&self) -> Option<crate::endpoint::Endpoint> {
        // SAFETY: `self.handle` is either null (the shim returns null) or a
        // live connection handle produced by the shim.
        let handle = unsafe { ffi::nw_shim_connection_copy_endpoint(self.handle) };
        if handle.is_null() {
            None
        } else {
            // SAFETY: the shim returns a retained endpoint handle for the
            // caller to wrap and own.
            Some(unsafe { crate::endpoint::Endpoint::from_raw(handle) })
        }
    }

    /// Copy the connection's parameters snapshot.
    #[must_use]
    pub fn parameters(&self) -> Option<ConnectionParameters> {
        // SAFETY: `self.handle` is either null (the shim returns null) or a
        // live connection handle produced by the shim.
        let handle = unsafe { ffi::nw_shim_connection_copy_parameters(self.handle) };
        if handle.is_null() {
            None
        } else {
            // SAFETY: the shim returns a retained parameters handle for the
            // caller to wrap and own.
            Some(unsafe { ConnectionParameters::from_raw(handle) })
        }
    }

    /// Copy the connection's current network path, if available.
    #[must_use]
    pub fn current_path(&self) -> Option<crate::path::Path> {
        // SAFETY: `self.handle` is either null (the shim returns null) or a
        // live connection handle produced by the shim.
        let handle = unsafe { ffi::nw_shim_connection_copy_current_path(self.handle) };
        if handle.is_null() {
            None
        } else {
            // SAFETY: the shim returns a retained path snapshot for the caller
            // to wrap and own.
            Some(unsafe { crate::path::Path::from_raw(handle) })
        }
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    fn clear_viability_changed_handler(&mut self) {
        if !self.viability_raw.is_null() {
            if !self.handle.is_null() {
                // SAFETY: `self.handle` is a live connection handle. Clearing
                // the handler and draining the serial queue ensures no queued
                // viability callback can still observe `self.viability_raw`.
                unsafe {
                    ffi::nw_shim_connection_set_viability_changed_handler(
                        self.handle,
                        None,
                        ptr::null_mut(),
                    );
                    ffi::nw_shim_connection_drain_queue(self.handle);
                }
            }
            reclaim_arc_raw(&mut self.viability_raw);
        }
    }

    fn clear_better_path_available_handler(&mut self) {
        if !self.better_path_raw.is_null() {
            if !self.handle.is_null() {
                // SAFETY: `self.handle` is a live connection handle. Clearing
                // the handler and draining the serial queue ensures no queued
                // better-path callback can still observe `self.better_path_raw`.
                unsafe {
                    ffi::nw_shim_connection_set_better_path_available_handler(
                        self.handle,
                        None,
                        ptr::null_mut(),
                    );
                    ffi::nw_shim_connection_drain_queue(self.handle);
                }
            }
            reclaim_arc_raw(&mut self.better_path_raw);
        }
    }

    fn clear_path_changed_handler(&mut self) {
        if !self.path_raw.is_null() {
            if !self.handle.is_null() {
                // SAFETY: `self.handle` is a live connection handle. Clearing
                // the handler and draining the serial queue ensures no queued
                // path callback can still observe `self.path_raw`.
                unsafe {
                    ffi::nw_shim_connection_set_path_changed_handler(
                        self.handle,
                        None,
                        ptr::null_mut(),
                    );
                    ffi::nw_shim_connection_drain_queue(self.handle);
                }
            }
            reclaim_arc_raw(&mut self.path_raw);
        }
    }

    /// Restart the connection's path and protocol selection.
    pub fn restart(&self) {
        // SAFETY: `self.handle` is the live connection handle owned by this
        // client, and the shim forwards the request without retaining pointers.
        unsafe {
            ffi::nw_shim_connection_restart(self.handle);
        }
    }

    /// Force immediate cancellation without graceful teardown.
    pub fn force_cancel(&self) {
        // SAFETY: `self.handle` is the live connection handle owned by this
        // client, and the shim forwards the request without retaining pointers.
        unsafe {
            ffi::nw_shim_connection_force_cancel(self.handle);
        }
    }

    /// Cancel the current endpoint and force path fallback when possible.
    pub fn cancel_current_endpoint(&self) {
        // SAFETY: `self.handle` is the live connection handle owned by this
        // client, and the shim forwards the request without retaining pointers.
        unsafe {
            ffi::nw_shim_connection_cancel_current_endpoint(self.handle);
        }
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
            // SAFETY: `user_info` is the address of `callback_ref` passed to
            // `nw_shim_connection_batch`, and the shim invokes the callback
            // synchronously before returning.
            let callback = unsafe { &mut *user_info.cast::<&mut dyn FnMut()>() };
            catch_user_panic("tcp_client_batch_invoke", callback);
        }

        let mut callback_ref: &mut dyn FnMut() = &mut callback;
        // SAFETY: `self.handle` is a live connection handle, and `callback_ref`
        // lives until `nw_shim_connection_batch` returns. The shim does not
        // retain `user_info` beyond that call.
        unsafe {
            ffi::nw_shim_connection_batch(
                self.handle,
                Some(invoke),
                ptr::addr_of_mut!(callback_ref).cast(),
            );
        }
    }

    /// Human-readable description from Network.framework.
    #[must_use]
    pub fn description(&self) -> Option<String> {
        // SAFETY: `self.handle` is either null (the shim returns null) or a
        // live connection handle produced by the shim.
        let description = unsafe { ffi::nw_shim_connection_copy_description(self.handle) };
        copied_string(description)
    }

    /// Maximum datagram size currently allowed by the transport.
    #[must_use]
    pub fn maximum_datagram_size(&self) -> u32 {
        // SAFETY: `self.handle` is either null (the shim returns 0) or a live
        // connection handle produced by the shim.
        unsafe { ffi::nw_shim_connection_get_maximum_datagram_size(self.handle) }
    }

    /// Copy protocol metadata associated with the connection for a specific protocol definition.
    #[must_use]
    pub fn protocol_metadata(&self, definition: &ProtocolDefinition) -> Option<ProtocolMetadata> {
        // SAFETY: `self.handle` and `definition.as_ptr()` are live shim handles
        // for the duration of the call.
        let handle = unsafe {
            ffi::nw_shim_connection_copy_protocol_metadata(self.handle, definition.as_ptr())
        };
        if handle.is_null() {
            None
        } else {
            // SAFETY: the shim returns a retained protocol-metadata handle for
            // the caller to wrap and own.
            Some(unsafe { ProtocolMetadata::from_raw(handle) })
        }
    }

    /// Receive viability updates for the connection.
    pub fn set_viability_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        self.clear_viability_changed_handler();

        let callback: Box<dyn FnMut(bool) + Send + 'static> = Box::new(callback);
        let viability_raw = Arc::into_raw(Arc::new(Mutex::new(callback)));
        if self.handle.is_null() {
            self.viability_raw = viability_raw;
            return;
        }

        // SAFETY: `self.handle` is a live connection handle, and
        // `viability_raw` points at an `Arc` allocation that stays valid until
        // we clear the handler and reclaim it.
        unsafe {
            ffi::nw_shim_connection_set_viability_changed_handler(
                self.handle,
                Some(boolean_trampoline),
                viability_raw.cast::<c_void>().cast_mut(),
            );
        }
        self.viability_raw = viability_raw;
    }

    /// Receive updates when a better network path becomes available.
    pub fn set_better_path_available_handler<F>(&mut self, callback: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        self.clear_better_path_available_handler();

        let callback: Box<dyn FnMut(bool) + Send + 'static> = Box::new(callback);
        let better_path_raw = Arc::into_raw(Arc::new(Mutex::new(callback)));
        if self.handle.is_null() {
            self.better_path_raw = better_path_raw;
            return;
        }

        // SAFETY: `self.handle` is a live connection handle, and
        // `better_path_raw` points at an `Arc` allocation that stays valid
        // until we clear the handler and reclaim it.
        unsafe {
            ffi::nw_shim_connection_set_better_path_available_handler(
                self.handle,
                Some(boolean_trampoline),
                better_path_raw.cast::<c_void>().cast_mut(),
            );
        }
        self.better_path_raw = better_path_raw;
    }

    /// Receive path snapshots whenever Network.framework changes the active path.
    pub fn set_path_changed_handler<F>(&mut self, callback: F)
    where
        F: FnMut(Option<Path>) + Send + 'static,
    {
        self.clear_path_changed_handler();

        let callback: Box<dyn FnMut(Option<Path>) + Send + 'static> = Box::new(callback);
        let path_raw = Arc::into_raw(Arc::new(Mutex::new(callback)));
        if self.handle.is_null() {
            self.path_raw = path_raw;
            return;
        }

        // SAFETY: `self.handle` is a live connection handle, and `path_raw`
        // points at an `Arc` allocation that stays valid until we clear the
        // handler and reclaim it.
        unsafe {
            ffi::nw_shim_connection_set_path_changed_handler(
                self.handle,
                Some(path_trampoline),
                path_raw.cast::<c_void>().cast_mut(),
            );
        }
        self.path_raw = path_raw;
    }

    /// Send `data` over the connection. Blocks until the framework has
    /// acknowledged the buffer.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::SendFailed`].
    pub fn send(&self, data: &[u8]) -> Result<(), NetworkError> {
        // SAFETY: `self.handle` is a live connection handle, and `data`
        // remains valid for the duration of the blocking shim call.
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
        // SAFETY: `self.handle` and `context.as_ptr()` are live shim handles,
        // and `data` remains valid for the duration of the blocking shim call.
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
        // SAFETY: `self.handle` is a live connection handle, and `buf`
        // provides writable storage for the duration of the blocking shim call.
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
        let mut context = ptr::null_mut();
        let mut is_complete = 0;
        // SAFETY: `self.handle` is a live connection handle, and `buf`,
        // `context`, and `is_complete` all provide valid out-pointers for the
        // duration of the blocking shim call.
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
        let context = if context.is_null() {
            None
        } else {
            // SAFETY: the shim returned a retained content-context handle for
            // the caller to wrap and own.
            Some(unsafe { ContentContext::from_raw(context) })
        };
        Ok(ReceivedContent {
            data: buf,
            context,
            is_complete: is_complete != 0,
        })
    }
}

unsafe extern "C" fn boolean_trampoline(value: c_int, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }

    // SAFETY: `user_info` is the stable pointer created by `Arc::into_raw` in
    // the setter and remains valid until the handler is cleared and reclaimed.
    let callback = unsafe { &*user_info.cast::<BooleanCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    catch_user_panic("tcp_client_boolean_trampoline", || guard(value != 0));
}

unsafe extern "C" fn path_trampoline(path: *mut c_void, user_info: *mut c_void) {
    if user_info.is_null() {
        return;
    }

    // SAFETY: `user_info` is the stable pointer created by `Arc::into_raw` in
    // the setter and remains valid until the handler is cleared and reclaimed.
    let callback = unsafe { &*user_info.cast::<PathChangedCallback>() };
    let Ok(mut guard) = callback.lock() else {
        return;
    };
    let path = if path.is_null() {
        None
    } else {
        // SAFETY: the shim passes a retained path snapshot to the callback, and
        // ownership transfers to this wrapper value.
        Some(unsafe { Path::from_raw(path) })
    };
    catch_user_panic("tcp_client_path_trampoline", || guard(path));
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: `self.handle` is the live connection handle owned by this
            // client. `nw_shim_tcp_close` waits for the serial queue to reach
            // the cancelled state before returning, so no more callbacks can
            // fire after this call completes.
            unsafe {
                ffi::nw_shim_tcp_close(self.handle);
            }
            self.handle = ptr::null_mut();
        }
        reclaim_arc_raw(&mut self.viability_raw);
        reclaim_arc_raw(&mut self.better_path_raw);
        reclaim_arc_raw(&mut self.path_raw);
    }
}
