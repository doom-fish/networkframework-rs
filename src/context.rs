use core::ffi::c_void;
use std::sync::Arc;

use doom_fish_utils::callback_context::CallbackContext;
use doom_fish_utils::panic_safe::catch_user_panic;

use crate::ffi::NwShimContextCallback;

pub struct Subscription<T: Send + Sync + 'static> {
    pub token: u64,
    context: CallbackContext<T>,
}

impl<T: Send + Sync + 'static> Subscription<T> {
    pub fn register(
        value: T,
        register: impl FnOnce(*mut c_void, NwShimContextCallback, NwShimContextCallback) -> u64,
    ) -> Option<Self> {
        let context = CallbackContext::new(value);
        let token = register(
            context.retained_ptr(),
            CallbackContext::<T>::RETAIN,
            CallbackContext::<T>::RELEASE,
        );
        (token != 0).then_some(Self { token, context })
    }

    pub fn deactivate(&self) {
        self.context.deactivate();
    }
}

impl<T: Send + Sync + 'static> std::fmt::Debug for Subscription<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("token", &self.token)
            .field("active", &self.context.is_active())
            .finish()
    }
}

pub unsafe extern "C" fn release_arc<T: Send + Sync + 'static>(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    catch_user_panic("networkframework::release_arc", || {
        drop(unsafe { Arc::from_raw(ptr.cast::<T>()) });
    });
}

pub unsafe extern "C" fn retain_arc<T: Send + Sync + 'static>(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { Arc::increment_strong_count(ptr.cast::<T>()) };
    }
}
