//! Runs a scenario through the product frame loop, offscreen: each gesture batch is handed to
//! the runtime and followed by one frame, plus the frames the runtime asks for at once, as on
//! the tablet, and then frames until the storage worker is done, so every step sees the store's
//! answer. The app's library lives in a directory the caller owns; `restart` reopens it. What
//! the app asks of the system (pickers, the share sheet) waits for a `system` step to answer it
//! as the user would (`system.rs`). Checkpoints, screenshots and exported files are collected
//! into a report; a failed checkpoint does not stop the run.
//!
//! Motion (D44) runs on a clock the runner holds: the time of the input being handed over, then
//! one 60 Hz frame further per frame without input. Input steps never wait for motion (the hand
//! acts on the final state at once); a step that looks (a checkpoint, a screenshot, a view, an
//! export) first lets every motion settle, unless a screenshot asks for a moment `after_ms` the
//! last input.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use openpaint_runtime::{HostRequest, MotionBatch, Runtime, TextFonts, TextInput};

use crate::error::ScenarioError;
use crate::gesture;
use crate::report::{Record, Report, record, view_check};
use crate::scenario::{Scenario, Step};
use crate::switch::Switched;
use crate::system::{self, Written};

/// Milliseconds of one 60 Hz frame, for the input clock.
const FRAME_MS: i64 = 16;
/// Most frames run after one because the runtime asked for them.
const FOLLOW_UPS: usize = 4;
/// Longest wait for the storage worker after a step.
const STORAGE_LIMIT: Duration = Duration::from_secs(10);
/// Most frames motion may take to settle before a step looks.
const SETTLE_FRAMES: usize = 240;

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
        input_ms: 0,
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
    /// The runner's clock in milliseconds: the time of the next input sample and of the frames;
    /// never goes back, restarts included.
    clock_ms: i64,
    /// When the last input step's last batch was handed over.
    input_ms: i64,
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
            Step::View { .. }
            | Step::Checkpoint { .. }
            | Step::Screenshot { .. }
            | Step::Exported { .. } => return self.look(runtime, step),
            Step::Type(text) => {
                runtime.text(TextInput::Commit(text.clone()));
                self.frame(runtime)?;
            }
            Step::Back => {
                if !runtime.back() {
                    return Err(ScenarioError::Missing("back was not used by the app"));
                }
                self.frame(runtime)?;
                self.input_ms = self.clock_ms;
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
                    // What the answer brings (a picture, a notice) appears from here.
                    self.input_ms = self.clock_ms;
                }
            }
        }
        Ok(None)
    }

    /// A step that looks: once motion settled, or a screenshot `after_ms` the last input.
    fn look(
        &mut self,
        runtime: &mut Runtime,
        step: &Step,
    ) -> Result<Option<Record>, ScenarioError> {
        match step {
            Step::Screenshot {
                after_ms: Some(ms), ..
            } => self.advance_to(runtime, self.input_ms + i64::from(*ms))?,
            _ => self.settle(runtime)?,
        }
        if matches!(step, Step::View { .. }) {
            return Ok(view_check(runtime, step));
        }
        record(runtime, step, &self.written).map(Some)
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
        let step = Step::Screenshot {
            name: name.clone(),
            after_ms: None,
        };
        let shot = record(runtime, &step, &self.written)?;
        self.gesture(runtime, std::slice::from_ref(lift))?;
        Ok(Some(shot))
    }

    /// Each batch is one platform event followed by one frame at the batch's time.
    fn gesture(
        &mut self,
        runtime: &mut Runtime,
        batches: &[MotionBatch],
    ) -> Result<(), ScenarioError> {
        for batch in batches {
            let at = gesture::end_ms(std::slice::from_ref(batch), self.clock_ms);
            self.clock_ms = self.clock_ms.max(at);
            runtime.motion(batch);
            self.frame(runtime)?;
        }
        self.input_ms = self.clock_ms;
        self.clock_ms = gesture::end_ms(batches, self.clock_ms).max(self.clock_ms) + FRAME_MS;
        Ok(())
    }

    /// Frames one 60 Hz frame apart until nothing moves on screen, at most `SETTLE_FRAMES`.
    fn settle(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        for _ in 0..SETTLE_FRAMES {
            if !runtime.moving() {
                return Ok(());
            }
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
        Err(ScenarioError::Missing("motion did not settle"))
    }

    /// Frames one 60 Hz frame apart up to `target`, then one at `target`.
    fn advance_to(&mut self, runtime: &mut Runtime, target: i64) -> Result<(), ScenarioError> {
        while self.clock_ms < target {
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
        let next = self.clock_ms;
        self.clock_ms = target;
        self.frame(runtime)?;
        self.clock_ms = next;
        Ok(())
    }

    /// One frame and the frames the runtime then asks for at once (a button that changed what
    /// the UI offers, egui's own layout), as the host's frame pacing gives them, up to
    /// `FOLLOW_UPS` more, all at the runner's clock now; again while a worker (storage,
    /// transfer) is busy, until it is done. Then the host takes what the app asks of the system.
    fn frame(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        let screen = self.scenario.screen;
        let start = Instant::now();
        runtime.fix_clock(self.clock_ms);
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
