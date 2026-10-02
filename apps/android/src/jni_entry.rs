//! JNI entry points for `dev.openpaint.Native` (Kotlin). All calls arrive on the Android main
//! thread. Errors and panics are logged and turned into a default return value; nothing is
//! thrown into Java.

use std::sync::Once;

use jni::errors::LogErrorAndDefault;
use jni::objects::{
    JClass, JFloatArray, JIntArray, JLongArray, JObject, JPrimitiveArray, TypeArray,
};
use jni::sys::{jboolean, jfloat, jint, jlong};
use jni::{Env, EnvUnowned};
use openpaint_runtime::Runtime;

use crate::error::HostError;
use crate::motion::{RawMotion, decode};
use crate::window::NativeWindow;

/// The handle Kotlin holds is a leaked `Box<Runtime>`; 0 means "no runtime".
fn runtime<'a>(handle: jlong) -> Option<&'a mut Runtime> {
    // SAFETY: non-zero handles come from `create` and stay valid until `destroy`; Kotlin calls
    // in on one thread only, so no other reference is alive during a call.
    unsafe { (handle as *mut Runtime).as_mut() }
}

fn size(width: jint, height: jint) -> [u32; 2] {
    [width, height].map(|side| u32::try_from(side).unwrap_or(0))
}

fn init_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("openpaint"),
        );
        // The JNI layer catches panics but loses non-string messages and locations.
        std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_create<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    density: jfloat,
) -> jlong {
    init_logging();
    env.with_env(|_| -> Result<jlong, HostError> {
        let runtime = Runtime::start(density)?;
        Ok(Box::into_raw(Box::new(runtime)) as jlong)
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_destroy<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    if handle != 0 {
        // SAFETY: the handle came from `create`, and Kotlin forgets it after this call.
        drop(unsafe { Box::from_raw(handle as *mut Runtime) });
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_attach<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    surface: JObject<'local>,
    width: jint,
    height: jint,
    density: jfloat,
) {
    env.with_env(|env| -> Result<(), HostError> {
        let Some(runtime) = runtime(handle) else {
            return Ok(());
        };
        let window = NativeWindow::from_surface(env, &surface).ok_or(HostError::NoWindow)?;
        runtime.resize(size(width, height), density);
        runtime.attach(window, size(width, height))?;
        log::info!("surface attached {width} x {height} at density {density}");
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_resize<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    width: jint,
    height: jint,
    density: jfloat,
) {
    if let Some(runtime) = runtime(handle) {
        runtime.resize(size(width, height), density);
        log::info!("surface resized {width} x {height} at density {density}");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_detach<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    if let Some(runtime) = runtime(handle) {
        runtime.detach();
        log::info!("surface detached");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_frame<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jboolean {
    env.with_env(|_| -> Result<jboolean, HostError> {
        match runtime(handle) {
            Some(runtime) => Ok(runtime.frame()?),
            None => Ok(false),
        }
    })
    .resolve::<LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_motion<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    action: jint,
    action_index: jint,
    buttons: jint,
    ids: JIntArray<'local>,
    tools: JIntArray<'local>,
    samples: JFloatArray<'local>,
    times_ms: JLongArray<'local>,
) -> jboolean {
    env.with_env(|env| -> Result<jboolean, HostError> {
        let Some(runtime) = runtime(handle) else {
            return Ok(false);
        };
        let (ids, tools) = (read(env, &ids)?, read(env, &tools)?);
        let (samples, times_ms) = (read(env, &samples)?, read(env, &times_ms)?);
        let raw = RawMotion {
            action,
            action_index,
            buttons,
            ids: &ids,
            tools: &tools,
            samples: &samples,
            times_ms: &times_ms,
        };
        Ok(match decode(&raw)? {
            Some(batch) => runtime.motion(&batch),
            None => false,
        })
    })
    .resolve::<LogErrorAndDefault>()
}

/// Copies a whole Java primitive array.
fn read<T: TypeArray + Default>(
    env: &Env<'_>,
    array: &JPrimitiveArray<'_, T>,
) -> Result<Vec<T>, HostError> {
    let mut values = vec![T::default(); array.len(env)?];
    array.get_region(env, 0, &mut values)?;
    Ok(values)
}
