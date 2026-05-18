//! Async stream wrappers for Network.framework callback-based APIs.
//!
//! Each stream type subscribes to one or more handler-based Apple APIs and
//! delivers events through a [`BoundedAsyncStream`] that any async runtime
//! can `.await` on.
//!
//! # Feature gate
//!
//! All types in this module require the **`async`** cargo feature.
//!
//! # Back-pressure policy
//!
//! Streams are **lossy by default**: when the internal buffer is full the
//! oldest event is dropped to make room for the newest. Choose a capacity
//! large enough that your consumer can drain it between bursts.
//!
//! # Drop semantics
//!
//! Dropping any of the stream types automatically unsubscribes from the
//! underlying Network.framework handler and frees the associated sender.

#![cfg(feature = "async")]

use core::ffi::{c_int, c_void};
use core::fmt;
use core::marker::PhantomData;
use core::ptr;
use doom_fish_utils::panic_safe::catch_user_panic;
use doom_fish_utils::stream::{AsyncStreamSender, BoundedAsyncStream, NextItem};

use crate::browser::{BrowseResult, BrowseResultChange, BrowserState};
use crate::error::FrameworkError;
use crate::ffi;

struct SubscriptionHandle {
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

// SAFETY: `SubscriptionHandle` only stores a `Send` cleanup closure and moves
// it to the dropping thread; no raw pointers are shared here.
unsafe impl Send for SubscriptionHandle {}
// SAFETY: shared references never execute the cleanup closure. It is only taken
// and run during `Drop`, which requires unique access.
unsafe impl Sync for SubscriptionHandle {}

impl fmt::Debug for SubscriptionHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubscriptionHandle")
            .field("has_cleanup", &self.cleanup.is_some())
            .finish_non_exhaustive()
    }
}

/// State of an `nw_connection_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Invalid,
    Waiting,
    Preparing,
    Ready,
    Failed,
    Cancelled,
    Unknown(i32),
}

impl ConnectionState {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::Waiting,
            2 => Self::Preparing,
            3 => Self::Ready,
            4 => Self::Failed,
            5 => Self::Cancelled,
            other => Self::Unknown(other),
        }
    }
}

/// Event fired when `nw_connection_set_state_changed_handler` fires.
#[derive(Clone)]
pub struct ConnectionStateEvent {
    pub state: ConnectionState,
    pub error: Option<FrameworkError>,
}

impl fmt::Debug for ConnectionStateEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionStateEvent")
            .field("state", &self.state)
            .field(
                "error",
                &self
                    .error
                    .as_ref()
                    .map(|error| (error.domain(), error.code())),
            )
            .finish()
    }
}

/// Async stream of [`ConnectionStateEvent`] for a [`crate::client::TcpClient`].
#[derive(Debug)]
pub struct ConnectionStateStream<'a> {
    inner: BoundedAsyncStream<ConnectionStateEvent>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::client::TcpClient>,
}

unsafe extern "C" fn connection_state_cb(state: c_int, error: *mut c_void, ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    catch_user_panic("connection_state_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<ConnectionStateEvent>>() };
        let error = if error.is_null() {
            None
        } else {
            // SAFETY: the shim hands this callback a retained `nw_error_t`
            // ownership token, which `FrameworkError::from_raw` takes over.
            Some(unsafe { FrameworkError::from_raw(error) })
        };
        sender.push(ConnectionStateEvent {
            state: ConnectionState::from_raw(state),
            error,
        });
    });
}

impl<'a> ConnectionStateStream<'a> {
    /// Subscribe to connection state changes.
    #[must_use]
    pub fn subscribe(client: &'a crate::client::TcpClient, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let sender_ptr = Box::into_raw(Box::new(sender));
        let obj_ptr = client.as_ptr();
        let sender_addr = sender_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed connection handle and
        // `sender_ptr` stays valid until the cleanup closure clears the handler,
        // drains the queue, and frees it.
        unsafe {
            ffi::nw_shim_connection_set_state_changed_handler(
                obj_ptr,
                Some(connection_state_cb),
                sender_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let sender_ptr = sender_addr as *mut AsyncStreamSender<ConnectionStateEvent>;
            // SAFETY: `sender_ptr` was produced by `Box::into_raw` above. The
            // handler is cleared and the queue is drained before we reconstruct
            // the box, so no callback can ever observe `sender_ptr` again.
            unsafe {
                ffi::nw_shim_connection_set_state_changed_handler(obj_ptr, None, ptr::null_mut());
                ffi::nw_shim_connection_drain_queue(obj_ptr);
                drop(Box::from_raw(sender_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, ConnectionStateEvent> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<ConnectionStateEvent> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// Async stream of viability changes (`true` = viable) for a [`crate::client::TcpClient`].
#[derive(Debug)]
pub struct ConnectionViabilityStream<'a> {
    inner: BoundedAsyncStream<bool>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::client::TcpClient>,
}

unsafe extern "C" fn connection_viability_cb(value: c_int, ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    catch_user_panic("connection_viability_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<bool>>() };
        sender.push(value != 0);
    });
}

impl<'a> ConnectionViabilityStream<'a> {
    /// Subscribe to viability changes.
    #[must_use]
    pub fn subscribe(client: &'a crate::client::TcpClient, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let sender_ptr = Box::into_raw(Box::new(sender));
        let obj_ptr = client.as_ptr();
        let sender_addr = sender_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed connection handle and
        // `sender_ptr` stays valid until the cleanup closure clears the handler,
        // drains the queue, and frees it.
        unsafe {
            ffi::nw_shim_connection_set_viability_changed_handler(
                obj_ptr,
                Some(connection_viability_cb),
                sender_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let sender_ptr = sender_addr as *mut AsyncStreamSender<bool>;
            // SAFETY: `sender_ptr` was produced by `Box::into_raw` above. The
            // handler is cleared and the queue is drained before we reconstruct
            // the box, so no callback can ever observe `sender_ptr` again.
            unsafe {
                ffi::nw_shim_connection_set_viability_changed_handler(
                    obj_ptr,
                    None,
                    ptr::null_mut(),
                );
                ffi::nw_shim_connection_drain_queue(obj_ptr);
                drop(Box::from_raw(sender_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, bool> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<bool> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// Async stream of better-path-available events for a [`crate::client::TcpClient`].
#[derive(Debug)]
pub struct ConnectionBetterPathStream<'a> {
    inner: BoundedAsyncStream<bool>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::client::TcpClient>,
}

unsafe extern "C" fn connection_better_path_cb(value: c_int, ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    catch_user_panic("connection_better_path_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<bool>>() };
        sender.push(value != 0);
    });
}

impl<'a> ConnectionBetterPathStream<'a> {
    /// Subscribe to better-path notifications.
    #[must_use]
    pub fn subscribe(client: &'a crate::client::TcpClient, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let sender_ptr = Box::into_raw(Box::new(sender));
        let obj_ptr = client.as_ptr();
        let sender_addr = sender_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed connection handle and
        // `sender_ptr` stays valid until the cleanup closure clears the handler,
        // drains the queue, and frees it.
        unsafe {
            ffi::nw_shim_connection_set_better_path_available_handler(
                obj_ptr,
                Some(connection_better_path_cb),
                sender_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let sender_ptr = sender_addr as *mut AsyncStreamSender<bool>;
            // SAFETY: `sender_ptr` was produced by `Box::into_raw` above. The
            // handler is cleared and the queue is drained before we reconstruct
            // the box, so no callback can ever observe `sender_ptr` again.
            unsafe {
                ffi::nw_shim_connection_set_better_path_available_handler(
                    obj_ptr,
                    None,
                    ptr::null_mut(),
                );
                ffi::nw_shim_connection_drain_queue(obj_ptr);
                drop(Box::from_raw(sender_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, bool> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<bool> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// Async stream of path-changed events for a [`crate::client::TcpClient`].
#[derive(Debug)]
pub struct ConnectionPathChangedStream<'a> {
    inner: BoundedAsyncStream<crate::path::Path>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::client::TcpClient>,
}

unsafe extern "C" fn connection_path_changed_cb(path: *mut c_void, ctx: *mut c_void) {
    if path.is_null() || ctx.is_null() {
        return;
    }

    catch_user_panic("connection_path_changed_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<crate::path::Path>>() };
        // SAFETY: the shim passes this callback a retained `nw_path_t`
        // ownership token, which `Path::from_raw` takes over.
        let path = unsafe { crate::path::Path::from_raw(path) };
        sender.push(path);
    });
}

impl<'a> ConnectionPathChangedStream<'a> {
    /// Subscribe to path changes.
    #[must_use]
    pub fn subscribe(client: &'a crate::client::TcpClient, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let sender_ptr = Box::into_raw(Box::new(sender));
        let obj_ptr = client.as_ptr();
        let sender_addr = sender_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed connection handle and
        // `sender_ptr` stays valid until the cleanup closure clears the handler,
        // drains the queue, and frees it.
        unsafe {
            ffi::nw_shim_connection_set_path_changed_handler(
                obj_ptr,
                Some(connection_path_changed_cb),
                sender_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let sender_ptr = sender_addr as *mut AsyncStreamSender<crate::path::Path>;
            // SAFETY: `sender_ptr` was produced by `Box::into_raw` above. The
            // handler is cleared and the queue is drained before we reconstruct
            // the box, so no callback can ever observe `sender_ptr` again.
            unsafe {
                ffi::nw_shim_connection_set_path_changed_handler(obj_ptr, None, ptr::null_mut());
                ffi::nw_shim_connection_drain_queue(obj_ptr);
                drop(Box::from_raw(sender_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, crate::path::Path> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<crate::path::Path> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// State of an `nw_listener_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerState {
    Invalid,
    Waiting,
    Ready,
    Failed,
    Cancelled,
    Unknown(i32),
}

impl ListenerState {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Invalid,
            1 => Self::Waiting,
            2 => Self::Ready,
            3 => Self::Failed,
            4 => Self::Cancelled,
            other => Self::Unknown(other),
        }
    }
}

/// Event from a [`ListenerEventStream`].
pub enum ListenerEvent {
    /// Listener state changed.
    State {
        state: ListenerState,
        error: Option<FrameworkError>,
    },
    /// A new inbound connection was accepted.
    NewConnection(crate::client::TcpClient),
}

impl fmt::Debug for ListenerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State { state, error } => f
                .debug_struct("State")
                .field("state", state)
                .field(
                    "error",
                    &error.as_ref().map(|error| (error.domain(), error.code())),
                )
                .finish(),
            Self::NewConnection(_) => f.write_str("NewConnection(TcpClient { .. })"),
        }
    }
}

struct ListenerNewConnectionContext {
    sender: AsyncStreamSender<ListenerEvent>,
    keepalives: crate::parameters::KeepAlives,
}

/// Async stream of [`ListenerEvent`] for a [`crate::listener::TcpListener`].
#[derive(Debug)]
pub struct ListenerEventStream<'a> {
    inner: BoundedAsyncStream<ListenerEvent>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::listener::TcpListener>,
}

unsafe extern "C" fn listener_state_cb(state: c_int, error: *mut c_void, ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    catch_user_panic("listener_state_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<ListenerEvent>>() };
        let error = if error.is_null() {
            None
        } else {
            // SAFETY: the shim hands this callback a retained `nw_error_t`
            // ownership token, which `FrameworkError::from_raw` takes over.
            Some(unsafe { FrameworkError::from_raw(error) })
        };
        sender.push(ListenerEvent::State {
            state: ListenerState::from_raw(state),
            error,
        });
    });
}

unsafe extern "C" fn listener_new_connection_cb(connection_handle: *mut c_void, ctx: *mut c_void) {
    if connection_handle.is_null() || ctx.is_null() {
        return;
    }

    catch_user_panic("listener_new_connection_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the context box.
        let ctx = unsafe { &*ctx.cast::<ListenerNewConnectionContext>() };
        // SAFETY: `connection_handle` is a live accepted-connection handle
        // produced by the listener shim, and ownership transfers to the new
        // `TcpClient` wrapper.
        let client = unsafe {
            crate::client::TcpClient::from_raw_with_keepalives(
                connection_handle,
                ctx.keepalives.clone(),
            )
        };
        ctx.sender.push(ListenerEvent::NewConnection(client));
    });
}

impl<'a> ListenerEventStream<'a> {
    /// Subscribe to listener state changes and inbound connections.
    #[must_use]
    pub fn subscribe(listener: &'a crate::listener::TcpListener, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let state_ptr = Box::into_raw(Box::new(sender.clone()));
        let conn_ptr = Box::into_raw(Box::new(ListenerNewConnectionContext {
            sender,
            keepalives: listener.keepalives(),
        }));
        let obj_ptr = listener.as_ptr();
        let state_addr = state_ptr as usize;
        let conn_addr = conn_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed listener handle, and both box
        // pointers stay valid until the cleanup closure clears the handlers,
        // drains the queue, and frees them.
        unsafe {
            ffi::nw_shim_listener_set_state_changed_handler(
                obj_ptr,
                Some(listener_state_cb),
                state_ptr.cast(),
            );
            ffi::nw_shim_listener_set_new_connection_handler(
                obj_ptr,
                Some(listener_new_connection_cb),
                conn_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let state_ptr = state_addr as *mut AsyncStreamSender<ListenerEvent>;
            let conn_ptr = conn_addr as *mut ListenerNewConnectionContext;
            // SAFETY: `state_ptr` and `conn_ptr` were produced by `Box::into_raw`
            // above. Both handlers are cleared and the queue is drained before
            // we reconstruct the boxes, so no callback can ever observe these
            // pointers again.
            unsafe {
                ffi::nw_shim_listener_set_state_changed_handler(obj_ptr, None, ptr::null_mut());
                ffi::nw_shim_listener_set_new_connection_handler(obj_ptr, None, ptr::null_mut());
                ffi::nw_shim_listener_drain_queue(obj_ptr);
                drop(Box::from_raw(state_ptr));
                drop(Box::from_raw(conn_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, ListenerEvent> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<ListenerEvent> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// Async stream of path updates from a [`crate::path_monitor::PathMonitor`].
#[derive(Debug)]
pub struct PathUpdateStream<'a> {
    inner: BoundedAsyncStream<crate::path::Path>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::path_monitor::PathMonitor>,
}

unsafe extern "C" fn path_update_cb(path: *mut c_void, ctx: *mut c_void) {
    if path.is_null() || ctx.is_null() {
        return;
    }

    catch_user_panic("path_update_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<crate::path::Path>>() };
        // SAFETY: the shim passes this callback a retained `nw_path_t`
        // ownership token, which `Path::from_raw` takes over.
        let path = unsafe { crate::path::Path::from_raw(path) };
        sender.push(path);
    });
}

impl<'a> PathUpdateStream<'a> {
    /// Subscribe to path monitor updates.
    #[must_use]
    pub fn subscribe(monitor: &'a crate::path_monitor::PathMonitor, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let sender_ptr = Box::into_raw(Box::new(sender));
        let obj_ptr = monitor.as_ptr();
        let sender_addr = sender_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed path-monitor handle and
        // `sender_ptr` stays valid until the cleanup closure clears the handler,
        // drains the queue, and frees it.
        unsafe {
            ffi::nw_shim_path_monitor_set_update_handler(
                obj_ptr,
                Some(path_update_cb),
                sender_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let sender_ptr = sender_addr as *mut AsyncStreamSender<crate::path::Path>;
            // SAFETY: `sender_ptr` was produced by `Box::into_raw` above. The
            // handler is cleared and the queue is drained before we reconstruct
            // the box, so no callback can ever observe `sender_ptr` again.
            unsafe {
                ffi::nw_shim_path_monitor_set_update_handler(obj_ptr, None, ptr::null_mut());
                ffi::nw_shim_path_monitor_drain_queue(obj_ptr);
                drop(Box::from_raw(sender_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, crate::path::Path> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<crate::path::Path> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// Event from a [`BrowserEventStream`].
#[derive(Clone)]
pub enum BrowserAsyncEvent {
    /// Browser state changed.
    State {
        state: BrowserState,
        error: Option<FrameworkError>,
    },
    /// Browse results changed.
    Results {
        old_result: Option<BrowseResult>,
        new_result: Option<BrowseResult>,
        changes: BrowseResultChange,
        batch_complete: bool,
    },
}

impl fmt::Debug for BrowserAsyncEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State { state, error } => f
                .debug_struct("State")
                .field("state", state)
                .field(
                    "error",
                    &error.as_ref().map(|error| (error.domain(), error.code())),
                )
                .finish(),
            Self::Results {
                old_result,
                new_result,
                changes,
                batch_complete,
            } => f
                .debug_struct("Results")
                .field("old_result_present", &old_result.is_some())
                .field("new_result_present", &new_result.is_some())
                .field("changes", changes)
                .field("batch_complete", batch_complete)
                .finish(),
        }
    }
}

/// Async stream of [`BrowserAsyncEvent`] from a [`crate::browser::Browser`].
#[derive(Debug)]
pub struct BrowserEventStream<'a> {
    inner: BoundedAsyncStream<BrowserAsyncEvent>,
    _handle: SubscriptionHandle,
    _owner: PhantomData<&'a crate::browser::Browser>,
}

unsafe extern "C" fn browser_state_cb(state: c_int, error: *mut c_void, ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    catch_user_panic("browser_state_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<BrowserAsyncEvent>>() };
        let error = if error.is_null() {
            None
        } else {
            // SAFETY: the shim hands this callback a retained `nw_error_t`
            // ownership token, which `FrameworkError::from_raw` takes over.
            Some(unsafe { FrameworkError::from_raw(error) })
        };
        sender.push(BrowserAsyncEvent::State {
            state: BrowserState::from_raw(state),
            error,
        });
    });
}

unsafe extern "C" fn browser_results_cb(
    old_result: *mut c_void,
    new_result: *mut c_void,
    changes: u64,
    batch_complete: c_int,
    ctx: *mut c_void,
) {
    if ctx.is_null() {
        return;
    }

    catch_user_panic("browser_results_cb", || {
        // SAFETY: `ctx` was created by `Box::into_raw` in `subscribe` and
        // remains valid until cleanup clears the handler, drains the queue,
        // and reclaims the sender box.
        let sender = unsafe { &*ctx.cast::<AsyncStreamSender<BrowserAsyncEvent>>() };
        let old_result = if old_result.is_null() {
            None
        } else {
            // SAFETY: the shim passes retained browse-result handles to the
            // callback, and ownership transfers to the Rust wrappers.
            Some(unsafe { BrowseResult::from_raw(old_result) })
        };
        let new_result = if new_result.is_null() {
            None
        } else {
            // SAFETY: the shim passes retained browse-result handles to the
            // callback, and ownership transfers to the Rust wrappers.
            Some(unsafe { BrowseResult::from_raw(new_result) })
        };
        sender.push(BrowserAsyncEvent::Results {
            old_result,
            new_result,
            changes: BrowseResultChange::from_raw(changes),
            batch_complete: batch_complete != 0,
        });
    });
}

impl<'a> BrowserEventStream<'a> {
    /// Subscribe to browser state and browse-result updates.
    #[must_use]
    pub fn subscribe(browser: &'a crate::browser::Browser, capacity: usize) -> Self {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let state_ptr = Box::into_raw(Box::new(sender.clone()));
        let results_ptr = Box::into_raw(Box::new(sender));
        let obj_ptr = browser.as_ptr();
        let state_addr = state_ptr as usize;
        let results_addr = results_ptr as usize;
        let obj_addr = obj_ptr as usize;
        // SAFETY: `obj_ptr` is a live borrowed browser handle, and both box
        // pointers stay valid until the cleanup closure clears the handlers,
        // drains the queue, and frees them.
        unsafe {
            ffi::nw_shim_browser_set_state_changed_handler(
                obj_ptr,
                Some(browser_state_cb),
                state_ptr.cast(),
            );
            ffi::nw_shim_browser_set_browse_results_changed_handler(
                obj_ptr,
                Some(browser_results_cb),
                results_ptr.cast(),
            );
        }
        let cleanup: Box<dyn FnOnce() + Send> = Box::new(move || {
            let obj_ptr = obj_addr as *mut c_void;
            let state_ptr = state_addr as *mut AsyncStreamSender<BrowserAsyncEvent>;
            let results_ptr = results_addr as *mut AsyncStreamSender<BrowserAsyncEvent>;
            // SAFETY: `state_ptr` and `results_ptr` were produced by
            // `Box::into_raw` above. Both handlers are cleared and the queue is
            // drained before we reconstruct the boxes, so no callback can ever
            // observe these pointers again.
            unsafe {
                ffi::nw_shim_browser_set_state_changed_handler(obj_ptr, None, ptr::null_mut());
                ffi::nw_shim_browser_set_browse_results_changed_handler(
                    obj_ptr,
                    None,
                    ptr::null_mut(),
                );
                ffi::nw_shim_browser_drain_queue(obj_ptr);
                drop(Box::from_raw(state_ptr));
                drop(Box::from_raw(results_ptr));
            }
        });
        Self {
            inner: stream,
            _handle: SubscriptionHandle {
                cleanup: Some(cleanup),
            },
            _owner: PhantomData,
        }
    }

    /// Asynchronously wait for the next event.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, BrowserAsyncEvent> {
        self.inner.next()
    }

    /// Try to get an event without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<BrowserAsyncEvent> {
        self.inner.try_next()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}
