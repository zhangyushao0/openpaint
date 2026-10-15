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
//! - `strokes`: at fit, a stroke of each texture brush across that place, the GPU waited for after
//!   every frame: frame work, GPU and both, the pen-down's and the lift's frames.
//! - `session`: the artwork's strokes appended as records to a new screen-size artwork as fast as
//!   frames take them, every twentieth undone and every other of those redone, a layer added
//!   where the next stroke's layer is above the last (`test-art session`'s pace).

use std::time::{Duration, Instant};

use openpaint_render::Counts;

use crate::error::RuntimeError;
use crate::frame::Runtime;

#[path = "bench_app_cases.rs"]
mod cases;
#[path = "bench_app_paint.rs"]
mod paint;

pub use cases::busiest;

/// The scenarios `bench_scenario` runs.
pub const SCENARIOS: [&str; 6] = ["open-cold", "open", "zoom", "pan", "strokes", "session"];

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
    /// Background work's GPU time measured by timestamps, ms.
    gpu: f64,
    counts: Counts,
}

impl Watch {
    /// Frames as fast as the GPU takes them (input fed as fast as frames take it).
    pub(crate) fn start(runtime: &Runtime) -> Self {
        Self {
            started: Instant::now(),
            paced: false,
            frames: Vec::new(),
            skipped: 0,
            gpu: 0.0,
            counts: runtime.core.counts(),
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
        format!(
            "{wall:.0} ms over {} frames ({} skipped), frame work p50/p95/max {}, background GPU \
             {:.0} ms ({:.0} %); dispatches {}, plans {}, residuals derived {}, tier tiles filled {}",
            self.frames.len(),
            self.skipped,
            spread(&self.frames),
            self.gpu,
            100.0 * self.gpu / wall.max(1.0),
            c.dispatches,
            c.plans,
            c.derived,
            c.filled
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
            _ => return Err(RuntimeError::NotNow("a scenario of SCENARIOS")),
        }
        Ok(lines)
    }

    /// One frame as the host draws it (into the window; offscreen without one), watched; a frame
    /// skipped for the GPU waits a millisecond before the next is asked for.
    pub(crate) fn bench_frame(&mut self, watch: &mut Watch) -> Result<bool, RuntimeError> {
        let started = Instant::now();
        let again = if self.surface.is_some() {
            self.frame()?
        } else {
            let size = (self.offscreen.as_ref()).map_or(OFFSCREEN, |target| target.size());
            self.frame_offscreen(size)?
        };
        let ms = started.elapsed().as_secs_f64() * 1e3;
        let skipped = self.paced().skipped;
        if skipped {
            // A skipped call began no frame: the background readings are the last frame's.
            watch.skipped += 1;
        } else {
            watch.frames.push(ms);
            if let Some(ns) = self.core.background().measured {
                watch.gpu += ns / 1e6;
            }
        }
        watch.wait(skipped);
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
