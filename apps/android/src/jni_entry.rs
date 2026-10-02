//! JNI entry points for `dev.openpaint.Native` (Kotlin). All calls arrive on the Android main
//! thread. Errors and panics are logged and turned into a default return value; nothing is
//! thrown into Java.

use std::path::PathBuf;
use std::sync::Once;
use std::time::Instant;

use jni::errors::LogErrorAndDefault;
use jni::objects::{
    JClass, JFloatArray, JIntArray, JLongArray, JObject, JPrimitiveArray, JString, TypeArray,
};
use jni::sys::{jboolean, jfloat, jint, jlong};
use jni::{Env, EnvUnowned};
use openpaint_runtime::Runtime;
use openpaint_store::Library;

use crate::error::HostError;
use crate::latency::PenLatency;
use crate::motion::{RawMotion, decode};
use crate::window::NativeWindow;

/// What the handle Kotlin holds points to.
struct Host {
    runtime: Runtime,
    latency: PenLatency,
}

/// The handle Kotlin holds is a leaked `Box<Host>`; 0 means "no runtime".
fn host<'a>(handle: jlong) -> Option<&'a mut Host> {
    // SAFETY: non-zero handles come from `create` and stay valid until `destroy`; Kotlin calls
    // in on one thread only, so no other reference is alive during a call.
    unsafe { (handle as *mut Host).as_mut() }
}

fn runtime<'a>(handle: jlong) -> Option<&'a mut Runtime> {
    host(handle).map(|host| &mut host.runtime)
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
                // The store's database reports every keyspace recovery at info level.
                .with_filter(
                    android_logger::FilterBuilder::new()
                        .parse("info,fjall=warn,lsm_tree=warn")
                        .build(),
                )
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
    files_dir: JString<'local>,
) -> jlong {
    init_logging();
    env.with_env(|env| -> Result<jlong, HostError> {
        // The app's private storage (`Context.getFilesDir()`); the library opens on a worker.
        let library = PathBuf::from(files_dir.try_to_string(env)?).join("library");
        let runtime = Runtime::start(density, Box::new(Library::at(library)))?;
        let host = Host {
            runtime,
            latency: PenLatency::default(),
        };
        Ok(Box::into_raw(Box::new(host)) as jlong)
    })
    .resolve::<LogErrorAndDefault>()
}

/// The user leaves the app (`onPause`): waits briefly for the current version to be stored.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_leave<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    if let Some(runtime) = runtime(handle) {
        runtime.leave();
        log::info!("left the app");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_destroy<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    if handle != 0 {
        // SAFETY: the handle came from `create`, and Kotlin forgets it after this call.
        drop(unsafe { Box::from_raw(handle as *mut Host) });
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

/// `start_ns` is `System.nanoTime()` as Kotlin calls in; the pen latency is logged per stroke.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_frame<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    start_ns: jlong,
) -> jboolean {
    env.with_env(|_| -> Result<jboolean, HostError> {
        let Some(host) = host(handle) else {
            return Ok(false);
        };
        let started = Instant::now();
        let again = host.runtime.frame()?;
        let work = i64::try_from(started.elapsed().as_nanos()).unwrap_or(i64::MAX);
        if let Some(summary) = host.latency.frame(start_ns, start_ns.saturating_add(work)) {
            log::info!("{summary}");
        }
        Ok(again)
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
    flags: jint,
    buttons: jint,
    ids: JIntArray<'local>,
    tools: JIntArray<'local>,
    samples: JFloatArray<'local>,
    times_ms: JLongArray<'local>,
    event_ns: jlong,
) -> jboolean {
    env.with_env(|env| -> Result<jboolean, HostError> {
        let Some(host) = host(handle) else {
            return Ok(false);
        };
        let (ids, tools) = (read(env, &ids)?, read(env, &tools)?);
        let (samples, times_ms) = (read(env, &samples)?, read(env, &times_ms)?);
        let raw = RawMotion {
            action,
            action_index,
            flags,
            buttons,
            ids: &ids,
            tools: &tools,
            samples: &samples,
            times_ms: &times_ms,
        };
        let Some(batch) = decode(&raw)? else {
            return Ok(false);
        };
        host.latency.input(&batch, event_ns);
        Ok(host.runtime.motion(&batch))
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
