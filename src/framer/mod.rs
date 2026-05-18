//! Custom protocol framers built on `nw_framer_*`.

#![allow(
    clippy::missing_errors_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::use_self
)]

use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::marker::PhantomData;
use std::slice;
use std::sync::Arc;
use std::time::Duration;

use crate::endpoint::Endpoint;
use crate::error::NetworkError;
use crate::ffi;
use crate::parameters::ConnectionParameters;
use crate::protocol::ProtocolOptions;

/// Result of [`Framer::on_start`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramerStart {
    Ready,
    WillMarkReady,
}

/// Trait implemented by per-connection framer instances.
pub trait Framer: Send {
    /// Called when a new framer instance starts.
    fn on_start(&mut self, context: &mut FramerContext) -> FramerStart;

    /// Called when new inbound bytes are available.
    ///
    /// Return a hint describing how many bytes should be available before the
    /// input callback is invoked again.
    fn on_input(&mut self, context: &mut FramerContext) -> usize;

    /// Called when a new outbound message is ready to be framed.
    fn on_output(
        &mut self,
        context: &mut FramerContext,
        message: Option<FramerMessageView<'_>>,
        message_length: usize,
        is_complete: bool,
    );

    /// Called when the connection is stopping.
    fn on_stop(&mut self, context: &mut FramerContext) -> bool;

    /// Called when a scheduled wakeup fires.
    fn on_wakeup(&mut self, _context: &mut FramerContext) {}

    /// Called when the protocol stack is being torn down.
    fn on_cleanup(&mut self, _context: &mut FramerContext) {}
}

type FactoryFn = dyn Fn() -> Box<dyn Framer> + Send + Sync + 'static;

pub(crate) struct FramerCallbacksOwner {
    factory: Box<FactoryFn>,
}

/// A retained custom framer protocol definition.
pub struct FramerDefinition {
    handle: *mut c_void,
    keepalive: Arc<FramerCallbacksOwner>,
}

unsafe impl Send for FramerDefinition {}
unsafe impl Sync for FramerDefinition {}

impl std::fmt::Debug for FramerDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramerDefinition")
            .field("handle", &self.handle)
            .field("keepalive_refs", &Arc::strong_count(&self.keepalive))
            .finish_non_exhaustive()
    }
}

/// Framer options attachable to [`crate::ConnectionParameters`].
pub struct FramerOptions {
    handle: *mut c_void,
    keepalive: Arc<FramerCallbacksOwner>,
}

unsafe impl Send for FramerOptions {}
unsafe impl Sync for FramerOptions {}

impl std::fmt::Debug for FramerOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramerOptions")
            .field("handle", &self.handle)
            .field("keepalive_refs", &Arc::strong_count(&self.keepalive))
            .finish_non_exhaustive()
    }
}

/// Owned framer metadata.
pub struct FramerMessage {
    handle: *mut c_void,
}

unsafe impl Send for FramerMessage {}
unsafe impl Sync for FramerMessage {}

impl std::fmt::Debug for FramerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramerMessage")
            .field("handle", &self.handle)
            .finish()
    }
}

/// Borrowed framer metadata passed to [`Framer::on_output`].
pub struct FramerMessageView<'a> {
    handle: *mut c_void,
    _marker: PhantomData<&'a ()>,
}

impl std::fmt::Debug for FramerMessageView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramerMessageView")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

struct AsyncCallbackHolder(Box<dyn FnMut(&mut FramerContext) + Send + 'static>);

impl FramerDefinition {
    /// Create a new custom framer definition.
    pub fn new<F, T>(identifier: &str, factory: F) -> Result<Self, NetworkError>
    where
        F: Fn() -> T + Send + Sync + 'static,
        T: Framer + 'static,
    {
        let identifier = CString::new(identifier)
            .map_err(|e| NetworkError::InvalidArgument(format!("identifier NUL byte: {e}")))?;
        let keepalive = Arc::new(FramerCallbacksOwner {
            factory: Box::new(move || Box::new(factory()) as Box<dyn Framer>),
        });
        let handle = unsafe {
            ffi::nw_shim_framer_definition_create(
                identifier.as_ptr(),
                0,
                create_instance_trampoline,
                drop_instance_trampoline,
                start_trampoline,
                input_trampoline,
                output_trampoline,
                Some(wakeup_trampoline),
                Some(stop_trampoline),
                Some(cleanup_trampoline),
                Arc::as_ptr(&keepalive).cast_mut().cast::<c_void>(),
            )
        };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create framer definition".into(),
            ));
        }
        Ok(Self { handle, keepalive })
    }

    /// Create options from this definition for use on a protocol stack.
    pub fn options(&self) -> Result<FramerOptions, NetworkError> {
        let handle = unsafe { ffi::nw_shim_framer_create_options(self.handle) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create framer options".into(),
            ));
        }
        Ok(FramerOptions {
            handle,
            keepalive: self.keepalive.clone(),
        })
    }
}

impl Drop for FramerDefinition {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl FramerOptions {
    /// Create outbound framer metadata for a message.
    pub fn create_message(&self) -> Result<FramerMessage, NetworkError> {
        let handle = unsafe { ffi::nw_shim_framer_message_create_from_options(self.handle) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create framer message".into(),
            ));
        }
        Ok(FramerMessage { handle })
    }

    /// Store an opaque object handle on the framer options.
    ///
    /// # Safety
    ///
    /// `value` must be either null or a valid Objective-C/Network.framework
    /// object handle that remains valid for the duration expected by the framer.
    pub unsafe fn set_object_value_handle(
        &mut self,
        key: &str,
        value: *mut c_void,
    ) -> Result<&mut Self, NetworkError> {
        let key = CString::new(key)
            .map_err(|e| NetworkError::InvalidArgument(format!("key NUL byte: {e}")))?;
        unsafe { ffi::nw_shim_framer_options_set_object_value(self.handle, key.as_ptr(), value) };
        Ok(self)
    }

    /// Copy an opaque object handle from the framer options.
    pub fn copy_object_value_handle(&self, key: &str) -> Result<*mut c_void, NetworkError> {
        let key = CString::new(key)
            .map_err(|e| NetworkError::InvalidArgument(format!("key NUL byte: {e}")))?;
        Ok(unsafe { ffi::nw_shim_framer_options_copy_object_value(self.handle, key.as_ptr()) })
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) fn keepalive(&self) -> Arc<FramerCallbacksOwner> {
        self.keepalive.clone()
    }
}

impl Drop for FramerOptions {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl FramerMessage {
    /// Set an integer value on the message metadata.
    pub fn set_u64(&mut self, key: &str, value: u64) -> Result<&mut Self, NetworkError> {
        let key = CString::new(key)
            .map_err(|e| NetworkError::InvalidArgument(format!("key NUL byte: {e}")))?;
        let status =
            unsafe { ffi::nw_shim_framer_message_set_u64(self.handle, key.as_ptr(), value) };
        if status != ffi::NW_OK {
            return Err(crate::error::from_status(status));
        }
        Ok(self)
    }

    /// Get an integer value from the message metadata.
    #[must_use]
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        FramerMessageView {
            handle: self.handle,
            _marker: PhantomData,
        }
        .get_u64(key)
    }

    /// Store an opaque object handle on the message metadata.
    ///
    /// # Safety
    ///
    /// `value` must be either null or a valid Objective-C/Network.framework
    /// object handle that remains valid for the duration expected by the framer.
    pub unsafe fn set_object_value_handle(
        &mut self,
        key: &str,
        value: *mut c_void,
    ) -> Result<&mut Self, NetworkError> {
        let key = CString::new(key)
            .map_err(|e| NetworkError::InvalidArgument(format!("key NUL byte: {e}")))?;
        unsafe { ffi::nw_shim_framer_message_set_object_value(self.handle, key.as_ptr(), value) };
        Ok(self)
    }

    /// Copy an opaque object handle from the message metadata.
    pub fn copy_object_value_handle(&self, key: &str) -> Result<*mut c_void, NetworkError> {
        let key = CString::new(key)
            .map_err(|e| NetworkError::InvalidArgument(format!("key NUL byte: {e}")))?;
        Ok(unsafe { ffi::nw_shim_framer_message_copy_object_value(self.handle, key.as_ptr()) })
    }

    #[must_use]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.handle
    }

    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Clone for FramerMessage {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for FramerMessage {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl FramerMessageView<'_> {
    /// Get an integer value from the message metadata.
    #[must_use]
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        let key = CString::new(key).ok()?;
        let mut value = 0_u64;
        let found =
            unsafe { ffi::nw_shim_framer_message_get_u64(self.handle, key.as_ptr(), &mut value) };
        if found > 0 {
            Some(value)
        } else {
            None
        }
    }
}

/// Context passed into framer callbacks.
pub struct FramerContext {
    handle: *mut c_void,
}

impl std::fmt::Debug for FramerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramerContext")
            .field("handle", &self.handle)
            .finish()
    }
}

impl FramerContext {
    #[must_use]
    const fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Create framer metadata for an inbound message.
    pub fn create_message(&mut self) -> Result<FramerMessage, NetworkError> {
        let handle = unsafe { ffi::nw_shim_framer_message_create_for_instance(self.handle) };
        if handle.is_null() {
            return Err(NetworkError::InvalidArgument(
                "failed to create inbound framer message".into(),
            ));
        }
        Ok(FramerMessage { handle })
    }

    /// Copy the current remote endpoint for the framer.
    #[must_use]
    pub fn remote_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_framer_copy_remote_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Copy the current local endpoint for the framer.
    #[must_use]
    pub fn local_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_framer_copy_local_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Copy the current connection parameters for the framer.
    #[must_use]
    pub fn parameters(&self) -> Option<ConnectionParameters> {
        let handle = unsafe { ffi::nw_shim_framer_copy_parameters(self.handle) };
        (!handle.is_null()).then_some(unsafe { ConnectionParameters::from_raw(handle) })
    }

    /// Copy the protocol options associated with the framer.
    #[must_use]
    pub fn options(&self) -> Option<ProtocolOptions> {
        let handle = unsafe { ffi::nw_shim_framer_copy_options(self.handle) };
        (!handle.is_null()).then_some(unsafe { ProtocolOptions::from_raw(handle) })
    }

    /// Parse available input bytes.
    pub fn parse_input<F>(
        &mut self,
        minimum_incomplete_length: usize,
        maximum_length: usize,
        temp_buffer: Option<&mut [u8]>,
        mut parse: F,
    ) -> bool
    where
        F: FnMut(&[u8], bool) -> usize,
    {
        let (temp_buffer, max_length) = temp_buffer
            .map_or((core::ptr::null_mut(), maximum_length), |buffer| {
                (buffer.as_mut_ptr(), buffer.len())
            });
        unsafe {
            ffi::nw_shim_framer_parse_input(
                self.handle,
                minimum_incomplete_length,
                max_length,
                temp_buffer,
                parse_trampoline::<F>,
                std::ptr::addr_of_mut!(parse).cast(),
            ) != 0
        }
    }

    /// Parse available output bytes.
    pub fn parse_output<F>(
        &mut self,
        minimum_incomplete_length: usize,
        maximum_length: usize,
        temp_buffer: Option<&mut [u8]>,
        mut parse: F,
    ) -> bool
    where
        F: FnMut(&[u8], bool) -> usize,
    {
        let (temp_buffer, max_length) = temp_buffer
            .map_or((core::ptr::null_mut(), maximum_length), |buffer| {
                (buffer.as_mut_ptr(), buffer.len())
            });
        unsafe {
            ffi::nw_shim_framer_parse_output(
                self.handle,
                minimum_incomplete_length,
                max_length,
                temp_buffer,
                parse_trampoline::<F>,
                std::ptr::addr_of_mut!(parse).cast(),
            ) != 0
        }
    }

    /// Deliver bytes from the current input cursor without copying.
    pub fn pass_input_data(
        &mut self,
        input_length: usize,
        message: Option<&FramerMessage>,
        is_complete: bool,
    ) -> bool {
        unsafe {
            ffi::nw_shim_framer_pass_input_data(
                self.handle,
                input_length,
                message.map_or(core::ptr::null_mut(), FramerMessage::as_ptr),
                i32::from(is_complete),
            ) != 0
        }
    }

    /// Deliver transformed input bytes to the application.
    pub fn deliver_input_data(
        &mut self,
        input_buffer: &[u8],
        message: Option<&FramerMessage>,
        is_complete: bool,
    ) {
        unsafe {
            ffi::nw_shim_framer_deliver_input_data(
                self.handle,
                input_buffer.as_ptr(),
                input_buffer.len(),
                message.map_or(core::ptr::null_mut(), FramerMessage::as_ptr),
                i32::from(is_complete),
            )
        };
    }

    /// Stop receiving input callbacks and become pass-through on the input side.
    pub fn pass_through_input(&mut self) {
        unsafe { ffi::nw_shim_framer_pass_through_input(self.handle) };
    }

    /// Pass output bytes from the current message without copying them.
    pub fn pass_output_data(&mut self, output_length: usize) -> bool {
        unsafe { ffi::nw_shim_framer_pass_output_data(self.handle, output_length) != 0 }
    }

    /// Write transformed output bytes.
    pub fn write_output_data(&mut self, output_buffer: &[u8]) {
        unsafe {
            ffi::nw_shim_framer_write_output_data(
                self.handle,
                output_buffer.as_ptr(),
                output_buffer.len(),
            )
        };
    }

    /// Stop receiving output callbacks and become pass-through on the output side.
    pub fn pass_through_output(&mut self) {
        unsafe { ffi::nw_shim_framer_pass_through_output(self.handle) };
    }

    /// Mark the framer as ready.
    pub fn mark_ready(&mut self) {
        unsafe { ffi::nw_shim_framer_mark_ready(self.handle) };
    }

    /// Dynamically prepend another application protocol above this framer.
    #[must_use]
    pub fn prepend_application_protocol(&mut self, options: &FramerOptions) -> bool {
        unsafe {
            ffi::nw_shim_framer_prepend_application_protocol(self.handle, options.handle) != 0
        }
    }

    /// Fail the connection associated with this framer.
    pub fn mark_failed_with_error(&mut self, error_code: i32) {
        unsafe { ffi::nw_shim_framer_mark_failed_with_error(self.handle, error_code as c_int) };
    }

    /// Schedule a wakeup in the future.
    pub fn schedule_wakeup(&mut self, after: Duration) {
        let milliseconds = u64::try_from(after.as_millis()).unwrap_or(u64::MAX);
        unsafe { ffi::nw_shim_framer_schedule_wakeup(self.handle, milliseconds) };
    }

    /// Unschedule any pending wakeup timer.
    pub fn clear_wakeup(&mut self) {
        unsafe { ffi::nw_shim_framer_schedule_wakeup(self.handle, u64::MAX) };
    }

    /// Execute a callback asynchronously on the framer's scheduling context.
    pub fn async_invoke<F>(&mut self, callback: F)
    where
        F: FnMut(&mut FramerContext) + Send + 'static,
    {
        let holder = Box::new(AsyncCallbackHolder(Box::new(callback)));
        unsafe {
            ffi::nw_shim_framer_async(
                self.handle,
                async_callback_trampoline,
                Box::into_raw(holder).cast(),
            )
        };
    }
}

unsafe extern "C" fn create_instance_trampoline(user_info: *mut c_void) -> *mut c_void {
    let owner = unsafe { &*user_info.cast::<FramerCallbacksOwner>() };
    let instance = (owner.factory)();
    Box::into_raw(Box::new(instance)).cast()
}

unsafe extern "C" fn drop_instance_trampoline(instance: *mut c_void) {
    drop(unsafe { Box::from_raw(instance.cast::<Box<dyn Framer>>()) });
}

unsafe extern "C" fn start_trampoline(instance: *mut c_void, framer: *mut c_void) -> c_int {
    let framer_instance = unsafe { &mut *instance.cast::<Box<dyn Framer>>() };
    let mut context = FramerContext::new(framer);
    match framer_instance.as_mut().on_start(&mut context) {
        FramerStart::Ready => ffi::NW_FRAMER_START_READY,
        FramerStart::WillMarkReady => ffi::NW_FRAMER_START_WILL_MARK_READY,
    }
}

unsafe extern "C" fn input_trampoline(instance: *mut c_void, framer: *mut c_void) -> usize {
    let framer_instance = unsafe { &mut *instance.cast::<Box<dyn Framer>>() };
    let mut context = FramerContext::new(framer);
    framer_instance.as_mut().on_input(&mut context)
}

unsafe extern "C" fn output_trampoline(
    instance: *mut c_void,
    framer: *mut c_void,
    message: *mut c_void,
    message_length: usize,
    is_complete: c_int,
) {
    let framer_instance = unsafe { &mut *instance.cast::<Box<dyn Framer>>() };
    let mut context = FramerContext::new(framer);
    let message = (!message.is_null()).then_some(FramerMessageView {
        handle: message,
        _marker: PhantomData,
    });
    framer_instance
        .as_mut()
        .on_output(&mut context, message, message_length, is_complete != 0);
}

unsafe extern "C" fn wakeup_trampoline(instance: *mut c_void, framer: *mut c_void) {
    let framer_instance = unsafe { &mut *instance.cast::<Box<dyn Framer>>() };
    let mut context = FramerContext::new(framer);
    framer_instance.as_mut().on_wakeup(&mut context);
}

unsafe extern "C" fn stop_trampoline(instance: *mut c_void, framer: *mut c_void) -> c_int {
    let framer_instance = unsafe { &mut *instance.cast::<Box<dyn Framer>>() };
    let mut context = FramerContext::new(framer);
    i32::from(framer_instance.as_mut().on_stop(&mut context))
}

unsafe extern "C" fn cleanup_trampoline(instance: *mut c_void, framer: *mut c_void) {
    let framer_instance = unsafe { &mut *instance.cast::<Box<dyn Framer>>() };
    let mut context = FramerContext::new(framer);
    framer_instance.as_mut().on_cleanup(&mut context);
}

unsafe extern "C" fn parse_trampoline<F>(
    buffer: *const u8,
    buffer_length: usize,
    is_complete: c_int,
    user_info: *mut c_void,
) -> usize
where
    F: FnMut(&[u8], bool) -> usize,
{
    let callback = unsafe { &mut *user_info.cast::<F>() };
    let bytes = if buffer.is_null() || buffer_length == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(buffer, buffer_length) }
    };
    callback(bytes, is_complete != 0)
}

unsafe extern "C" fn async_callback_trampoline(framer: *mut c_void, user_info: *mut c_void) {
    let mut holder = unsafe { Box::from_raw(user_info.cast::<AsyncCallbackHolder>()) };
    let mut context = FramerContext::new(framer);
    holder.0.as_mut()(&mut context);
}
