//! The performance hint session (S67, DECISIONS D94): Android's `APerformanceHint` (API 33; GPU
//! durations and power efficiency from API 35), found at run time in `libandroid.so` since the app
//! runs from API 28. The runtime tells it each frame's work while a hand is on or over the glass
//! (`openpaint_runtime::WorkHints`); here those become one session over the frame loop's and the
//! raster worker's threads whose target is the display's vsync, made at the first report after the
//! app came to the front and closed when it leaves. Resting, the session prefers power efficiency
//! (API 35; Android has no pause) and gets no reports. Without the API nothing is made (logged
//! once at start).

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use jni::EnvUnowned;
use jni::objects::JClass;
use jni::sys::jlong;
use openpaint_runtime::{FrameWork, WorkHints};

use crate::jni_entry::host;

unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(library: *mut c_void, name: *const c_char) -> *mut c_void;
    fn gettid() -> i32;
    fn clock_gettime(clock: c_int, time: *mut Timespec) -> c_int;
}

#[repr(C)]
struct Timespec {
    seconds: i64,
    nanos: i64,
}

/// bionic's `RTLD_NOW` and `CLOCK_MONOTONIC` (`System.nanoTime()`'s clock).
const RTLD_NOW: c_int = 2;
const CLOCK_MONOTONIC: c_int = 1;
/// The target until the host says the display's vsync: 120 Hz.
const VSYNC_NS: i64 = 8_333_333;

type Manager = unsafe extern "C" fn() -> *mut c_void;
type Create = unsafe extern "C" fn(*mut c_void, *const i32, usize, i64) -> *mut c_void;
type WithNanos = unsafe extern "C" fn(*mut c_void, i64) -> c_int;
type Close = unsafe extern "C" fn(*mut c_void);
type Prefer = unsafe extern "C" fn(*mut c_void, bool) -> c_int;
type Report2 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;
type WorkNew = unsafe extern "C" fn() -> *mut c_void;
type WorkSet = unsafe extern "C" fn(*mut c_void, i64);

/// The calls of API 33.
struct Api {
    manager: *mut c_void,
    create: Create,
    target: WithNanos,
    report: WithNanos,
    close: Close,
    gpu: Option<Api35>,
}

/// The calls of API 35: a work duration with the GPU's time, power efficiency.
struct Api35 {
    prefer: Prefer,
    report: Report2,
    work: *mut c_void,
    release: Close,
    start: WorkSet,
    total: WorkSet,
    cpu: WorkSet,
    gpu: WorkSet,
}

struct Session {
    api: Api,
    threads: Vec<i32>,
    target_ns: i64,
    /// The session while the app is in front and a hand came; null otherwise.
    open: *mut c_void,
    efficient: bool,
    /// Failures logged so far (a few, then quiet).
    failures: u32,
}

// SAFETY: the pointers are the NDK's manager, session and work duration, used only under the
// mutex; the NDK lets them be used from any thread, one call at a time.
unsafe impl Send for Session {}

/// The host's `WorkHints`.
pub(crate) struct HintSession(Mutex<Session>);

/// The symbol `name` in `library` as a function of type `F`, or `None`.
///
/// # Safety
/// `F` must be the C function type the NDK declares for `name`.
unsafe fn symbol<F: Copy>(library: *mut c_void, name: &CStr) -> Option<F> {
    // SAFETY: `library` is a handle from `dlopen`, `name` a C string.
    let found = unsafe { dlsym(library, name.as_ptr()) };
    // SAFETY: a non-null symbol of that name is a function of type `F` (the caller's word).
    (!found.is_null()).then(|| unsafe { std::mem::transmute_copy::<*mut c_void, F>(&found) })
}

impl HintSession {
    /// The NDK's calls when the device has them and the system makes sessions; else why not.
    pub(crate) fn open() -> Result<Self, &'static str> {
        // SAFETY: a C string naming a system library the app has loaded already.
        let library = unsafe { dlopen(c"libandroid.so".as_ptr(), RTLD_NOW) };
        if library.is_null() {
            return Err("libandroid.so not found");
        }
        // SAFETY: each type is the NDK's declaration of its symbol (android/performance_hint.h).
        let api = unsafe {
            let manager = symbol::<Manager>(library, c"APerformanceHint_getManager")
                .ok_or("no APerformanceHint (API 33)")?;
            let manager = manager();
            if manager.is_null() {
                return Err("no performance hint manager");
            }
            let missing = "an APerformanceHint call is missing";
            let gpu = (|| {
                let work: WorkNew = symbol(library, c"AWorkDuration_create")?;
                Some(Api35 {
                    prefer: symbol(library, c"APerformanceHint_setPreferPowerEfficiency")?,
                    report: symbol(library, c"APerformanceHint_reportActualWorkDuration2")?,
                    start: symbol(library, c"AWorkDuration_setWorkPeriodStartTimestampNanos")?,
                    total: symbol(library, c"AWorkDuration_setActualTotalDurationNanos")?,
                    cpu: symbol(library, c"AWorkDuration_setActualCpuDurationNanos")?,
                    gpu: symbol(library, c"AWorkDuration_setActualGpuDurationNanos")?,
                    release: symbol(library, c"AWorkDuration_release")?,
                    work: work(),
                })
            })()
            .filter(|gpu| !gpu.work.is_null());
            Api {
                manager,
                create: symbol(library, c"APerformanceHint_createSession").ok_or(missing)?,
                target: symbol(library, c"APerformanceHint_updateTargetWorkDuration")
                    .ok_or(missing)?,
                report: symbol(library, c"APerformanceHint_reportActualWorkDuration")
                    .ok_or(missing)?,
                close: symbol(library, c"APerformanceHint_closeSession").ok_or(missing)?,
                gpu,
            }
        };
        Ok(Self(Mutex::new(Session {
            api,
            threads: Vec::new(),
            target_ns: VSYNC_NS,
            open: std::ptr::null_mut(),
            efficient: false,
            failures: 0,
        })))
    }

    /// What it reports: "API 35 (the GPU's time too)" or "API 33".
    pub(crate) fn describe(&self) -> &'static str {
        if self.session().api.gpu.is_some() {
            "API 35 (CPU, GPU and total durations, power efficiency at rest)"
        } else {
            "API 33 (total durations)"
        }
    }

    /// The display's vsync: the session's target from now on.
    pub(crate) fn set_vsync(&self, period_ns: i64) {
        let mut session = self.session();
        session.target_ns = period_ns.max(1);
        if !session.open.is_null() {
            // SAFETY: an open session of this API.
            let failed = unsafe { (session.api.target)(session.open, session.target_ns) };
            session.failed("updating the target", failed);
        }
    }

    fn session(&self) -> std::sync::MutexGuard<'_, Session> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Session {
    /// The session, made with the threads joined so far if there is none.
    fn opened(&mut self) -> Option<*mut c_void> {
        if self.open.is_null() && self.failures < 3 {
            // SAFETY: the manager is the NDK's, the threads this process's.
            self.open = unsafe {
                (self.api.create)(
                    self.api.manager,
                    self.threads.as_ptr(),
                    self.threads.len(),
                    self.target_ns,
                )
            };
            if self.open.is_null() {
                self.failures += 1;
                log::warn!("performance hints: no session made for {:?}", self.threads);
            } else {
                log::info!(
                    "performance hints: a session for threads {:?}, target {:.2} ms",
                    self.threads,
                    self.target_ns as f64 / 1e6
                );
            }
        }
        (!self.open.is_null()).then_some(self.open)
    }

    /// A call that returned `code`: a failure is logged (the first few) and the session made again
    /// at the next report.
    fn failed(&mut self, what: &str, code: c_int) {
        if code == 0 {
            return;
        }
        if self.failures < 3 {
            log::warn!("performance hints: {what} failed ({code})");
        }
        self.failures += 1;
        self.close();
    }

    fn close(&mut self) {
        if !self.open.is_null() {
            // SAFETY: an open session, closed once.
            unsafe { (self.api.close)(self.open) };
            self.open = std::ptr::null_mut();
        }
        self.efficient = false;
    }

    /// Power efficiency on or off (API 35).
    fn prefer_efficiency(&mut self, on: bool) {
        let (Some(gpu), false) = (&self.api.gpu, self.open.is_null()) else {
            return;
        };
        if self.efficient != on {
            // SAFETY: an open session of API 35.
            let code = unsafe { (gpu.prefer)(self.open, on) };
            self.efficient = on;
            self.failed("preferring power efficiency", code);
        }
    }
}

/// `System.nanoTime()` now.
fn monotonic_ns() -> i64 {
    let mut time = Timespec {
        seconds: 0,
        nanos: 0,
    };
    // SAFETY: a valid clock and a timespec to write.
    unsafe { clock_gettime(CLOCK_MONOTONIC, &raw mut time) };
    time.seconds * 1_000_000_000 + time.nanos
}

fn ns(duration: Duration) -> i64 {
    i64::try_from(duration.as_nanos())
        .unwrap_or(i64::MAX)
        .max(1)
}

impl WorkHints for HintSession {
    fn joined(&self) {
        // SAFETY: no arguments; the calling thread's id.
        let thread = unsafe { gettid() };
        let mut session = self.session();
        if !session.threads.contains(&thread) {
            session.threads.push(thread);
            // A session made before covers the threads then; the next report makes it again.
            session.close();
        }
    }

    fn report(&self, work: FrameWork) {
        let mut session = self.session();
        let Some(open) = session.opened() else {
            return;
        };
        session.prefer_efficiency(false);
        let total = ns(work.total);
        let code = match &session.api.gpu {
            Some(api) => {
                let start = (monotonic_ns() - ns(work.began.elapsed())).max(1);
                // SAFETY: the work duration is API 35's own, the values within its rules.
                unsafe {
                    (api.start)(api.work, start);
                    (api.total)(api.work, total);
                    (api.cpu)(api.work, ns(work.cpu));
                    (api.gpu)(api.work, work.gpu.map_or(0, ns));
                    (api.report)(open, api.work)
                }
            }
            // SAFETY: an open session, a positive duration.
            None => unsafe { (session.api.report)(open, total) },
        };
        session.failed("reporting", code);
    }

    fn pause(&self) {
        self.session().prefer_efficiency(true);
    }

    fn close(&self) {
        self.session().close();
    }
}

impl Drop for HintSession {
    fn drop(&mut self) {
        let mut session = self.session();
        session.close();
        if let Some(api) = &session.api.gpu {
            // SAFETY: the work duration made in `open`, released once.
            unsafe { (api.release)(api.work) };
        }
    }
}

/// Makes the hints for `runtime` at start, or logs once why there are none.
pub(crate) fn start() -> Option<Arc<HintSession>> {
    match HintSession::open() {
        Ok(hints) => {
            log::info!("performance hints: {}", hints.describe());
            Some(Arc::new(hints))
        }
        Err(why) => {
            log::info!("performance hints: none ({why})");
            None
        }
    }
}

/// The display's vsync period in ns (Kotlin reads it from the display): the hints' target.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_vsync<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    period_ns: jlong,
) {
    if let Some(hints) = host(handle).and_then(|host| host.hints.as_ref()) {
        hints.set_vsync(period_ns);
        log::info!(
            "performance hints: target {:.2} ms, the display's vsync",
            period_ns as f64 / 1e6
        );
    }
}
