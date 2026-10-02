//! `NativeWindow`: the `ANativeWindow` behind a Java `Surface`, handed to the renderer as a
//! `raw-window-handle` window.

use std::ffi::c_void;
use std::ptr::NonNull;

use jni::Env;
use jni::objects::JObject;
use raw_window_handle::{
    AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawWindowHandle, WindowHandle,
};

#[link(name = "android")]
unsafe extern "C" {
    fn ANativeWindow_fromSurface(
        env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
    ) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
}

/// Holds one reference to an `ANativeWindow`, released on drop.
pub(crate) struct NativeWindow(NonNull<c_void>);

// SAFETY: `ANativeWindow` is reference counted and may be used and released from any thread.
unsafe impl Send for NativeWindow {}
// SAFETY: as above; every call through the handle is thread-safe on the NDK side.
unsafe impl Sync for NativeWindow {}

impl NativeWindow {
    /// The window behind `surface`, or `None` when the surface is already released.
    pub(crate) fn from_surface(env: &Env<'_>, surface: &JObject<'_>) -> Option<Self> {
        // SAFETY: `env` is the calling thread's JNI environment and `surface` is a live local
        // reference to an `android.view.Surface`; the call acquires a reference we now own.
        let window = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
        NonNull::new(window).map(Self)
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        // SAFETY: we own exactly one reference, acquired in `from_surface`.
        unsafe { ANativeWindow_release(self.0.as_ptr()) }
    }
}

impl HasWindowHandle for NativeWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(self.0));
        // SAFETY: the window stays acquired for as long as `self`, which outlives the borrow.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

/// wgpu refuses to create a surface without a display handle; Android's carries no data.
impl HasDisplayHandle for NativeWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}
