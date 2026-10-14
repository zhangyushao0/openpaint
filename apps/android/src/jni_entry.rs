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
use openpaint_runtime::{DisplayCache, Runtime, TextFonts};
use openpaint_store::Library;

use crate::error::HostError;
use crate::fonts::read_face;
use crate::latency::PenLatency;
use crate::motion::{RawMotion, decode};
use crate::navigation::{Frame, NavigationLog};
use crate::transfer::Pick;
use crate::window::NativeWindow;

/// What the handle Kotlin holds points to.
pub(crate) struct Host {
    pub(crate) runtime: Runtime,
    latency: PenLatency,
    /// Frames while fingers navigate or work waits (S53).
    navigation: NavigationLog,
    /// Where files to share are made (`Context.getCacheDir()/share`, served by `ShareProvider`).
    pub(crate) share_dir: PathBuf,
    /// What the Kotlin host was last asked to show; its answer is for that.
    pub(crate) pending: Option<Pick>,
}

/// The handle Kotlin holds is a leaked `Box<Host>`; 0 means "no runtime".
pub(crate) fn host<'a>(handle: jlong) -> Option<&'a mut Host> {
    // SAFETY: non-zero handles come from `create` and stay valid until `destroy`; Kotlin calls
    // in on one thread only, so no other reference is alive during a call.
    unsafe { (handle as *mut Host).as_mut() }
}

pub(crate) fn runtime<'a>(handle: jlong) -> Option<&'a mut Runtime> {
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
    cache_dir: JString<'local>,
) -> jlong {
    init_logging();
    env.with_env(|env| -> Result<jlong, HostError> {
        // The app's private storage (`Context.getFilesDir()`); the library opens on a worker.
        let library = PathBuf::from(files_dir.try_to_string(env)?).join("library");
        let cache_dir = PathBuf::from(cache_dir.try_to_string(env)?);
        let share_dir = cache_dir.join("share");
        // Each artwork's canvas as last shown complete (S54), derived: the system may clear it.
        let display = DisplayCache::at(cache_dir.join("canvas"));
        let runtime = Runtime::start_cached(density, Box::new(Library::at(library)), display)?;
        let host = Host {
            runtime,
            latency: PenLatency::default(),
            navigation: NavigationLog::default(),
            share_dir,
            pending: None,
        };
        let handle = Box::into_raw(Box::new(host)) as jlong;
        crate::jni_debug::remember(handle);
        Ok(handle)
    })
    .resolve::<LogErrorAndDefault>()
}

/// The system's text faces, each "<index>\n<path>" or null when the Kotlin host found none:
/// the regular sans and the regular Simplified Chinese face. Called once, before the first
/// frame; a face that cannot be read is logged and left out.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_fonts<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    latin: JString<'local>,
    cjk: JString<'local>,
) {
    env.with_env(|env| -> Result<(), HostError> {
        let Some(runtime) = runtime(handle) else {
            return Ok(());
        };
        let face = |line: &JString<'local>, what: &str| -> Result<_, HostError> {
            if line.is_null() {
                log::warn!("no system {what} face");
                return Ok(None);
            }
            let line = line.try_to_string(env)?;
            Ok(read_face(&line)
                .inspect(|face| log::info!("{what} face: {} bytes", face.bytes.len()))
                .inspect_err(|error| log::error!("{what} face unreadable: {error}"))
                .ok())
        };
        let fonts = TextFonts {
            latin: face(&latin, "Latin")?,
            cjk: face(&cjk, "Chinese")?,
        };
        runtime.set_text_fonts(fonts);
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
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

/// The device's offset from UTC in minutes (`onCreate`, `onResume`), for the dates of kept
/// versions.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_zone<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    minutes: jni::sys::jint,
) {
    if let Some(runtime) = runtime(handle) {
        runtime.set_utc_offset(minutes);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_destroy<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    crate::jni_debug::remember(0);
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
        let core = host.runtime.core();
        let frame = Frame {
            start_ns,
            work_ns: work,
            waiting: host.runtime.moving(),
            counts: core.counts(),
            background: core.background(),
        };
        if let Some(summary) = host.navigation.frame(frame) {
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
        host.navigation.input(&batch);
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
