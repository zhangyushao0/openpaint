//! JNI entry points for the keyboard and the back button (`dev.openpaint.Native`). All calls
//! arrive on the Android main thread; errors are logged, nothing is thrown into Java.

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong};
use openpaint_runtime::Runtime;

use crate::error::HostError;
use crate::jni_entry::runtime;
use crate::keys::{delete_surrounding, edit_key, text_input};

/// Text from the input connection: `kind` 0 composing, 1 committed.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_text<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    kind: jint,
    text: JString<'local>,
) {
    env.with_env(|env| -> Result<(), HostError> {
        let text = text.try_to_string(env)?;
        if let (Some(runtime), Some(input)) = (runtime(handle), text_input(kind, text)) {
            runtime.text(input);
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_deleteSurrounding<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    before: jint,
    after: jint,
) {
    if let Some(runtime) = runtime(handle) {
        runtime.text(delete_surrounding(before, after));
    }
}

/// An editing key (see `keys::edit_key`).
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_key<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    code: jint,
) {
    if let (Some(runtime), Some(input)) = (runtime(handle), edit_key(code)) {
        runtime.text(input);
    }
}

/// Whether a text field waits for the keyboard after the last frame.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_keyboard<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jboolean {
    runtime(handle).is_some_and(|runtime| runtime.keyboard_wanted())
}

/// The back button; returns whether the app used it.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_back<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jboolean {
    let used = runtime(handle).is_some_and(Runtime::back);
    log::info!("back: {}", if used { "used" } else { "system" });
    used
}
