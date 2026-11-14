//! JNI entry points for what the Pomodoro asks of the host after each frame (S124,
//! `dev.openpaint.Native`; Kotlin's `TimerHost`): when the next frame is wanted, whether the
//! screen stays on, whether the break's chime sounds. All calls arrive on the Android main thread.

use jni::EnvUnowned;
use jni::objects::JClass;
use jni::sys::{jboolean, jlong};

use crate::jni_entry::runtime;

/// Past the moment the timer names, so the frame's clock has reached it.
const MARGIN_MS: u128 = 50;

/// Milliseconds after the last frame at which the timer wants a frame; -1 for none.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_wakeIn<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jlong {
    (runtime(handle).and_then(|runtime| runtime.wake_in())).map_or(-1, |wake| {
        jlong::try_from(wake.as_millis() + MARGIN_MS).unwrap_or(jlong::MAX)
    })
}

/// Whether the screen stays on now: a Pomodoro's block or its break counts.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_screenOn<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jboolean {
    runtime(handle).is_some_and(|runtime| runtime.screen_on())
}

/// Whether the break's chime sounds now; true once.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_chime<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jboolean {
    let chime = runtime(handle).is_some_and(|runtime| runtime.take_chime());
    if chime {
        log::info!("the break's chime");
    }
    chime
}
