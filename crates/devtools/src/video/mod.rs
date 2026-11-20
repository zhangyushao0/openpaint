//! The replay's videos in a scenario (S163): the encoder the runner lends the app for every
//! `HostRequest::Video`, answered at once as Android makes one, keeping what a scenario checks
//! of each video (its order, every call with each frame's time, frames evenly spaced over it and
//! its last, never all of them), and the `video` step's check of the last video.
//!
//! The encoder takes frames as the runner lets it: each frame of the runner's lets it take
//! `VIDEO_FRAMES` more (`VideoSink::release`), the video worker waiting in `frame` until then,
//! and the runner goes on only once the worker waits again or the video ended. So how far a
//! video got when a step looks (the notice's percent, the moment of a Cancel) is the same every
//! run, on any machine.
//!
//! ```json
//! { "video": { "name": "full", "frames": 213, "size": [1280, 800], "ended": "finished" } }
//! ```
//!
//! The step runs frames until the video ended and the app took its end, then checks: begun with
//! its order, frame `k` at `k × 1_000_000 / fps` µs in order, each of the order's size, then
//! finished (or, with `"ended": "cancelled"`, cancelled); `frames` and `size` when given. The
//! frames kept are screenshots `<name>-<frame>` and `<name>-last`; with `last`, the last frame
//! is a checkpoint `<name>` that must hash to it (a drawing frame is the canvas's pixels scaled
//! in integers, alike on every GPU; the end card's line is text, which is not).

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use openpaint_app::{VideoEncoder, VideoError, VideoOrder};
use openpaint_render::RgbaImage;
use serde::Deserialize;

use crate::error::ScenarioError;
use crate::report::{Checkpoint, Record, Screenshot};
use crate::scenario::optional_hash;

/// Video frames the encoder may take for each frame of the runner's.
const VIDEO_FRAMES: u64 = 30;
/// Frames kept of a video beside its last: evenly spaced, their spacing doubled as it grows.
const KEPT: usize = 8;
/// The longest the runner waits for the video worker, and the worker for the runner: wall time,
/// a guard against a hang, never a measure.
const HANG: Duration = Duration::from_secs(120);

/// A `video` step: what the last video must be (see the module).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoCheck {
    pub name: String,
    #[serde(default)]
    pub frames: Option<u32>,
    #[serde(default)]
    pub size: Option<[u32; 2]>,
    #[serde(default)]
    pub ended: Ended,
    #[serde(default, deserialize_with = "optional_hash")]
    pub last: Option<u64>,
}

/// How a video ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ended {
    #[default]
    Finished,
    Cancelled,
}

/// What became of a lent encoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Lent,
    Begun,
    Finished,
    Cancelled,
    /// Dropped never begun (no job was made of it).
    Dropped,
}

/// A call of the encoder's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    Begin,
    Frame(u64),
    Finish,
    Cancel,
}

/// One video as its encoder kept it.
#[derive(Debug, Clone)]
pub(crate) struct Video {
    order: Option<VideoOrder>,
    calls: Vec<Call>,
    state: State,
    /// Frames taken, and those kept: every `stride`-th, then the newest.
    frames: u32,
    stride: u32,
    kept: Vec<(u32, RgbaImage)>,
    last: Option<(u32, RgbaImage)>,
    /// What it was handed that no encoder takes (a frame of another size, a call out of order).
    flaws: Vec<String>,
}

/// The encoders' frames and how many they may take.
#[derive(Debug, Default)]
struct Gate {
    allowed: u64,
    taken: u64,
    /// No limit: the run ends or the app restarts.
    open: bool,
    /// An encoder waits in `frame` for more.
    waiting: bool,
    videos: Vec<Video>,
}

type Shared = Arc<(Mutex<Gate>, Condvar)>;

fn locked(shared: &Shared) -> MutexGuard<'_, Gate> {
    shared.0.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The runner's side: it lends encoders, lets frames through and reads the videos.
#[derive(Debug, Clone, Default)]
pub(crate) struct VideoSink {
    shared: Shared,
}

/// Opens the sink's gate when dropped: a video worker never waits for a runner that is gone.
pub(crate) struct Opener(Shared);

impl Drop for Opener {
    fn drop(&mut self) {
        locked(&self.0).open = true;
        self.0.1.notify_all();
    }
}

impl VideoSink {
    /// A new encoder for the app, its video kept from now on.
    pub(crate) fn lend(&self) -> Box<dyn VideoEncoder> {
        let mut gate = locked(&self.shared);
        gate.videos.push(Video::lent());
        let index = gate.videos.len() - 1;
        Box::new(Encoder {
            shared: Arc::clone(&self.shared),
            index,
        })
    }

    /// The runner's frame: `VIDEO_FRAMES` more frames let through, then a wait until the video
    /// worker waits for more or no video is being made.
    pub(crate) fn release(&self) -> Result<(), ScenarioError> {
        let mut gate = locked(&self.shared);
        if !gate.making() {
            return Ok(());
        }
        gate.allowed = gate.taken + VIDEO_FRAMES;
        gate.waiting = false;
        self.shared.1.notify_all();
        let started = Instant::now();
        while gate.making() && !gate.waiting && !gate.open {
            let left = HANG.saturating_sub(started.elapsed());
            if left.is_zero() {
                return Err(ScenarioError::Missing(
                    "the video worker did not take its frames",
                ));
            }
            gate = (self.shared.1.wait_timeout(gate, left))
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        Ok(())
    }

    /// The gate opened from now on (an app about to be ended), or held again (`open` false).
    pub(crate) fn set_open(&self, open: bool) {
        locked(&self.shared).open = open;
        self.shared.1.notify_all();
    }

    /// What opens the gate when the run ends, however it ends.
    pub(crate) fn opener(&self) -> Opener {
        Opener(Arc::clone(&self.shared))
    }

    /// The last video lent, as kept so far.
    pub(crate) fn last(&self) -> Option<Video> {
        locked(&self.shared).videos.last().cloned()
    }
}

impl Gate {
    /// A video is lent or begun and not ended.
    fn making(&self) -> bool {
        (self.videos.iter()).any(|video| matches!(video.state, State::Lent | State::Begun))
    }
}

/// The encoder lent for one video.
struct Encoder {
    shared: Shared,
    index: usize,
}

impl Encoder {
    /// A call on this encoder's video, the runner told.
    fn with<T>(&self, call: impl FnOnce(&mut Video) -> T) -> T {
        let mut gate = locked(&self.shared);
        let kept = call(&mut gate.videos[self.index]);
        self.shared.1.notify_all();
        kept
    }
}

impl VideoEncoder for Encoder {
    fn begin(&mut self, order: &VideoOrder) -> Result<(), VideoError> {
        self.with(|video| video.begin(order));
        Ok(())
    }

    fn frame(&mut self, rgba: &[u8], at_us: u64) -> Result<(), VideoError> {
        let started = Instant::now();
        let mut gate = locked(&self.shared);
        while !gate.open && gate.taken >= gate.allowed && started.elapsed() < HANG {
            gate.waiting = true;
            self.shared.1.notify_all();
            let left = HANG.saturating_sub(started.elapsed());
            gate = (self.shared.1.wait_timeout(gate, left))
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        gate.waiting = false;
        gate.taken += 1;
        gate.videos[self.index].frame(rgba, at_us);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), VideoError> {
        self.with(|video| video.end(Call::Finish, State::Finished));
        Ok(())
    }

    fn cancel(&mut self) {
        self.with(|video| video.end(Call::Cancel, State::Cancelled));
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        self.with(|video| {
            if video.state == State::Lent {
                video.state = State::Dropped;
            }
        });
    }
}

impl Video {
    fn lent() -> Self {
        Self {
            order: None,
            calls: Vec::new(),
            state: State::Lent,
            frames: 0,
            stride: 1,
            kept: Vec::new(),
            last: None,
            flaws: Vec::new(),
        }
    }

    fn begin(&mut self, order: &VideoOrder) {
        if self.state != State::Lent {
            self.flaws.push(format!("begun again as {:?}", self.state));
        }
        self.order = Some(order.clone());
        self.calls.push(Call::Begin);
        self.state = State::Begun;
    }

    fn frame(&mut self, rgba: &[u8], at_us: u64) {
        self.calls.push(Call::Frame(at_us));
        let index = self.frames;
        self.frames += 1;
        let size = self.order.as_ref().map_or([0, 0], |order| order.size);
        let Some(image) = RgbaImage::from_rgba(size, rgba.to_vec()) else {
            let flaw = format!("frame {index}: {} bytes for {size:?}", rgba.len());
            self.flaws.push(flaw);
            return;
        };
        if index.is_multiple_of(self.stride) {
            self.kept.push((index, image.clone()));
            if self.kept.len() > KEPT {
                let wider = self.stride * 2;
                self.kept.retain(|(at, _)| at.is_multiple_of(wider));
                self.stride = wider;
            }
        }
        self.last = Some((index, image));
    }

    fn end(&mut self, call: Call, state: State) {
        if self.state != State::Begun {
            self.flaws.push(format!("{call:?} while {:?}", self.state));
        }
        self.calls.push(call);
        self.state = state;
    }

    /// It ended: finished, cancelled or dropped unbegun.
    pub(crate) fn ended(&self) -> bool {
        !matches!(self.state, State::Lent | State::Begun)
    }

    /// `check` against what was kept: its records, or what differs.
    pub(crate) fn check(&self, check: &VideoCheck) -> Result<Vec<Record>, ScenarioError> {
        let differs = |what: String| ScenarioError::Unexpected {
            what: format!("video {}: {what}", check.name),
            available: Vec::new(),
        };
        if let Some(flaw) = self.flaws.first() {
            return Err(differs(flaw.clone()));
        }
        let order = self
            .order
            .as_ref()
            .ok_or_else(|| differs("never begun".into()))?;
        let state = match check.ended {
            Ended::Finished => State::Finished,
            Ended::Cancelled => State::Cancelled,
        };
        if self.state != state {
            return Err(differs(format!("{:?}, not {state:?}", self.state)));
        }
        let fps = u64::from(order.fps.max(1));
        let frames = (0..u64::from(self.frames)).map(|k| Call::Frame(k * 1_000_000 / fps));
        let end = [if state == State::Finished {
            Call::Finish
        } else {
            Call::Cancel
        }];
        let calls: Vec<Call> = [Call::Begin].into_iter().chain(frames).chain(end).collect();
        if self.calls != calls {
            return Err(differs(format!("calls out of order: {:?}", self.calls)));
        }
        if check.frames.is_some_and(|frames| frames != self.frames) {
            return Err(differs(format!("{} frames", self.frames)));
        }
        if check.size.is_some_and(|size| size != order.size) {
            return Err(differs(format!("{:?} pixels", order.size)));
        }
        Ok(self.records(check))
    }

    /// The kept frames as screenshots, the last as one, and as a checkpoint with `last`.
    fn records(&self, check: &VideoCheck) -> Vec<Record> {
        let shot = |name: String, screen: &RgbaImage| {
            Record::Screenshot(Screenshot {
                name,
                screen: screen.clone(),
            })
        };
        let mut records: Vec<Record> = (self.kept.iter())
            .map(|(index, image)| shot(format!("{}-{index:04}", check.name), image))
            .collect();
        if let Some((_, image)) = &self.last {
            records.push(shot(format!("{}-last", check.name), image));
            if let Some(expected) = check.last {
                records.push(Record::Checkpoint(Checkpoint {
                    name: check.name.clone(),
                    expected,
                    canvas: image.clone(),
                }));
            }
        }
        records
    }
}

#[cfg(test)]
mod tests;
