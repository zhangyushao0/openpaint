//! Android composition root: activity lifecycle, Surface, system pickers, the replay's video
//! encoder (`video.rs`), JNI. The only crate allowed to hold `unsafe`, and only in the JNI,
//! video and native-window modules. No product rules.
//!
//! The Kotlin host (`app/src/main/java/dev/openpaint/`) owns the lifecycle and the
//! `SurfaceView` (DECISIONS D11); this crate turns its calls into `Runtime` calls. Everything
//! JNI is Android-only; the motion decoding and the pen latency probe are plain Rust and
//! tested on the host.

pub mod fonts;
pub mod keys;
pub mod latency;
pub mod motion;
pub mod navigation;
pub mod pacing;
pub mod transfer;

#[cfg(test)]
mod debug_only;
#[cfg(test)]
mod host_frames;
#[cfg(test)]
mod test_dir;

#[cfg(target_os = "android")]
mod error;
#[cfg(target_os = "android")]
#[expect(unsafe_code, reason = "JNI entry points export unmangled symbols")]
mod jni_debug;
#[cfg(target_os = "android")]
#[expect(unsafe_code, reason = "JNI entry points export unmangled symbols")]
mod jni_entry;
#[cfg(target_os = "android")]
#[expect(unsafe_code, reason = "JNI entry points export unmangled symbols")]
mod jni_keyboard;
#[cfg(target_os = "android")]
#[expect(unsafe_code, reason = "JNI entry points export unmangled symbols")]
mod jni_timer;
#[cfg(target_os = "android")]
#[expect(
    unsafe_code,
    reason = "JNI entry points export unmangled symbols and take over file descriptors"
)]
mod jni_transfer;
#[cfg(target_os = "android")]
#[expect(
    unsafe_code,
    reason = "a video's frames go to the Kotlin encoder as direct ByteBuffers"
)]
mod video;
#[cfg(target_os = "android")]
#[expect(
    unsafe_code,
    reason = "ANativeWindow is reached through the NDK's C API"
)]
mod window;
