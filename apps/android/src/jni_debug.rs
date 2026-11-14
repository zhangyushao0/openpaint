//! For debug builds: finds a control on screen by its name for the dev machine's
//! `cargo device-input tap <name>` (D48's names, as scenarios use them), says what the app shows
//! (`device-input state`, the device smoke), turns the screen's
//! precision check on and off (`gradient on|off`), loses the GPU device on purpose
//! (`lose-device`), runs the app's scenario benchmarks (`bench`), opens a pushed
//! OpenPaint file without the picker (`open-file`) and starts and removes a test practice
//! lesson beside the author's (`practice-test start|end`, D102).
//! Only Kotlin's debug source set declares
//! them (`DebugControls`, called from `GestureInstrumentation` on the main thread); the library
//! is built with the release profile for every APK, so a release APK carries these symbols with
//! nothing that can call them.

use std::sync::atomic::{AtomicI64, Ordering};

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jlong};

use openpaint_runtime::{
    PracticeTest, Runtime, RuntimeError, SCENARIOS, ScreenProjection, Start, skill_start,
};

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

/// For `cargo device-input state` and the device smoke: what the app shows, fields separated
/// by tabs, each "key value": `screen` (starting, canvas, gallery); in the gallery `listed` (0
/// or 1) and `cards` (the artworks' ids, the one changed last first); `fit` (where a canvas
/// fits: left, top, width, height in pixels); a `name` per control in the last frame.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_state<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JObject<'local> {
    env.with_env(|env| -> Result<JObject<'local>, HostError> {
        let line = runtime(HANDLE.load(Ordering::SeqCst))
            .map_or_else(|| "screen starting".to_owned(), |runtime| state(runtime));
        Ok(env.new_string(line)?.into())
    })
    .resolve::<LogErrorAndDefault>()
}

fn state(runtime: &Runtime) -> String {
    let mut fields = Vec::new();
    match runtime.screen() {
        ScreenProjection::Starting => fields.push("screen starting".to_owned()),
        ScreenProjection::Canvas(_) => fields.push("screen canvas".to_owned()),
        ScreenProjection::Gallery(gallery) => {
            fields.push("screen gallery".to_owned());
            fields.push(format!("listed {}", u8::from(gallery.listed())));
            let ids: Vec<String> = gallery
                .items()
                .iter()
                .map(|item| item.id.get().to_string())
                .collect();
            fields.push(format!("cards {}", ids.join(" ")));
        }
    }
    if let Some(area) = runtime.canvas_area() {
        let ([x, y], [w, h]) = (area.fit.origin, area.fit.size);
        fields.push(format!("fit {x} {y} {w} {h}"));
    }
    let names = runtime.controls().names().into_iter();
    fields.extend(names.map(|name| format!("name {name}")));
    fields.join("\t")
}

/// For `cargo device-input lose-device`: the GPU device destroyed under the app as a driver
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

/// For `cargo device-input bench <scenario>`: that scenario benchmark on the artwork open,
/// driving the app's own frame loop here on the main thread; each line written to the log, and
/// all of them returned separated by tabs.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_bench<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    scene: JString<'local>,
) -> JObject<'local> {
    env.with_env(|env| -> Result<JObject<'local>, HostError> {
        let scene = scene.try_to_string(env)?;
        let lines = match runtime(HANDLE.load(Ordering::SeqCst)) {
            None => vec!["no runtime".to_owned()],
            Some(runtime) if SCENARIOS.contains(&scene.split('@').next().unwrap_or("")) => {
                match runtime.bench_scenario(&scene) {
                    Ok(lines) => lines,
                    Err(error) => vec![format!("bench {scene} failed: {error}")],
                }
            }
            Some(_) => vec![format!(
                "no bench scenario {scene}: {}",
                SCENARIOS.join(", ")
            )],
        };
        for line in &lines {
            log::info!("bench: {line}");
        }
        Ok(env.new_string(lines.join("\t"))?.into())
    })
    .resolve::<LogErrorAndDefault>()
}

/// For `cargo device-input open-file <name>`: a pushed OpenPaint file's or practice pack's `bytes` (read by
/// the instrumentation's shell, as the app may not read the test folder) opened as the gallery's
/// Open OpenPaint file… opens a chosen one, no picker shown. Returns "opening", or why not.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_openFile<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    bytes: JByteArray<'local>,
) -> JObject<'local> {
    env.with_env(|env| -> Result<JObject<'local>, HostError> {
        let bytes = env.convert_byte_array(&bytes)?;
        let line = match runtime(HANDLE.load(Ordering::SeqCst)) {
            None => "no runtime".to_owned(),
            Some(runtime) => match runtime.open_file_now(bytes) {
                Ok(()) => imported(runtime)?,
                Err(error) => format!("not opened: {error}"),
            },
        };
        log::info!("open-file: {line}");
        Ok(env.new_string(line)?.into())
    })
    .resolve::<LogErrorAndDefault>()
}

/// Frames until the library imported the file and the gallery listed it (nothing asks for
/// Choreographer's frames meanwhile: no hand touched the screen), at most a minute: "opening"
/// then, or why not.
fn imported(runtime: &mut Runtime) -> Result<String, HostError> {
    let started = std::time::Instant::now();
    loop {
        runtime.frame()?;
        if !runtime.storing() {
            return Ok(format!(
                "opening, imported in {} ms",
                started.elapsed().as_millis()
            ));
        }
        if started.elapsed().as_secs() > 60 {
            return Ok("opening, still importing after 60 s".to_owned());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// For `cargo device-input practice-test start [lesson <skill> [level] | full [unit] | first |
/// copy] | end`:
/// `start` starts a test lesson (or a test colour copy) beside any open one, from the
/// home, and answers "started <name>" once its sheet shows; `end:<name>` removes the lesson
/// of that name, its record and sheet, from the home, and answers "removed <name>" once the
/// store did, or "no such lesson". Otherwise why not.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_DebugControls_practiceTest<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    command: JString<'local>,
) -> JObject<'local> {
    env.with_env(|env| -> Result<JObject<'local>, HostError> {
        let command = command.try_to_string(env)?;
        let line = match runtime(HANDLE.load(Ordering::SeqCst)) {
            None => "no runtime".to_owned(),
            Some(runtime) => practice_test(runtime, &command)?,
        };
        log::info!("practice-test: {line}");
        Ok(env.new_string(line)?.into())
    })
    .resolve::<LogErrorAndDefault>()
}

/// The test lesson or copy `given` (what follows `start`) asks for on `runtime`: its id, or why
/// not; none when `given` is not understood.
fn test_start(runtime: &mut Runtime, given: &str) -> Option<Result<u64, RuntimeError>> {
    let parts: Vec<&str> = given.split(':').skip(1).collect();
    let number = |part: &str| part.parse::<u8>().ok();
    Some(match parts[..] {
        [] => runtime.start_test_lesson(Start::Next),
        ["first"] => runtime.start_test_lesson(Start::First),
        ["copy"] => runtime.start_test_copy(),
        ["lesson", skill] => skill_start(skill, None).and_then(|s| runtime.start_test_lesson(s)),
        ["lesson", skill, level] => skill_start(skill, Some(number(level)?))
            .and_then(|start| runtime.start_test_lesson(start)),
        _ => return None,
    })
}

/// `command` (`start`, `start:first`, `start:copy`, `start:lesson:<skill>[:<level>]` or `end:<name>`) on `runtime`, frames run here until it is
/// done, at most a minute: the answer.
fn practice_test(runtime: &mut Runtime, command: &str) -> Result<String, HostError> {
    let start =
        (command.strip_prefix("start")).filter(|given| given.is_empty() || given.starts_with(':'));
    if let Some(given) = start {
        let Some(started) = test_start(runtime, given) else {
            return Ok(format!("not understood: {command}"));
        };
        let id = match started {
            Ok(id) => id,
            Err(error) => return Ok(format!("not started: {error}")),
        };
        if !settled(runtime, &|r| r.test_practice(id).is_some())? {
            return Ok("not started: no sheet after 60 s".to_owned());
        }
        let name = runtime.test_practice(id).map(|test| test.to_string());
        return Ok(format!("started {}", name.unwrap_or_default()));
    }
    let Some(Ok(test)) = command.strip_prefix("end:").map(str::parse::<PracticeTest>) else {
        return Ok(format!("not understood: {command}"));
    };
    if !runtime.names_practice(&test) {
        return Ok("no such lesson".to_owned());
    }
    if let Err(error) = runtime.end_test_practice(&test) {
        return Ok(format!("not removed: {error}"));
    }
    if !settled(runtime, &|r| !r.holds_practice(test.id))? {
        return Ok("not removed after 60 s".to_owned());
    }
    Ok(format!("removed {test}"))
}

/// Frames until `done` holds, no worker is busy and the runtime wants no more frames (what shows
/// has grown in: nothing asks for Choreographer's frames afterwards, as no hand touches the
/// screen), at most a minute; whether it got there.
fn settled(runtime: &mut Runtime, done: &dyn Fn(&Runtime) -> bool) -> Result<bool, HostError> {
    let started = std::time::Instant::now();
    let mut wanted = true;
    while wanted || !done(runtime) || runtime.storing() {
        wanted = runtime.frame()?;
        if started.elapsed().as_secs() > 60 {
            return Ok(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(true)
}

/// For `cargo device-input gradient on|off`: the screen's precision check on or off, and
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
