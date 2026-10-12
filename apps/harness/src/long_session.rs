//! `long-session` (S50): an hour of painting at the author's pace through the whole frame loop,
//! on a library of its own (`artifacts/harness/long-session-library`), to show that nothing
//! grows without bound. 3,000 strokes over 60 simulated minutes, one every 1.2 s (the author's
//! strokes hold about 342 samples at about 300 a second, STATUS) on a screen-size artwork at fit:
//! the ten built-in brushes in rotation, ten strokes each, at their default sizes; every tenth
//! stroke a smudge; one stroke in twenty undone, every other one of those redone; the layer
//! panel opened every 50 strokes until its pictures are made, a layer added every 500; the app
//! left (`onPause`) every 5 minutes, which keeps a version. Every 5 simulated minutes a row: the
//! process's private memory, the arena (smudge residuals), the canvas's GPU caches, the history,
//! the library on disk and the versions kept, each checked against its bound (`BOUNDS`). At the
//! end the canvas must equal a cold evaluation of the artwork. Run it with `--release`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use openpaint_app::{HISTORY_BUDGET, KEPT_VERSIONS};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::launch::fresh_library;
use crate::long_session_store::Watched;
use crate::output::workspace_root;

/// The MovinkPad 14.
const SIZE: [u32; 2] = [2880, 1800];
const PIXELS_PER_POINT: f32 = 1.75;
pub const STROKES: usize = 3000;
/// Simulated milliseconds per stroke: 3,000 in an hour.
const STROKE_MS: i64 = 1200;
/// Samples per stroke and their spacing in window pixels (the author's calm pace).
const SAMPLES: usize = 342;
const SPACING: f32 = 2.5;
/// Samples handed over per frame.
const PER_BATCH: usize = 10;
/// Strokes per report and per leave: 5 simulated minutes.
const EVERY_FIVE_MINUTES: usize = 250;
/// The built-ins in rotation, each with the set its picker shows it in.
const BRUSHES: [(&str, &str); 10] = [
    ("Pencil", "Sketching"),
    ("Ink Pen", "Inking"),
    ("Inker", "Inking"),
    ("Monoline", "Inking"),
    ("Hard Round", "Painting"),
    ("Soft Brush", "Painting"),
    ("Flat", "Painting"),
    ("Opaque Paint", "Painting"),
    ("Textured Round", "Texture"),
    ("Speckle", "Texture"),
];

/// What each row is checked against, in bytes: the history's budget and the versions kept, the
/// GPU's budget on a device with memory of its own for the arena and the caches, the others
/// stated from the first run (2026-10-12, STATUS: the process 4.2 GB, the library 277 MB at the
/// end) with room.
const BOUNDS: Bounds = Bounds {
    memory: 5 << 30,
    arena: 2 << 30,
    caches: 2 << 30,
    history: HISTORY_BUDGET as u64,
    library: 512 << 20,
    versions: KEPT_VERSIONS,
};

struct Bounds {
    memory: u64,
    arena: u64,
    caches: u64,
    history: u64,
    library: u64,
    versions: usize,
}

pub fn run(strokes: usize) -> Result<(), HarnessError> {
    let mut painter = Painter::start()?;
    println!("adapter {}", painter.runtime.adapter());
    println!(
        "{:>4} {:>7} {:>10} {:>9} {:>10} {:>11} {:>11} {:>8} {:>7}",
        "min",
        "strokes",
        "memory MB",
        "arena MB",
        "caches MB",
        "history MB",
        "library MB",
        "versions",
        "wall s"
    );
    painter.report()?;
    for k in 0..strokes {
        painter.step(k)?;
        if (k + 1).is_multiple_of(EVERY_FIVE_MINUTES) {
            painter.runtime.leave();
            painter.frames(1)?;
            painter.report()?;
        }
    }
    painter.finish()
}

/// The app being painted in, and the hand's clock.
struct Painter {
    runtime: Runtime,
    /// The simulated time in milliseconds: the input's and the frames'.
    clock: i64,
    versions: Arc<Mutex<usize>>,
    library: PathBuf,
    started: Instant,
}

impl Painter {
    fn start() -> Result<Self, HarnessError> {
        let dir = workspace_root()
            .join("artifacts")
            .join("harness")
            .join("long-session-library");
        let library = fresh_library(dir)?;
        let versions = Arc::new(Mutex::new(0));
        let store = Watched {
            inner: Library::at(&library),
            versions: Arc::clone(&versions),
        };
        let mut painter = Self {
            runtime: Runtime::start(PIXELS_PER_POINT, Box::new(store))?,
            clock: 0,
            versions,
            library,
            started: Instant::now(),
        };
        let opened = Instant::now();
        while painter.runtime.artwork().is_none() {
            if opened.elapsed().as_secs() > 10 {
                return Err(HarnessError::Missing("the library did not open"));
            }
            painter.frames(1)?;
        }
        painter.settle()?;
        Ok(painter)
    }

    /// Stroke `k` and what comes with it: a brush chosen, a smudge, undo and redo, pictures.
    fn step(&mut self, k: usize) -> Result<(), HarnessError> {
        if k.is_multiple_of(10) {
            self.choose(BRUSHES[(k / 10) % BRUSHES.len()])?;
        }
        let smudge = k % 10 == 9;
        if smudge {
            self.tap("Smudge")?;
        }
        self.clock = self.clock.max(k as i64 * STROKE_MS);
        self.stroke(k)?;
        if smudge {
            self.tap("Brushes")?;
        }
        if k % 20 == 19 {
            self.tap("Undo")?;
            if k % 40 == 39 {
                self.tap("Redo")?;
            }
        }
        if k % 50 == 49 {
            self.pictures(k % 500 == 499)?;
        }
        Ok(())
    }

    /// The brush picker opened, the brush's set and the brush tapped, the picker closed.
    fn choose(&mut self, (brush, set): (&str, &str)) -> Result<(), HarnessError> {
        for name in ["Brushes", set, brush, "Brushes"] {
            self.tap(name)?;
        }
        Ok(())
    }

    /// The layer panel opened until its pictures are made (with a layer added first), closed.
    fn pictures(&mut self, add: bool) -> Result<(), HarnessError> {
        self.tap("Layers")?;
        if add {
            self.tap("Add layer")?;
        }
        // The pictures are made on their worker; the hand waits without the clock moving on.
        self.runtime.fix_clock(self.clock);
        let waited = Instant::now();
        loop {
            self.runtime.frame_offscreen(SIZE)?;
            if !self.runtime.storing() {
                return self.tap("Layers");
            }
            if waited.elapsed().as_secs() > 60 {
                return Err(HarnessError::Missing("the layer pictures were never made"));
            }
        }
    }

    /// One pen stroke, a batch of `PER_BATCH` samples a frame: a gentle curve from a place and
    /// direction of its own, pressure rising and falling, about 3.3 ms between samples.
    fn stroke(&mut self, k: usize) -> Result<(), HarnessError> {
        let samples = path(k, self.clock);
        let last = samples.len() - 1;
        let mut batches = vec![(MotionPhase::Down, &samples[..1])];
        batches.extend(
            samples[1..last]
                .chunks(PER_BATCH)
                .map(|chunk| (MotionPhase::Move, chunk)),
        );
        batches.push((MotionPhase::Up, &samples[last..]));
        for (phase, chunk) in batches {
            self.clock = chunk.last().map_or(self.clock, |sample| sample.time_ms);
            self.send(phase, ToolKind::Stylus, chunk)?;
        }
        Ok(())
    }

    /// A finger tap on the middle of the control `name` of the last frame.
    fn tap(&mut self, name: &str) -> Result<(), HarnessError> {
        let control = self.runtime.controls().find(name).map_err(|_| {
            println!(
                "no single control {name}: {:?}",
                self.runtime.controls().names()
            );
            HarnessError::Missing("a control to tap is not on screen")
        })?;
        let at = control.at([0.5, 0.5]);
        let touch = |time_ms| PointerSample {
            position: at,
            pressure: 1.0,
            tilt: 0.0,
            orientation: 0.0,
            time_ms,
        };
        self.clock += 16;
        self.send(MotionPhase::Down, ToolKind::Finger, &[touch(self.clock)])?;
        self.clock += 40;
        self.send(MotionPhase::Up, ToolKind::Finger, &[touch(self.clock)])
    }

    /// One batch, its frame and the frames the runtime asks for at once (at most four).
    fn send(
        &mut self,
        phase: MotionPhase,
        tool: ToolKind,
        samples: &[PointerSample],
    ) -> Result<(), HarnessError> {
        self.runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples: samples.to_vec(),
            }],
        });
        self.frames(5)
    }

    /// Up to `most` frames at the clock, fewer when the runtime asks for none.
    fn frames(&mut self, most: usize) -> Result<(), HarnessError> {
        self.runtime.fix_clock(self.clock);
        for _ in 0..most {
            if !self.runtime.frame_offscreen(SIZE)? {
                break;
            }
        }
        Ok(())
    }

    /// Frames 16 ms apart until nothing moves, fills in or waits for a worker.
    fn settle(&mut self) -> Result<(), HarnessError> {
        for _ in 0..2000 {
            self.clock += 16;
            self.runtime.fix_clock(self.clock);
            let again = self.runtime.frame_offscreen(SIZE)?;
            if !again && !self.runtime.moving() && !self.runtime.storing() {
                return Ok(());
            }
        }
        Err(HarnessError::Missing("the app did not come to rest"))
    }
}

#[path = "long_session_report.rs"]
mod report;
use report::path;
