//! `nav-timing <library dir>` (S53): what navigation costs on a real artwork. Opens the library
//! in DIR through the app (point it at a copy: it opens as the app does) on the tablet's screen,
//! the last artwork the library opened, then, a frame every 8 ms of the fixed clock (120 Hz):
//! the fresh start until the canvas is complete; a pinch to about 2 screen pixels per canvas
//! pixel over the artwork's busiest place (where most strokes cross) and the frames until the
//! 2x tier is complete; a pinch on to about 4 and its frames; 10 s of two-finger pans at 4x;
//! a pinch back to 2 and pen strokes over the busiest place while that tier fills. Each frame's
//! CPU time (the frame) and GPU time (waited for after it), and what render counted
//! (`Counts`: dispatches, op entries, plans binned and their entries, smudge residuals
//! derived, tier tiles filled); per phase p50 / p95 / max and the sums. On a desktop GPU the
//! times are small; the counts show what thrashes. Run with `--release`.

use std::path::Path;
use std::time::{Duration, Instant};

use openpaint_core::{Artwork, StrokeTool};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind, busiest,
};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::launch::text_fonts;

/// The MovinkPad 14.
const SIZE: [u32; 2] = [2880, 1800];
const PIXELS_PER_POINT: f32 = 1.75;
/// The fixed clock's step: a 120 Hz frame.
const FRAME_MS: i64 = 8;
/// Frames at most until the app rests after a step.
const SETTLE_FRAMES: usize = 20_000;

pub fn run(dir: &str) -> Result<(), HarnessError> {
    if !Path::new(dir).is_dir() {
        return Err(HarnessError::Missing(
            "nav-timing needs a library directory",
        ));
    }
    let mut runtime = Runtime::start(PIXELS_PER_POINT, Box::new(Library::at(dir)))?;
    runtime.set_text_fonts(text_fonts());
    println!("adapter {}", runtime.adapter());
    let mut nav = Nav {
        runtime,
        clock: 0,
        view: View::default(),
        frames: Vec::new(),
    };
    let opened = Instant::now();
    while nav.runtime.artwork().is_none() {
        if opened.elapsed() > Duration::from_secs(30) {
            return Err(HarnessError::Missing("the library did not open"));
        }
        nav.frame()?;
    }
    let artwork = nav
        .runtime
        .artwork()
        .cloned()
        .ok_or(HarnessError::Missing("no artwork"))?;
    let place = busiest(&artwork);
    let canvas = artwork.canvas();
    println!(
        "artwork {} x {}, {} strokes ({} smudges); busiest place {:.0}, {:.0}",
        canvas.width(),
        canvas.height(),
        count(&artwork, |_| true),
        count(&artwork, |tool| tool == StrokeTool::Smudge),
        place[0],
        place[1]
    );
    nav.view = View::fit(&nav.runtime, [canvas.width(), canvas.height()])?;
    report::header();
    // The frames that opened the library and the canvas count to the open.
    nav.settle()?;
    nav.report("open (canvas fills)");
    nav.pinch(place, 2.0)?;
    nav.report("pinch to 2x");
    nav.settle()?;
    nav.report("2x tier fills");
    nav.pinch(place, 4.0)?;
    nav.report("pinch to 4x");
    nav.settle()?;
    nav.report("4x tier fills");
    nav.pan(10_000)?;
    nav.report("10 s of pans at 4x");
    nav.settle()?;
    nav.report("after the pans");
    nav.pinch(place, 2.0)?;
    nav.report("pinch back to 2x");
    nav.strokes(place)?;
    nav.report("pen strokes while 2x fills");
    nav.settle()?;
    nav.report("after the strokes");
    Ok(())
}

/// The app being navigated, the hand's clock and where the canvas shows.
struct Nav {
    runtime: Runtime,
    clock: i64,
    view: View,
    frames: Vec<report::Frame>,
}

/// Where the canvas shows: screen = canvas x scale + offset (upright; pinches and pans only).
#[derive(Debug, Clone, Copy, Default)]
struct View {
    scale: f32,
    offset: [f32; 2],
}

impl View {
    /// The fit `runtime`'s canvas area gives a canvas of `size` (`openpaint_app::FIT_MARGIN`).
    fn fit(runtime: &Runtime, size: [u32; 2]) -> Result<Self, HarnessError> {
        let area = runtime
            .canvas_area()
            .ok_or(HarnessError::Missing("no canvas area"))?
            .fit;
        let [w, h] = area.size.map(|side| side as f32);
        let [cw, ch] = size.map(|side| side as f32);
        let scale = (w / cw).min(h / ch) * openpaint_app::FIT_MARGIN;
        let [x, y] = area.origin.map(|at| at as f32);
        Ok(Self {
            scale,
            offset: [
                x + w / 2.0 - cw / 2.0 * scale,
                y + h / 2.0 - ch / 2.0 * scale,
            ],
        })
    }

    fn screen(self, canvas: [f32; 2]) -> [f32; 2] {
        [0, 1].map(|axis| canvas[axis] * self.scale + self.offset[axis])
    }
}

impl Nav {
    /// One frame at the clock, its CPU and GPU time and what render counted.
    fn frame(&mut self) -> Result<bool, HarnessError> {
        self.clock += FRAME_MS;
        self.runtime.fix_clock(self.clock);
        let before = self.runtime.core().counts();
        let started = Instant::now();
        let again = self.runtime.frame_offscreen(SIZE)?;
        let cpu = started.elapsed();
        let waited = Instant::now();
        self.runtime.wait_idle()?;
        let gpu = waited.elapsed();
        let counts = self.runtime.core().counts().since(before);
        let background = self.runtime.core().background();
        self.frames.push(report::Frame {
            cpu,
            gpu,
            counts,
            background,
        });
        Ok(again)
    }

    /// Frames until nothing moves, fills in or waits for a worker.
    fn settle(&mut self) -> Result<(), HarnessError> {
        for _ in 0..SETTLE_FRAMES {
            let again = self.frame()?;
            if !again && !self.runtime.moving() && !self.runtime.storing() {
                return Ok(());
            }
        }
        Err(HarnessError::Missing("the app did not come to rest"))
    }

    fn report(&mut self, phase: &str) {
        report::phase(phase, &self.frames);
        self.frames.clear();
    }

    /// Two fingers 300 px apart about `place` move apart until the canvas shows `zoom` screen
    /// pixels per canvas pixel, over 0.4 s, and lift.
    fn pinch(&mut self, place: [f32; 2], zoom: f32) -> Result<(), HarnessError> {
        let ratio = zoom / self.view.scale;
        let middle = self.view.screen(place);
        let half = 150.0;
        let from = [[middle[0] - half, middle[1]], [middle[0] + half, middle[1]]];
        let to = [
            [middle[0] - half * ratio, middle[1]],
            [middle[0] + half * ratio, middle[1]],
        ];
        self.fingers(from, to, 400)?;
        self.view.scale *= ratio;
        self.view.offset =
            [0, 1].map(|axis| middle[axis] - (middle[axis] - self.view.offset[axis]) * ratio);
        Ok(())
    }

    /// Two-finger pans for `ms`: 600 px to the left, back, up and back, again and again.
    fn pan(&mut self, ms: i64) -> Result<(), HarnessError> {
        let start = self.clock;
        let moves = [[-600.0, 0.0], [600.0, 0.0], [0.0, -400.0], [0.0, 400.0]];
        let mut k = 0;
        while self.clock - start < ms {
            let by = moves[k % moves.len()];
            let at = [SIZE[0] as f32 / 2.0 - 150.0, SIZE[1] as f32 / 2.0];
            let from = [at, [at[0] + 300.0, at[1]]];
            let to = from.map(|p| [p[0] + by[0], p[1] + by[1]]);
            self.fingers(from, to, 500)?;
            self.view.offset = [0, 1].map(|axis| self.view.offset[axis] + by[axis]);
            k += 1;
        }
        Ok(())
    }

    /// Fingers down at `from`, moved straight to `to` over `ms` with a frame each 8 ms, lifted.
    fn fingers(
        &mut self,
        from: [[f32; 2]; 2],
        to: [[f32; 2]; 2],
        ms: i64,
    ) -> Result<(), HarnessError> {
        let steps = (ms / FRAME_MS).max(1);
        let at = |k: i64| -> [[f32; 2]; 2] {
            let t = k as f32 / steps as f32;
            [0, 1].map(|f| [0, 1].map(|axis| from[f][axis] + (to[f][axis] - from[f][axis]) * t))
        };
        self.send(MotionPhase::Down, ToolKind::Finger, &at(0)[..1])?;
        self.send(
            MotionPhase::PointerDown { index: 1 },
            ToolKind::Finger,
            &at(0),
        )?;
        for k in 1..=steps {
            self.send(MotionPhase::Move, ToolKind::Finger, &at(k))?;
        }
        self.send(
            MotionPhase::PointerUp { index: 1 },
            ToolKind::Finger,
            &at(steps),
        )?;
        self.send(MotionPhase::Up, ToolKind::Finger, &at(steps)[..1])
    }

    /// Three pen strokes across `place`, a sample a frame.
    fn strokes(&mut self, place: [f32; 2]) -> Result<(), HarnessError> {
        let middle = self.view.screen(place);
        for row in [-120.0, 0.0, 120.0] {
            let path: Vec<[f32; 2]> = (0..=60)
                .map(|k| {
                    [
                        middle[0] - 450.0 + 15.0 * k as f32,
                        middle[1] + row + 40.0 * (k as f32 * 0.2).sin(),
                    ]
                })
                .collect();
            self.send(MotionPhase::Down, ToolKind::Stylus, &path[..1])?;
            for point in &path[1..path.len() - 1] {
                self.send(
                    MotionPhase::Move,
                    ToolKind::Stylus,
                    std::slice::from_ref(point),
                )?;
            }
            self.send(MotionPhase::Up, ToolKind::Stylus, &path[path.len() - 1..])?;
        }
        Ok(())
    }

    /// One batch of one sample per pointer at the clock, and its frame.
    fn send(
        &mut self,
        phase: MotionPhase,
        tool: ToolKind,
        points: &[[f32; 2]],
    ) -> Result<(), HarnessError> {
        let time_ms = self.clock + FRAME_MS;
        let pointers = (0i32..)
            .zip(points)
            .map(|(id, position)| Pointer {
                id,
                tool,
                samples: vec![PointerSample {
                    position: *position,
                    pressure: if tool == ToolKind::Stylus { 0.8 } else { 1.0 },
                    tilt: 0.0,
                    orientation: 0.0,
                    time_ms,
                }],
            })
            .collect();
        self.runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers,
        });
        self.frame().map(drop)
    }
}

/// Strokes of `artwork` whose tool `keep` keeps.
fn count(artwork: &Artwork, keep: impl Fn(StrokeTool) -> bool) -> usize {
    (artwork.layers().iter())
        .flat_map(|layer| layer.strokes().iter())
        .filter(|stroke| keep(stroke.style().tool()))
        .count()
}

#[path = "nav_timing_report.rs"]
mod report;
