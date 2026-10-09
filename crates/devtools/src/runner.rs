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

use openpaint_render::Core;
use openpaint_runtime::{
    EditKey, HostRequest, MotionBatch, Runtime, RuntimeError, TextFonts, TextInput,
};

use crate::error::ScenarioError;
use crate::report::{Record, Report, record, view_check};
use crate::scenario::{Scenario, Step};
use crate::switch::Switched;
use crate::system::{self, Written};
use crate::target::{self, Key, Spot};
use crate::{gesture, pen};

/// Milliseconds of one 60 Hz frame, for the input clock.
const FRAME_MS: i64 = 16;
/// Most frames run after one because the runtime asked for them.
const FOLLOW_UPS: usize = 4;
/// Longest wait for the storage worker after a step.
const STORAGE_LIMIT: Duration = Duration::from_secs(10);
/// Most frames motion may take to settle before a step looks.
const SETTLE_FRAMES: usize = 240;
/// The seed every stroke's stamps take (S45): fixed, so hashes hold.
const SEED: u32 = 0x5EED_0045;

/// Starts the app on the library in `library` (created when missing), with the text faces a
/// host would hand over, opens it and runs every step.
pub fn run(
    scenario: &Scenario,
    library: &Path,
    fonts: &TextFonts,
) -> Result<Report, ScenarioError> {
    let started = Instant::now();
    // One device for the run, restarts included; its screen in 8 bits: every check reads 8-bit
    // values, which a ten-bit screen gives only after decoding each texel on the CPU.
    let core = Core::headless_eight_bits().map_err(RuntimeError::Render)?;
    let mut runner = Runner {
        core: Arc::new(core),
        scenario,
        library,
        fonts,
        storage_off: Arc::new(AtomicBool::new(false)),
        clock_ms: 0,
        input_ms: 0,
        frames: 0,
        requests: Vec::new(),
        written: Written::default(),
        index: 0,
        unnamed: Vec::new(),
    };
    let mut runtime = runner.start()?;
    let mut records = Vec::new();
    for (index, step) in scenario.steps.iter().enumerate() {
        runner.index = index;
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
        unnamed: runner.unnamed,
        elapsed: started.elapsed(),
        frames: runner.frames,
        adapter: runtime.adapter(),
    })
}

struct Runner<'a> {
    core: Arc<Core>,
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
    /// The step running, from 0.
    index: usize,
    /// Contacts given as window pixels that land on a control, which the step should name.
    unnamed: Vec<String>,
}

impl Runner<'_> {
    /// A new app process on the library: everything a runtime held before is gone, history
    /// included; what survives is what the library stores. Runs frames until it has opened.
    fn start(&mut self) -> Result<Runtime, ScenarioError> {
        let store = Switched::new(self.library, Arc::clone(&self.storage_off));
        let core = Arc::clone(&self.core);
        let mut runtime = Runtime::start_on(core, self.scenario.density, Box::new(store))?;
        runtime.set_text_fonts(self.fonts.clone());
        self.frame(&mut runtime)?;
        Ok(runtime)
    }

    fn step(&mut self, runtime: &mut Runtime, step: &Step) -> Result<Vec<Record>, ScenarioError> {
        match step {
            Step::Tap { .. }
            | Step::Hover { .. }
            | Step::Drag { .. }
            | Step::Press { .. }
            | Step::Stroke { .. }
            | Step::Touch { .. } => return self.contact(runtime, step),
            Step::View { .. }
            | Step::Checkpoint { .. }
            | Step::Screenshot { .. }
            | Step::Exported { .. } => {
                return self
                    .look(runtime, step)
                    .map(|seen| seen.into_iter().collect());
            }
            Step::Type(text) => {
                let (text, enter) = match text.strip_suffix('\n') {
                    Some(text) => (text, true),
                    None => (text.as_str(), false),
                };
                runtime.text(TextInput::Commit(text.to_owned()));
                self.frame(runtime)?;
                if enter {
                    runtime.text(TextInput::Key(EditKey::Enter));
                    self.frame(runtime)?;
                }
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
        Ok(Vec::new())
    }

    /// A step that touches the screen, its spots resolved on the screen the last frame laid
    /// out.
    fn contact(
        &mut self,
        runtime: &mut Runtime,
        step: &Step,
    ) -> Result<Vec<Record>, ScenarioError> {
        let now = self.clock_ms;
        let batches = match step {
            Step::Tap { at, tool } => gesture::tap(*tool, self.spot(runtime, at)?, now),
            Step::Hover { at, pen } => {
                pen::held(gesture::hover(self.spot(runtime, at)?, now), *pen)
            }
            Step::Drag { from, to, tool } => {
                let (from, to) = (self.spot(runtime, from)?, self.spot(runtime, to)?);
                let points = [[from[0], from[1], 1.0], [to[0], to[1], 1.0]];
                gesture::path(*tool, &points, now)
            }
            Step::Press {
                path,
                tool,
                pressure,
                before_lift,
                canvas_before_lift,
            } => {
                let keys = self.keys(runtime, path)?;
                let batches = gesture::press(*tool, &keys, *pressure, now);
                let before = before_lift.as_ref().map(|name| (name, *canvas_before_lift));
                return self.press(runtime, &batches, before);
            }
            Step::Stroke {
                points,
                lift,
                palm,
                sparse,
                hold_ms,
                pen,
                tilt,
                azimuth,
            } => {
                let path = (points.as_slice(), *sparse, *hold_ms);
                let lean = [*tilt, *azimuth];
                pen::held(gesture::stroke(path, *lift, *palm, lean, now), *pen)
            }
            Step::Touch { from, to, ms } => {
                let from = self.spots(runtime, from)?;
                let to = match to {
                    Some(to) => self.spots(runtime, to)?,
                    None => from.clone(),
                };
                gesture::fingers(&from, &to, *ms, now)
            }
            _ => return Err(ScenarioError::Missing("not a step that touches")),
        };
        self.gesture(runtime, &batches)?;
        Ok(Vec::new())
    }

    /// `spot` in window pixels on the screen the last frame laid out. Window pixels on a
    /// control are noted: the step should name it.
    fn spot(&mut self, runtime: &Runtime, spot: &Spot) -> Result<[f32; 2], ScenarioError> {
        let map = runtime.controls();
        let at = target::resolve(map, spot)?;
        if matches!(spot, Spot::Point(_))
            && let Some(instead) = target::named_instead(map, at)
        {
            self.unnamed.push(format!("step {}: {instead}", self.index));
        }
        Ok(at)
    }

    fn spots(&mut self, runtime: &Runtime, spots: &[Spot]) -> Result<Vec<[f32; 2]>, ScenarioError> {
        spots.iter().map(|spot| self.spot(runtime, spot)).collect()
    }

    /// A press's keys as `[x, y, ms]`.
    fn keys(&mut self, runtime: &Runtime, keys: &[Key]) -> Result<Vec<[f32; 3]>, ScenarioError> {
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            let (spot, ms) = match key {
                Key::Point([x, y, ms]) => (Spot::Point([*x, *y]), *ms),
                Key::At { at, ms } => (at.clone(), *ms),
            };
            let [x, y] = self.spot(runtime, &spot)?;
            out.push([x, y, ms]);
        }
        Ok(out)
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

    /// A press's batches; with `before_lift`, a screenshot of that name before the lift and,
    /// given its hash, a checkpoint of the canvas there too (S44).
    fn press(
        &mut self,
        runtime: &mut Runtime,
        batches: &[MotionBatch],
        before_lift: Option<(&String, Option<u64>)>,
    ) -> Result<Vec<Record>, ScenarioError> {
        let Some((name, canvas)) = before_lift else {
            self.gesture(runtime, batches)?;
            return Ok(Vec::new());
        };
        let (lift, held) = batches
            .split_last()
            .ok_or(ScenarioError::Missing("a press has no samples"))?;
        self.gesture(runtime, held)?;
        let shot = Step::Screenshot {
            name: name.clone(),
            after_ms: None,
        };
        let mut records = vec![record(runtime, &shot, &self.written)?];
        if let Some(canvas) = canvas {
            let name = name.clone();
            let check = Step::Checkpoint { name, canvas };
            records.push(record(runtime, &check, &self.written)?);
        }
        self.gesture(runtime, std::slice::from_ref(lift))?;
        Ok(records)
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
        runtime.fix_seed(SEED);
        // The follow-ups count once no worker is busy: the frame that takes a worker's result
        // may ask for more (a canvas just opened lays itself out), however late it comes.
        let mut follow_ups = 0;
        loop {
            let again = runtime.frame_offscreen(screen)?;
            self.frames += 1;
            if runtime.storing() {
                if start.elapsed() > STORAGE_LIMIT {
                    return Err(ScenarioError::Missing("the storage worker did not finish"));
                }
                if !again {
                    std::thread::sleep(Duration::from_millis(1));
                }
                continue;
            }
            follow_ups += 1;
            if !again || follow_ups > FOLLOW_UPS {
                break;
            }
        }
        // The host takes what the app asks of the system after each frame.
        self.requests
            .extend(std::iter::from_fn(|| runtime.host_request()));
        Ok(())
    }
}
