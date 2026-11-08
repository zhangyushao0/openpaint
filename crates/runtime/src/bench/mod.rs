//! On-device scenario benchmarks (DECISIONS D85): the app's own frame loop driven the way a hand
//! drives it (the same `frame`, input as `MotionBatch`es through the normaliser, the UI and the
//! session), only without waiting for a person's pace where none is wanted. The tablet runs them
//! through the debug build (`cargo device-input bench <scenario>`, on the main thread, into the
//! window), on the artwork open (a test artwork `device-input open-file` opened); the runtime's
//! tests run them offscreen. Every scenario reports its wall time, its drawn frames' work (the
//! frame call's time, p50 / p95 / max), frames skipped while the GPU was behind, background
//! work's GPU time by timestamps and what render counted (`Counts`).
//!
//! - `open-cold`, `open`: the artwork left for the gallery and opened again from its card, its
//!   display cache deleted first or not: time to the canvas shown and to complete (`open.rs`).
//! - `zoom`: from fit, pinches to 2x, 4x and 8x over the artwork's densest place, each until
//!   sharp, then back to 4x and 2x: time to sharp and tier tiles filled after each pinch.
//! - `pan`: at 4x over that place, 3 s of two-finger pans, then until sharp (`view.rs`).
//! - `strokes`: at fit, a stroke of each brush the author paints with and a smudge across that
//!   place, then the first stroke after an opening, an undo and a layer switch beside a later
//!   one: frame work, each frame's GPU time, event to present, the pen-down's and the lift's
//!   frames, the GPU caches held (`strokes.rs`).
//! - `session`: the artwork's strokes appended as records to a new screen-size artwork as fast as
//!   frames take them, every twentieth undone and every other of those redone, a layer added
//!   where the next stroke's layer is above the last (`test-art session`'s pace).
//! - `export`: the artwork exported as PNG at 1x, 2x and 4x, its bands and its encoding timed
//!   apart.
//! - `touch`: a pinch to 8x over the densest place, then while the tier fills a pen touching
//!   down and later two fingers starting a pan: from each event to its first frame's start, to
//!   its present and to the GPU finishing it.
//! - `undo`: the layer holding the most strokes hidden, then undone, redone and undone again:
//!   each fill's time to complete and its frames' work.
//! - `adjust`: on the busiest layer a slider dragged in each adjustment and filter panel, 60
//!   values, then cancelled: frame work, values shown in their frame, each one's time to show.
//! - `ui`: the interface's motions (each popover opening and closing, panels switching, the
//!   colour wheel, a slider and the brush editor's column dragged): frames drawn against the
//!   vsyncs wanting one, the longest gap, frame work, UI pass and GPU time a frame (`ui.rs`,
//!   `motion.rs`).
//! - `practice`: on the practice sheet open, How's card opened and closed, a line compared, Next
//!   three times, a group of combined lines, a shape and a colour combination compared, a colour
//!   filled and compared, measured as `ui` measures; a line, a group and a shape compared aligned
//!   also say when the fit showed after the tap and how long it took on its worker
//!   (`practice.rs`); in stage 3 a point, an angle and a point in space compared
//!   (`practice_points.rs`).
//! - `book`: in the gallery, the practice book opened and closed twice, measured as `ui` measures
//!   (`book.rs`).

use std::time::{Duration, Instant};

use openpaint_app::Intent;
use openpaint_render::{Counts, Spent, take_spent};

use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::screen::Screen;

mod adjust;
mod book;
mod export;
mod hand;
mod motion;
mod open;
mod practice;
mod practice_division;
mod practice_masses;
mod practice_points;
mod session;
mod strokes;
mod touch;
mod ui;
mod undo;
mod view;

pub use hand::busiest;
pub use strokes::STROKES_ADDED;

/// The scenarios `bench_scenario` runs.
pub const SCENARIOS: [&str; 13] = [
    "open-cold",
    "open",
    "zoom",
    "pan",
    "strokes",
    "session",
    "export",
    "touch",
    "undo",
    "adjust",
    "ui",
    "practice",
    "book",
];

/// The screen offscreen when no frame was drawn yet: the tablet's.
const OFFSCREEN: [u32; 2] = [2880, 1800];

/// The display's frame interval: the host calls a frame at each vsync it asked for (120 Hz).
const VSYNC: Duration = Duration::from_micros(8333);

/// How long a stage may take before the benchmark gives up on it.
const LIMIT: Duration = Duration::from_secs(60);

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
    /// Frame calls skipped for the GPU: their work (background work between the frames drawn),
    /// ms.
    between: Vec<f64>,
    /// The frame loop's time drawing the canvas and waiting for the GPU, ns.
    drawn: u64,
    waited: u64,
    /// The slowest drawn frame, ms, and what it spent.
    slowest: (f64, Spent),
    counts: Counts,
    /// What the last frame call spent (the scene the lift's frame built).
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
            drawn: 0,
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
             p50/p95/max {} (drawing the canvas {:.0} ms, waiting for the GPU {:.0} ms; the \
             slowest {slowest:.1} ms: drawing {:.1}, scenes {:.1}, waits {:.1} most in {}), \
             background GPU {gpu:.0} ms ({:.0} %); residuals derived {}, \
             tier tiles filled {}, raster regions {}",
            self.frames.len(),
            self.skipped,
            spread(&self.between),
            spread(&self.frames),
            ms(self.drawn),
            ms(self.waited),
            ms(spent.drawn_ns),
            ms(spent.scene_ns),
            ms(spent.waited_ns),
            spent.longest,
            100.0 * gpu / wall.max(1.0),
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

/// "<state> after" when a stage came to its `state` within the limit, else "NOT <state> within
/// the limit:", before the stage's line.
pub(crate) fn reached(done: bool, state: &str) -> String {
    if done {
        format!("{state} after")
    } else {
        format!("NOT {state} within the limit:")
    }
}

/// Bytes in MB.
pub(crate) fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

impl Runtime {
    /// For debug builds only (`cargo device-input bench <scenario>`): runs scenario `name`
    /// (`SCENARIOS`; fills and navigation at the display's cadence, or with `@fast` as fast as
    /// the GPU takes frames) on the artwork open; its report, a line per stage.
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
            "adjust" => self.bench_adjust(&mut lines)?,
            "ui" => self.bench_ui(&mut lines)?,
            "practice" => self.bench_practice(&mut lines)?,
            "book" => self.bench_book(&mut lines)?,
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
        watch.drawn += spent.drawn_ns;
        watch.waited += spent.waited_ns;
        if !self.paced().skipped && ms > watch.slowest.0 {
            watch.slowest = (ms, spent);
        }
        Ok(again)
    }

    /// Frames until `done` holds, at most `LIMIT` of wall time; whether it came to hold.
    pub(crate) fn bench_until(
        &mut self,
        watch: &mut Watch,
        done: impl Fn(&Self) -> bool,
    ) -> Result<bool, RuntimeError> {
        let started = Instant::now();
        while !done(self) {
            if started.elapsed() > LIMIT {
                return Ok(false);
            }
            self.bench_frame(watch)?;
        }
        Ok(true)
    }

    /// Frames until one is drawn (the GPU may be behind).
    pub(crate) fn bench_drawn(&mut self, watch: &mut Watch) -> Result<(), RuntimeError> {
        let drawn = watch.frames.len();
        while watch.frames.len() == drawn {
            self.bench_frame(watch)?;
        }
        Ok(())
    }

    /// The canvas shows complete and still: nothing fills, warms, moves or waits for a worker.
    pub(crate) fn bench_settled(&self) -> bool {
        self.screen
            .canvas()
            .is_some_and(|canvas| !canvas.filling() && !canvas.warming() && !canvas.view_pending())
            && !self.storing()
            && !self.shell.moving()
    }

    /// Frames until the canvas settled, on a watch of their own.
    pub(crate) fn bench_settle(&mut self) -> Result<bool, RuntimeError> {
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, Self::bench_settled)
    }

    /// `intents` applied to the open canvas as the frame applies a hand's.
    pub(crate) fn bench_dispatch(&mut self, intents: Vec<Intent>) {
        if let Screen::Canvas(canvas) = &mut self.screen {
            let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
            canvas.dispatch(&self.core, intents, app);
        }
    }

    /// `intents` applied to the open practice sheet, then a frame drawn.
    pub(crate) fn bench_practise(
        &mut self,
        intents: Vec<openpaint_practice_app::PracticeIntent>,
        watch: &mut Watch,
    ) -> Result<(), RuntimeError> {
        if let Screen::Canvas(canvas) = &mut self.screen {
            crate::practice::apply(canvas, intents, self.frame_clock.wall_seconds());
        }
        self.bench_drawn(watch)
    }

    /// `intents` applied to the open canvas, then a frame drawn.
    pub(crate) fn bench_intents(
        &mut self,
        intents: Vec<Intent>,
        watch: &mut Watch,
    ) -> Result<(), RuntimeError> {
        self.bench_dispatch(intents);
        self.bench_drawn(watch)
    }
}
