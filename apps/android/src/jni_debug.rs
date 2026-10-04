//! For debug builds: finds a control on screen by its name for the dev machine's
//! `cargo device-input tap <name>` (D48's names, as scenarios use them). Only Kotlin's debug
//! source set declares it (`DebugControls`, called from `GestureInstrumentation` on the main
//! thread); the library is built with the release profile for every APK, so a release APK
//! carries this symbol with nothing that can call it.

use std::sync::atomic::{AtomicI64, Ordering};

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString};
use jni::sys::jlong;

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
