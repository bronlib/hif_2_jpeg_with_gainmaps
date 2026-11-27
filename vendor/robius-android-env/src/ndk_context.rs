//! The implementation for UI toolkits that work with [ndk-context].
//!
//! This module works by accessing the [`AndroidContext`] state stored in [ndk-context],
//! which must be initialized by other crates (e.g., `android-activity`).
//! before this module uses those states.
//!
//! [ndk-context]: https://docs.rs/ndk-context/latest/ndk_context/
//! [`AndroidContext`]: https://docs.rs/ndk-context/latest/ndk_context/struct.AndroidContext.html

use crate::{JavaVM, JNIEnv, JObject};
use std::{ffi::c_void, sync::atomic::{AtomicPtr, Ordering}};

static ACTIVITY_PTR: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn set_activity_ptr(activity: *mut c_void) {
    ACTIVITY_PTR.store(activity, Ordering::Release);
}

pub fn with_activity_inner<F, R>(f: F) -> crate::Result<R>
where
    F: for<'a, 'b, 'c, 'd> FnOnce(&'a mut JNIEnv<'b>, &'c JObject<'d>) -> R,
{
    let android_context = ndk_context::android_context();
    // SAFETY: we have no option but to trust the pointers from ndk-context.
    let (vm, context) = unsafe { (
        JavaVM::from_raw(android_context.vm().cast())?,
        JObject::from_raw(android_context.context().cast()),
    )};

    let activity_ptr = ACTIVITY_PTR.load(Ordering::Acquire);
    let activity = if activity_ptr.is_null() {
        context
    } else {
        unsafe { JObject::from_raw(activity_ptr.cast()) }
    };

    let mut env = vm.attach_current_thread_permanently()?;
    Ok(f(&mut env, &activity))
}
