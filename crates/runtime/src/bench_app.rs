//! On-device scenario benchmarks (S58, DECISIONS D85): the app's own frame loop driven the way a
//! hand drives it (the same `frame`, input as `MotionBatch`es through the normaliser, the UI and
//! the session), only without waiting for the display's vsync callbacks or a person's pace: each
//! input batch goes in as soon as the frame before was drawn. The tablet runs them through the
//! debug build (`cargo device-input bench <scenario>`, on the main thread, into the window), on
//! the artwork open (a test artwork `device-input open-file` opened); the harness's tests run
//! them offscreen. Every scenario reports its wall time, its drawn frames' work (the frame
//! call's time, p50 / p95 / max), frames skipped while the GPU was behind (S56), background work's
//! GPU time by timestamps (S53) and what render counted (`Counts`).
//!
//! - `open-cold`, `open`: the artwork left for the gallery and opened again from its card, its
//!   display cache deleted first or not: time to the canvas shown and to complete.
//! - `zoom`: from fit, pinches to 2x, 4x and 8x over the artwork's densest place, each until
//!   sharp, then back to 4x and 2x: time to sharp and tier tiles filled after each pinch.
//! - `pan`: at 4x over that place, 3 s of two-finger pans, then until sharp.
//! - `strokes` (S63): at fit, a stroke of each brush the author paints with and a smudge across
//!   that place, then the first stroke after an opening, an undo and a layer switch beside a later
//!   one: frame work, each frame's GPU time, event to present, the pen-down's and the lift's
//!   frames, the GPU caches held (`bench_app_strokes.rs`).
//! - `session`: the artwork's strokes appended as records to a new screen-size artwork as fast as
//!   frames take them, every twentieth undone and every other of those redone, a layer added
//!   where the next stroke's layer is above the last (`test-art session`'s pace).
//! - `export` (S59): the artwork exported as PNG at 1x, 2x and 4x, its bands and its encoding
//!   timed apart.
//! - `touch` (S60): a pinch to 8x over the densest place, then while the tier fills a pen
//!   touching down and later two fingers starting a pan: from each event to its first frame's
//!   start, to its present and to the GPU finishing it.
//! - `undo` (S62): the layer holding the most strokes hidden, then undone, redone and undone
//!   again: each fill's time to complete and its frames' work.

use std::time::{Duration, Instant};

use openpaint_render::{Counts, Spent, take_spent};

use crate::error::RuntimeError;
use crate::frame::Runtime;

#[path = "bench_app_cases.rs"]
mod cases;
#[path = "bench_app_export.rs"]
mod export;
#[path = "bench_app_paint.rs"]
mod paint;
#[path = "bench_app_strokes.rs"]
mod strokes;
#[path = "bench_app_touch.rs"]
mod touch;
#[path = "bench_app_undo.rs"]
mod undo;

pub use cases::busiest;
pub use strokes::STROKES_ADDED;

/// The scenarios `bench_scenario` runs.
pub const SCENARIOS: [&str; 9] = [
    "open-cold",
    "open",
    "zoom",
    "pan",
    "strokes",
    "session",
    "export",
    "touch",
    "undo",
];

/// The screen offscreen when no frame was drawn yet: the tablet's.
const OFFSCREEN: [u32; 2] = [2880, 1800];

/// The display's frame interval: the host calls a frame at each vsync it asked for (120 Hz).
const VSYNC: Duration = Duration::from_micros(8333);

/// What a stretch of frames took.
pub(crate) struct Watch {
    started: Instant,
    /// Frame calls at the display's cadence, as the host makes them (a call skipped for the GPU
    /// waits for the next vsync too), rather than as fast as the GPU takes them.
    paced: bool,
    /// Drawn frames' work, ms.
    pub(crate) frames: Vec<f64>,
    skipped: usize,
    /// Background work's GPU time measured by timestamps since the device was opened when the
    /// stretch began, ns.
    gpu: f64,
    /// Frame calls skipped for the GPU: their work (S60: background work between the frames
    /// drawn), ms.
    between: Vec<f64>,
    /// The frame loop's time evaluating the canvas and waiting for the GPU, ns.
    evaluated: u64,
    waited: u64,
    /// The slowest drawn frame, ms, and what it spent.
    slowest: (f64, Spent),
    counts: Counts,
    /// What the last frame call spent (S63: the scene the lift's frame built).
    pub(crate) last: Spent,
}

impl Watch {
    /// Frames as fast as the GPU takes them (input fed as fast as frames take it).
    pub(crate) fn start(runtime: &Runtime) -> Self {
        Self {
            started: Instant::now(),
            paced: false,
            frames: Vec::new(),
            skipped: 0,
            gpu: runtime.core.background().total,
            between: Vec::new(),
            evaluated: 0,
            waited: 0,
            slowest: (0.0, Spent::default()),
            counts: runtime.core.counts(),
            last: Spent::default(),
        }
    }

    /// Frames at the display's cadence, as a person sees them (fills and navigation), unless the
    /// scenario was asked `@fast`: then as fast as the GPU takes them.
    pub(crate) fn paced(runtime: &Runtime) -> Self {
        Self {
            paced: !runtime.bench_fast,
            ..Self::start(runtime)
        }
    }

    pub(crate) const fn is_paced(&self) -> bool {
        self.paced
    }

    pub(crate) const fn skipped(&self) -> usize {
        self.skipped
    }

    /// Waits for the next vsync after the frame call that just returned, when paced; else a
    /// millisecond after a skipped call.
    fn wait(&self, skipped: bool) {
        if self.paced {
            let since = self.started.elapsed();
            let ticks = since.as_nanos() / VSYNC.as_nanos() + 1;
            let next = VSYNC * u32::try_from(ticks).unwrap_or(u32::MAX);
            std::thread::sleep(next.saturating_sub(since));
        } else if skipped {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub(crate) fn ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1e3
    }

    /// "<wall> ms over <n> frames (<k> skipped), frame work p50/p95/max a/b/c ms, background GPU
    /// g ms, <counts>".
    pub(crate) fn line(&self, runtime: &Runtime) -> String {
        let c = runtime.core.counts().since(self.counts);
        let wall = self.ms();
        let gpu = (runtime.core.background().total - self.gpu) / 1e6;
        let ms = |ns: u64| ns as f64 / 1e6;
        let (slowest, spent) = self.slowest;
        format!(
            "{wall:.0} ms over {} frames ({} skipped, their work p50/p95/max {}), frame work \
             p50/p95/max {} (evaluating the canvas {:.0} ms, waiting for the GPU {:.0} ms; the \
             slowest {slowest:.1} ms: evaluating {:.1}, scenes {:.1}, waits {:.1} most in {}), \
             background GPU {gpu:.0} ms ({:.0} %); dispatches {}, plans {}, residuals derived {}, \
             tier tiles filled {}, raster regions {}",
            self.frames.len(),
            self.skipped,
            spread(&self.between),
            spread(&self.frames),
            ms(self.evaluated),
            ms(self.waited),
            ms(spent.evaluated_ns),
            ms(spent.scene_ns),
            ms(spent.waited_ns),
            spent.longest,
            100.0 * gpu / wall.max(1.0),
            c.dispatches,
            c.plans,
            c.derived,
            c.filled,
            c.rastered
        )
    }
}

/// "p50 p95 max" (nearest rank) in ms.
pub(crate) fn spread(values: &[f64]) -> String {
    if values.is_empty() {
        return "-".to_owned();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = |share: f64| sorted[((sorted.len() as f64 * share).ceil() as usize).max(1) - 1];
    format!("{:.1}/{:.1}/{:.1}", rank(0.5), rank(0.95), rank(1.0))
}

impl Runtime {
    /// For debug builds only (`cargo device-input bench <scenario>`, S58): runs scenario `name`
    /// (`SCENARIOS`; `open-cold`, `open`, `zoom` and `pan` at the display's cadence, or with
    /// `@fast` as fast as the GPU takes frames) on the artwork open; its report, a line per stage.
    ///
    /// # Errors
    /// `NotNow` without an artwork of the library open or for another name; as the frame loop.
    pub fn bench_scenario(&mut self, name: &str) -> Result<Vec<String>, RuntimeError> {
        let mut lines = vec![format!("{name} on {}", self.core.adapter())];
        let (name, fast) = match name.strip_suffix("@fast") {
            Some(name) => (name, true),
            None => (name, false),
        };
        self.bench_fast = fast;
        match name {
            "open-cold" => self.bench_open(true, &mut lines)?,
            "open" => self.bench_open(false, &mut lines)?,
            "zoom" => self.bench_zoom(&mut lines)?,
            "pan" => self.bench_pan(&mut lines)?,
            "strokes" => self.bench_strokes(&mut lines)?,
            "session" => self.bench_session(&mut lines)?,
            "export" => self.bench_export(&mut lines)?,
            "touch" => self.bench_touch(&mut lines)?,
            "undo" => self.bench_undo(&mut lines)?,
            _ => return Err(RuntimeError::NotNow("a scenario of SCENARIOS")),
        }
        Ok(lines)
    }

    /// One frame as the host draws it (into the window; offscreen without one), watched; a frame
    /// skipped for the GPU waits a millisecond before the next is asked for.
    pub(crate) fn bench_frame(&mut self, watch: &mut Watch) -> Result<bool, RuntimeError> {
        let again = self.bench_call(watch)?;
        watch.wait(self.paced().skipped);
        Ok(again)
    }

    /// One frame call as `bench_frame` makes it, without waiting for the next vsync after it.
    pub(crate) fn bench_call(&mut self, watch: &mut Watch) -> Result<bool, RuntimeError> {
        let started = Instant::now();
        let again = if self.surface.is_some() {
            self.frame()?
        } else {
            let size = (self.offscreen.as_ref()).map_or(OFFSCREEN, |target| target.size());
            self.frame_offscreen(size)?
        };
        let ms = started.elapsed().as_secs_f64() * 1e3;
        if self.paced().skipped {
            watch.skipped += 1;
            watch.between.push(ms);
        } else {
            watch.frames.push(ms);
        }
        let spent = take_spent();
        watch.last = spent;
        watch.evaluated += spent.evaluated_ns;
        watch.waited += spent.waited_ns;
        if !self.paced().skipped && ms > watch.slowest.0 {
            watch.slowest = (ms, spent);
        }
        Ok(again)
    }

    /// Frames until `done` holds, at most `limit` of wall time; whether it came to hold.
    pub(crate) fn bench_until(
        &mut self,
        watch: &mut Watch,
        limit: Duration,
        done: impl Fn(&Self) -> bool,
    ) -> Result<bool, RuntimeError> {
        let started = Instant::now();
        while !done(self) {
            if started.elapsed() > limit {
                return Ok(false);
            }
            self.bench_frame(watch)?;
        }
        Ok(true)
    }

    /// The canvas shows complete and still: nothing fills, warms, moves or waits for a worker.
    pub(crate) fn bench_settled(&self) -> bool {
        self.screen
            .canvas()
            .is_some_and(|canvas| !canvas.filling() && !canvas.warming() && !canvas.view_pending())
            && !self.storing()
            && !self.shell.moving()
    }
}
