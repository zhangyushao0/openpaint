//! Runs a scenario through the product frame loop, offscreen: each gesture batch is handed to
//! the runtime and followed by one frame, plus the frames the runtime asks for at once, as on
//! the tablet, and then frames until the storage worker is done, so every step sees the store's
//! answer. The app's library lives in a directory the caller owns; `restart` reopens it. What
//! the app asks of the system (pickers, the share sheet) waits for a `system` step to answer it
//! as the user would (`system.rs`). Checkpoints, screenshots and exported files are collected
//! into a report; a failed checkpoint does not stop the run.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use openpaint_render::RgbaImage;
use openpaint_runtime::{HostRequest, MotionBatch, Runtime, TextFonts, TextInput};

use crate::error::ScenarioError;
use crate::gesture;
use crate::scenario::{Scenario, Step};
use crate::switch::Switched;
use crate::system::{self, Export, Written};

/// Milliseconds of one 60 Hz frame, for the input clock.
const FRAME_MS: i64 = 16;
/// Most frames run after one because the runtime asked for them.
const FOLLOW_UPS: usize = 4;
/// Longest wait for the storage worker after a step.
const STORAGE_LIMIT: Duration = Duration::from_secs(10);

/// The canvas at a checkpoint and the hash it should have.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub name: String,
    pub expected: u64,
    pub canvas: RgbaImage,
}

impl Checkpoint {
    #[must_use]
    pub fn actual(&self) -> u64 {
        self.canvas.content_hash()
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.actual() == self.expected
    }
}

/// The screen of the last frame.
#[derive(Debug, Clone)]
pub struct Screenshot {
    pub name: String,
    pub screen: RgbaImage,
}

/// The view at a view checkpoint and what it should be: zoom relative to fit, rotation in
/// degrees, and pan in points when the step gives one.
#[derive(Debug, Clone)]
pub struct ViewCheck {
    pub name: String,
    pub expected: (f32, f32, Option<[f32; 2]>),
    pub actual: (f32, f32, [f32; 2]),
}

impl ViewCheck {
    /// Zoom within 1 %, rotation within half a degree, pan within a point.
    #[must_use]
    pub fn passed(&self) -> bool {
        let (zoom, rotation, pan) = self.expected;
        let (actual_zoom, actual_rotation, actual_pan) = self.actual;
        let turn = (actual_rotation - rotation + 180.0).rem_euclid(360.0) - 180.0;
        (actual_zoom / zoom - 1.0).abs() <= 0.01
            && turn.abs() <= 0.5
            && pan.is_none_or(|[x, y]| {
                (actual_pan[0] - x).abs() <= 1.0 && (actual_pan[1] - y).abs() <= 1.0
            })
    }
}

#[derive(Debug, Clone)]
pub enum Record {
    Checkpoint(Checkpoint),
    Screenshot(Screenshot),
    Export(Export),
    View(ViewCheck),
}

#[derive(Debug, Clone)]
pub struct Report {
    /// In step order.
    pub records: Vec<Record>,
    /// From starting the runtime (GPU device included) to the last step.
    pub elapsed: Duration,
    pub frames: u64,
    pub adapter: String,
}

impl Report {
    pub fn checkpoints(&self) -> impl Iterator<Item = &Checkpoint> {
        self.records.iter().filter_map(|record| match record {
            Record::Checkpoint(checkpoint) => Some(checkpoint),
            _ => None,
        })
    }

    pub fn exports(&self) -> impl Iterator<Item = &Export> {
        self.records.iter().filter_map(|record| match record {
            Record::Export(export) => Some(export),
            _ => None,
        })
    }

    pub fn views(&self) -> impl Iterator<Item = &ViewCheck> {
        self.records.iter().filter_map(|record| match record {
            Record::View(view) => Some(view),
            _ => None,
        })
    }

    /// True when every checkpoint, export and view checkpoint passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checkpoints().all(Checkpoint::passed)
            && self.exports().all(Export::passed)
            && self.views().all(ViewCheck::passed)
    }
}

/// Starts the app on the library in `library` (created when missing), with the text faces a
/// host would hand over, opens it and runs every step.
pub fn run(
    scenario: &Scenario,
    library: &Path,
    fonts: &TextFonts,
) -> Result<Report, ScenarioError> {
    let started = Instant::now();
    let mut runner = Runner {
        scenario,
        library,
        fonts,
        storage_off: Arc::new(AtomicBool::new(false)),
        clock_ms: 0,
        frames: 0,
        requests: Vec::new(),
        written: Written::default(),
    };
    let mut runtime = runner.start()?;
    let mut records = Vec::new();
    for step in &scenario.steps {
        if *step == Step::Restart {
            // The user leaves (the host's `onPause`), then the system ends the process: what
            // the leave handed over is written and the library closes before the new start.
            runtime.leave();
            drop(runtime);
            runner.requests.clear();
            runtime = runner.start()?;
            continue;
        }
        records.extend(runner.step(&mut runtime, step)?);
    }
    Ok(Report {
        records,
        elapsed: started.elapsed(),
        frames: runner.frames,
        adapter: runtime.adapter(),
    })
}

struct Runner<'a> {
    scenario: &'a Scenario,
    library: &'a Path,
    /// The text faces a host would hand over, given to every start.
    fonts: &'a TextFonts,
    /// Set by `storage_fails`, cleared by `storage_works`; survives restarts.
    storage_off: Arc<AtomicBool>,
    /// Time of the next input sample, in milliseconds; never goes back, restarts included.
    clock_ms: i64,
    frames: u64,
    /// What the app asked the system for and the user has not answered yet.
    requests: Vec<HostRequest>,
    /// The file the last export wrote.
    written: Written,
}

impl Runner<'_> {
    /// A new app process on the library: everything a runtime held before is gone, history
    /// included; what survives is what the library stores. Runs frames until it has opened.
    fn start(&mut self) -> Result<Runtime, ScenarioError> {
        let store = Switched::new(self.library, Arc::clone(&self.storage_off));
        let mut runtime = Runtime::start(self.scenario.density, Box::new(store))?;
        runtime.set_text_fonts(self.fonts.clone());
        self.frame(&mut runtime)?;
        Ok(runtime)
    }

    fn step(
        &mut self,
        runtime: &mut Runtime,
        step: &Step,
    ) -> Result<Option<Record>, ScenarioError> {
        let now = self.clock_ms;
        match step {
            Step::Tap { at, tool } => self.gesture(runtime, &gesture::tap(*tool, *at, now))?,
            Step::Hover { at } => self.gesture(runtime, &gesture::hover(*at, now))?,
            Step::Drag { from, to, tool } => {
                let points = [[from[0], from[1], 1.0], [to[0], to[1], 1.0]];
                self.gesture(runtime, &gesture::path(*tool, &points, now))?;
            }
            Step::Press {
                path,
                tool,
                pressure,
                before_lift,
            } => {
                let batches = gesture::press(*tool, path, *pressure, now);
                return self.press(runtime, &batches, before_lift.as_ref());
            }
            Step::Stroke { points, lift, palm } => {
                let batches = gesture::stroke(points, *lift, *palm, now);
                self.gesture(runtime, &batches)?;
            }
            Step::Touch { from, to, ms } => {
                let to = to.as_ref().unwrap_or(from);
                self.gesture(runtime, &gesture::fingers(from, to, *ms, now))?;
            }
            Step::View { .. } => return Ok(view_check(runtime, step)),
            Step::Type(text) => {
                runtime.text(TextInput::Commit(text.clone()));
                self.frame(runtime)?;
            }
            Step::Back => {
                if !runtime.back() {
                    return Err(ScenarioError::Missing("back was not used by the app"));
                }
                self.frame(runtime)?;
            }
            Step::Wait { frames } => {
                for _ in 0..*frames {
                    self.frame(runtime)?;
                    self.clock_ms += FRAME_MS;
                }
            }
            Step::Restart => return Err(ScenarioError::Missing("restart is run by `run`")),
            Step::StorageFails => self.storage_off.store(true, Ordering::Relaxed),
            Step::StorageWorks => self.storage_off.store(false, Ordering::Relaxed),
            Step::System(answer) => {
                if let Some(answer) = system::answer(&mut self.requests, *answer, &self.written)? {
                    runtime.answer(answer);
                    self.frame(runtime)?;
                }
            }
            Step::Checkpoint { .. } | Step::Screenshot { .. } | Step::Exported { .. } => {
                return record(runtime, step, &self.written).map(Some);
            }
        }
        Ok(None)
    }

    /// A press's batches; with `before_lift`, a screenshot of that name before the lift.
    fn press(
        &mut self,
        runtime: &mut Runtime,
        batches: &[MotionBatch],
        before_lift: Option<&String>,
    ) -> Result<Option<Record>, ScenarioError> {
        let Some(name) = before_lift else {
            self.gesture(runtime, batches)?;
            return Ok(None);
        };
        let (lift, held) = batches
            .split_last()
            .ok_or(ScenarioError::Missing("a press has no samples"))?;
        self.gesture(runtime, held)?;
        let step = Step::Screenshot { name: name.clone() };
        let shot = record(runtime, &step, &self.written)?;
        self.gesture(runtime, std::slice::from_ref(lift))?;
        Ok(Some(shot))
    }

    /// Each batch is one platform event followed by one frame.
    fn gesture(
        &mut self,
        runtime: &mut Runtime,
        batches: &[MotionBatch],
    ) -> Result<(), ScenarioError> {
        for batch in batches {
            runtime.motion(batch);
            self.frame(runtime)?;
        }
        self.clock_ms = gesture::end_ms(batches, self.clock_ms) + FRAME_MS;
        Ok(())
    }

    /// One frame and the frames the runtime then asks for at once (a button that changed what
    /// the UI offers, egui's own layout), as the host's frame pacing gives them, up to
    /// `FOLLOW_UPS` more; again while a worker (storage, transfer) is busy, until it is done.
    /// Then the host takes what the app asks of the system.
    fn frame(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        let screen = self.scenario.screen;
        let start = Instant::now();
        loop {
            // The frame that takes a worker's result may ask for more (a window laid out).
            for _ in 0..=FOLLOW_UPS {
                let again = runtime.frame_offscreen(screen)?;
                self.frames += 1;
                if !again {
                    break;
                }
            }
            if !runtime.storing() {
                break;
            }
            if start.elapsed() > STORAGE_LIMIT {
                return Err(ScenarioError::Missing("the storage worker did not finish"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // The host takes what the app asks of the system after each frame.
        self.requests
            .extend(std::iter::from_fn(|| runtime.host_request()));
        Ok(())
    }
}

/// The view a view step looks at.
fn view_check(runtime: &Runtime, step: &Step) -> Option<Record> {
    let Step::View {
        name,
        zoom,
        rotation,
        pan,
    } = step
    else {
        return None;
    };
    let view = runtime.projection().view();
    Some(Record::View(ViewCheck {
        name: name.clone(),
        expected: (*zoom, *rotation, *pan),
        actual: (view.zoom(), view.rotation().to_degrees(), view.pan()),
    }))
}

/// What a checkpoint, screenshot or export step looks at.
fn record(runtime: &Runtime, step: &Step, written: &Written) -> Result<Record, ScenarioError> {
    let canvas = || {
        runtime
            .read_canvas()?
            .ok_or(ScenarioError::Missing("no canvas is open"))
    };
    Ok(match step {
        Step::Checkpoint { name, canvas: hash } => Record::Checkpoint(Checkpoint {
            name: name.clone(),
            expected: *hash,
            canvas: canvas()?,
        }),
        Step::Exported { name } => {
            let bytes = written
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take();
            let bytes = bytes.ok_or(ScenarioError::Missing("no export was written"))?;
            Record::Export(Export::of(name, bytes, &canvas()?)?)
        }
        Step::Screenshot { name } => {
            let image = runtime.read_offscreen()?;
            let screen = image.ok_or(ScenarioError::Missing("no frame was drawn"))?;
            Record::Screenshot(Screenshot {
                name: name.clone(),
                screen,
            })
        }
        _ => return Err(ScenarioError::Missing("not a step that records")),
    })
}
