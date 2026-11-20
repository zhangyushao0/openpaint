//! Runs a scenario through the product frame loop, offscreen: each gesture batch is handed to
//! the runtime and followed by one frame, plus the frames the runtime asks for at once, as on
//! the tablet, and then frames until the storage worker is done, so every step sees the store's
//! answer. The app's library lives in a directory the caller owns; `restart` reopens it. What
//! the app asks of the system (pickers, the share sheet) waits for a `system` step to answer it
//! as the user would (`system.rs`); an encoder for the replay's video is lent at once
//! (`video.rs`), its frames let through a runner's frame at a time. Checkpoints, screenshots,
//! exported files and the videos' frames are collected into a report; a failed checkpoint does
//! not stop the run. The steps that touch and look: `contact.rs`; the clock: `clock.rs`.
//!
//! Motion (D44) and every gesture that waits (a hold, a drop, QuickShape) run on a clock the
//! runner holds: the time of the input being handed over, then one 60 Hz frame further per frame
//! without input; waiting for a worker never moves it. Input steps never wait for motion (the hand
//! acts on the final state at once); a step that looks (a checkpoint, a screenshot, a view, an
//! export, an expect) first lets every motion settle, unless a screenshot asks for a moment
//! `after_ms` the last input, where the canvas's background work is done with the clock held
//! first (S129). A last newline of a `type` step is Enter, the keyboard's Done.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use openpaint_practice_store::Packs;
use openpaint_render::Core;
use openpaint_runtime::{
    EditKey, HostAnswer, HostRequest, Language, Runtime, RuntimeError, StartOptions, TextFonts,
    TextInput,
};

use crate::error::ScenarioError;
use crate::host::Hosted;
use crate::report::{Record, Report};
use crate::scenario::{Lesson, Scenario, StartOn, Step, SystemLanguage};
use crate::system::{self, Written};
use crate::target;
use crate::video::{VideoCheck, VideoSink};
use crate::watched::Watched;

/// Milliseconds of one 60 Hz frame, for the input clock.
const FRAME_MS: i64 = 16;
/// Most frames run after one because the runtime asked for them.
const FOLLOW_UPS: usize = 4;
/// Longest wait for a worker after a step, or for motion to settle before a step looks (a How
/// card's demonstration plays for up to about 10 s of the held clock, S92): wall time, a guard
/// against a hang, never a count of frames (the scenarios run a dozen at once beside the gate).
const HANG: Duration = Duration::from_secs(120);
/// The seed every stroke's stamps take: fixed, so hashes hold.
const SEED: u32 = 0x5EED_0045;

/// Starts the app on the library in `library` (created when missing) with its practice packs
/// kept in `packs` (a directory the caller makes fresh), with the text faces a host would hand
/// over, opens it and runs every step.
pub fn run(
    scenario: &Scenario,
    (library, packs): (&Path, &Path),
    fonts: &TextFonts,
) -> Result<Report, ScenarioError> {
    let started = Instant::now();
    // One device for the run, restarts included; its screen in 8 bits: every check reads 8-bit
    // values, which a ten-bit screen gives only after decoding each texel on the CPU.
    let core = Core::headless_eight_bits().map_err(RuntimeError::Render)?;
    clock::hold_gpu(&core)?;
    let mut runner = Runner {
        core: Arc::new(core),
        scenario,
        library,
        packs,
        fonts,
        storage_off: Arc::new(AtomicBool::new(false)),
        clock_ms: 0,
        input_ms: 0,
        frames: 0,
        requests: Vec::new(),
        hosted: Hosted::default(),
        written: Written::default(),
        videos: VideoSink::default(),
        index: 0,
        unnamed: Vec::new(),
    };
    let mut runtime = runner.start()?;
    // Dropped before the runtime, however the run ends: its video worker never waits for us.
    let _videos = runner.videos.opener();
    let mut records = Vec::new();
    for (index, step) in scenario.steps.iter().enumerate() {
        runner.index = index;
        if *step == Step::Restart {
            // The user leaves (the host's `onPause`), then the system ends the process: what
            // the leave handed over is written and the library closes before the new start.
            runtime.leave();
            runner.videos.set_open(true);
            drop(runtime);
            runner.videos.set_open(false);
            runner.requests.clear();
            runtime = runner.start()?;
            continue;
        }
        records.extend(runner.step(&mut runtime, step)?);
        // A `lose_device` step hands the runner the device made again; any other new device is
        // the driver's reset (under a suite's load), whose notice the screens would then show.
        if !Arc::ptr_eq(&runtime.core(), &runner.core) {
            return Err(ScenarioError::Missing(
                "device lost: the driver reset the GPU, not a step; the display was made again",
            ));
        }
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
    packs: &'a Path,
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
    /// The chimes and the screen kept on the app asked of its host (`host.rs`).
    hosted: Hosted,
    /// The file the last export wrote.
    written: Written,
    /// The encoders lent for the replay's videos, and what they kept.
    videos: VideoSink,
    /// The step running, from 0.
    index: usize,
    /// Contacts given as window pixels that land on a control, which the step should name.
    unnamed: Vec<String>,
}

impl Runner<'_> {
    /// A new app process on the library: everything a runtime held before is gone, history
    /// included; what survives is what the library stores. Runs frames until it has opened.
    fn start(&mut self) -> Result<Runtime, ScenarioError> {
        let store = Watched::new(self.library, Arc::clone(&self.storage_off));
        let language = match self.scenario.system_language {
            SystemLanguage::English => Language::English,
            SystemLanguage::Chinese => Language::Chinese,
        };
        let options = StartOptions {
            core: Some(Arc::clone(&self.core)),
            language,
            packs: Some(Packs::at(self.packs)),
            blank: self.scenario.start == StartOn::Canvas,
            ..StartOptions::default()
        };
        let mut runtime = Runtime::start(self.scenario.density, Box::new(store), options)?;
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
            | Step::Preview { .. }
            | Step::Screenshot { .. }
            | Step::Exported { .. } => {
                return self
                    .look(runtime, step)
                    .map(|seen| seen.into_iter().collect());
            }
            Step::Video(check) => return self.video(runtime, check),
            Step::Type(text) => {
                let typed = text.strip_suffix('\n');
                runtime.text(TextInput::Commit(typed.unwrap_or(text).to_owned()));
                self.frame(runtime)?;
                if typed.is_some() {
                    runtime.text(TextInput::Key(EditKey::Enter));
                    self.frame(runtime)?;
                }
            }
            Step::Expect { shown, absent } => {
                self.settle(runtime)?;
                target::expect(runtime.controls(), (shown, absent))?;
            }
            Step::Host(expected) => self.hosted.expect(*expected)?,
            Step::Back => {
                if !runtime.back() {
                    return Err(ScenarioError::Missing("back was not used by the app"));
                }
                self.frame(runtime)?;
                self.input_ms = self.clock_ms;
            }
            Step::Lesson(lesson) => self.lesson(runtime, lesson)?,
            Step::Wait { frames } => self.wait(runtime, *frames)?,
            Step::Later { minutes, hidden } => self.later(runtime, (*minutes, *hidden))?,
            Step::Restart => return Err(ScenarioError::Missing("restart is run by `run`")),
            Step::StorageFails => self.storage_off.store(true, Ordering::Relaxed),
            Step::StorageWorks => self.storage_off.store(false, Ordering::Relaxed),
            Step::LoseDevice => {
                runtime.lose_device();
                self.frame(runtime)?;
                // A restart opens the app on the device made again.
                self.core = runtime.core();
                clock::hold_gpu(&self.core)?;
            }
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

    /// A `lesson` step: the lesson started from the home, then frames until the store wrote its
    /// sheet and it opened.
    fn lesson(&mut self, runtime: &mut Runtime, lesson: &Lesson) -> Result<(), ScenarioError> {
        runtime.start_lesson(lesson.start()?)?;
        self.frame(runtime)?;
        self.input_ms = self.clock_ms;
        Ok(())
    }

    /// One frame and the frames the runtime then asks for at once (a button that changed what
    /// the UI offers, egui's own layout), as the host's frame pacing gives them, up to
    /// `FOLLOW_UPS` more, all at the runner's clock now; again while a worker (storage,
    /// transfer) is busy, until it is done. Then the host takes what the app asks of the system:
    /// an encoder for a video at once. A video being made takes its next frames first.
    fn frame(&mut self, runtime: &mut Runtime) -> Result<(), ScenarioError> {
        self.videos.release()?;
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
                if start.elapsed() > HANG {
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
        // The host takes what the app asks of the system after each frame; an encoder is made
        // at once, as Android makes it.
        let asked: Vec<HostRequest> = std::iter::from_fn(|| runtime.host_request()).collect();
        for request in asked {
            match request {
                HostRequest::Video { .. } => runtime.answer(HostAnswer::Video(self.videos.lend())),
                request => self.requests.push(request),
            }
        }
        self.hosted.frame(runtime);
        Ok(())
    }

    /// A `video` step: frames until the last video ended and the app took its end, then its
    /// check (`video.rs`).
    fn video(
        &mut self,
        runtime: &mut Runtime,
        check: &VideoCheck,
    ) -> Result<Vec<Record>, ScenarioError> {
        let start = Instant::now();
        loop {
            let video = self.videos.last();
            let video = video.ok_or(ScenarioError::Missing("the app asked for no video"))?;
            if video.ended() && !runtime.making_video() {
                return video.check(check);
            }
            if start.elapsed() > HANG {
                return Err(ScenarioError::Missing("the video did not end"));
            }
            self.frame(runtime)?;
            self.clock_ms += FRAME_MS;
        }
    }
}

mod clock;
mod contact;
mod press;
