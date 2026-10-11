//! For debug builds: finds a control on screen by its name for the dev machine's
//! `cargo device-input tap <name>` (D48's names, as scenarios use them), turns the screen's
//! precision check on and off (`gradient on|off`, S37) and loses the GPU device on purpose
//! (`lose-device`, S49). Only Kotlin's debug source set declares
//! them (`DebugControls`, called from `GestureInstrumentation` on the main thread); the library
//! is built with the release profile for every APK, so a release APK carries these symbols with
//! nothing that can call them.

use std::sync::atomic::{AtomicI64, Ordering};

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jlong};

use crate::error::HostError;
use crate::jni_entry::runtime;

/// The handle of the running activity's runtime; 0 while there is none.
static HANDLE: AtomicI64 = AtomicI64::new(0);

/// Called by `create` and `destroy` with the handle Kotlin now holds.
pub(crate) fn remember(handle: jlong) {
    HANDLE.store(handle, Ordering::SeqCst);
}

/// "x,y": the middle of the control called `name` (or `name #n`) in the last frame, in screen
/// pixels; otherwise why not (`Unresolved`), a tab and every name on screen, one per line.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_find<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    name: JString<'local>,
) -> JObject<'local> {
    env.with_env(|env| -> Result<JObject<'local>, HostError> {
        let name = name.try_to_string(env)?;
        let line = match runtime(HANDLE.load(Ordering::SeqCst)) {
            None => "no runtime".to_owned(),
            Some(runtime) => {
                let controls = runtime.controls();
                match controls.find(&name) {
                    Ok(control) => {
                        let [x, y] = control.at([0.5, 0.5]);
                        format!("{x},{y}")
                    }
                    Err(why) => format!("{why:?}\t{}", controls.names().join("\n")),
                }
            }
        };
        Ok(env.new_string(line)?.into())
    })
    .resolve::<LogErrorAndDefault>()
}

/// For `cargo device-input lose-device` (S49): the GPU device destroyed under the app as a driver
/// reset loses it, then one frame, which makes the renderer again.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_loseDevice<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    env.with_env(|_| -> Result<(), HostError> {
        if let Some(runtime) = runtime(HANDLE.load(Ordering::SeqCst)) {
            let reported = runtime.lose_device();
            log::info!("device lost on purpose; wgpu's lost callback reported it: {reported}");
            runtime.frame()?;
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// For `cargo device-input gradient on|off` (S37): the screen's precision check on or off, and
/// one frame drawn at once so it shows without a touch.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_gradient<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    on: jboolean,
) {
    env.with_env(|_| -> Result<(), HostError> {
        if let Some(runtime) = runtime(HANDLE.load(Ordering::SeqCst)) {
            runtime.show_gradient(on);
            runtime.frame()?;
        }
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}
