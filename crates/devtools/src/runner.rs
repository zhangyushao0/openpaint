//! Runs a scenario through the product frame loop, offscreen: each gesture batch is handed to
//! the runtime and followed by one frame, as on the tablet. Checkpoints and screenshots are
//! collected into a report; a failed checkpoint does not stop the run.

use std::time::{Duration, Instant};

use openpaint_render::RgbaImage;
use openpaint_runtime::{MotionBatch, Runtime};

use crate::error::ScenarioError;
use crate::gesture;
use crate::scenario::{Scenario, Step, Tool};

/// Milliseconds of one 60 Hz frame, for the input clock.
const FRAME_MS: i64 = 16;

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

#[derive(Debug, Clone)]
pub enum Record {
    Checkpoint(Checkpoint),
    Screenshot(Screenshot),
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
            Record::Screenshot(_) => None,
        })
    }

    /// True when every checkpoint passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checkpoints().all(Checkpoint::passed)
    }
}

/// Starts the app, opens it with one frame and runs every step.
pub fn run(scenario: &Scenario) -> Result<Report, ScenarioError> {
    let started = Instant::now();
    let mut runner = Runner {
        scenario,
        runtime: Runtime::start(scenario.density)?,
        clock_ms: 0,
        frames: 0,
    };
    runner.frame()?;
    let mut records = Vec::new();
    for step in &scenario.steps {
        records.extend(runner.step(step)?);
    }
    Ok(Report {
        records,
        elapsed: started.elapsed(),
        frames: runner.frames,
        adapter: runner.runtime.adapter(),
    })
}

struct Runner<'a> {
    scenario: &'a Scenario,
    runtime: Runtime,
    /// Time of the next input sample, in milliseconds; never goes back, restarts included.
    clock_ms: i64,
    frames: u64,
}

impl Runner<'_> {
    fn step(&mut self, step: &Step) -> Result<Option<Record>, ScenarioError> {
        let now = self.clock_ms;
        match step {
            Step::Tap { at, tool } => self.gesture(&gesture::tap(*tool, *at, now))?,
            Step::Drag { from, to, tool } => {
                let points = [[from[0], from[1], 1.0], [to[0], to[1], 1.0]];
                self.gesture(&gesture::path(*tool, &points, now))?;
            }
            Step::Stroke { points } => self.gesture(&gesture::path(Tool::Stylus, points, now))?,
            Step::Wait { frames } => {
                for _ in 0..*frames {
                    self.frame()?;
                    self.clock_ms += FRAME_MS;
                }
            }
            Step::Restart => self.restart()?,
            Step::Checkpoint { name, canvas } => {
                let image = self.runtime.read_canvas()?;
                let canvas_image = image.ok_or(ScenarioError::Missing("no canvas is open"))?;
                return Ok(Some(Record::Checkpoint(Checkpoint {
                    name: name.clone(),
                    expected: *canvas,
                    canvas: canvas_image,
                })));
            }
            Step::Screenshot { name } => {
                let image = self.runtime.read_offscreen()?;
                let screen = image.ok_or(ScenarioError::Missing("no frame was drawn"))?;
                return Ok(Some(Record::Screenshot(Screenshot {
                    name: name.clone(),
                    screen,
                })));
            }
        }
        Ok(None)
    }

    /// Each batch is one platform event followed by one frame.
    fn gesture(&mut self, batches: &[MotionBatch]) -> Result<(), ScenarioError> {
        for batch in batches {
            self.runtime.motion(batch);
            self.frame()?;
        }
        self.clock_ms = gesture::end_ms(batches, self.clock_ms) + FRAME_MS;
        Ok(())
    }

    /// The process ends and a new one starts: everything the runtime held is gone. What
    /// survives is what storage keeps, and there is no storage until S7.
    fn restart(&mut self) -> Result<(), ScenarioError> {
        self.runtime = Runtime::start(self.scenario.density)?;
        self.frame()
    }

    fn frame(&mut self) -> Result<(), ScenarioError> {
        self.runtime.frame_offscreen(self.scenario.screen)?;
        self.frames += 1;
        Ok(())
    }
}
