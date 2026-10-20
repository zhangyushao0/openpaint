//! Performance hints (S67, DECISIONS D94): while a hand is on or over the glass, and for
//! `IDLE_AFTER` after it left (D87's rule, `touches.rs`), every frame drawn reports its work to the
//! host's `WorkHints`, so the system keeps the clocks where the frames land on their vsync (on
//! Android a performance hint session, `apps/android/src/hint.rs`). The first frame call that
//! finds the screen idle pauses them; leaving the app closes them; a host without them changes
//! nothing. Frames are timed on the GPU while reports go (`Core::time_frames`): a report carries
//! the latest frame the GPU finished, a frame or two before the one reported.

use std::sync::Arc;
use std::time::{Duration, Instant};

use openpaint_render::FrameTime;

use crate::frame::Runtime;

/// What the host does with the frames' work. The runtime calls `joined` once on each thread doing
/// the frames' work (the frame loop's and the raster worker's), then `report` for each frame drawn
/// while a hand is there or was a moment ago, `pause` once the screen rests and `close` when the
/// app leaves the foreground; a report after `close` starts again.
pub trait WorkHints: Send + Sync {
    /// The calling thread does the frames' work.
    fn joined(&self);
    /// A frame drawn: its work.
    fn report(&self, work: FrameWork);
    /// The screen rests: no reports until a hand comes back.
    fn pause(&self);
    /// The app left the foreground.
    fn close(&self);
}

/// One frame's work as reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameWork {
    /// When the frame call began.
    pub began: Instant,
    /// The frame call's time on the frame loop's thread.
    pub cpu: Duration,
    /// The latest frame the GPU finished: its GPU time, `None` without timestamps.
    pub gpu: Option<Duration>,
    /// The frame's time until its result was ready: the frame call's, or the latest frame's from
    /// its beginning until the GPU finished it, whichever is longer.
    pub total: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Closed,
    Reporting,
    Paused,
}

/// The runtime's side: when to report, pause and close.
pub(crate) struct Hints {
    host: Option<Arc<dyn WorkHints>>,
    state: State,
    /// The latest frame the GPU finished, while frames are timed.
    latest: Option<FrameTime>,
}

impl Default for Hints {
    fn default() -> Self {
        Self {
            host: None,
            state: State::Closed,
            latest: None,
        }
    }
}

impl Hints {
    pub(crate) fn set(&mut self, host: Arc<dyn WorkHints>) {
        self.host = Some(host);
        self.state = State::Closed;
    }

    /// Whether frames are timed on the GPU: while a hand is there (not `idle`) and the host has
    /// hints.
    pub(crate) fn timing(&self, idle: bool) -> bool {
        self.host.is_some() && !idle
    }

    /// Timed frames read back.
    pub(crate) fn read(&mut self, times: &[FrameTime]) {
        if let Some(last) = times.last() {
            self.latest = Some(*last);
        }
    }

    /// A frame call ended, `idle` or not: a frame `drawn` (it began then and took `cpu`) is
    /// reported while a hand is there; the first call finding the screen idle pauses.
    pub(crate) fn called(&mut self, idle: bool, drawn: Option<(Instant, Duration)>) {
        let Some(host) = &self.host else {
            return;
        };
        if idle {
            if self.state == State::Reporting {
                host.pause();
                self.state = State::Paused;
            }
            self.latest = None;
            return;
        }
        if let Some((began, cpu)) = drawn {
            let done = (self.latest).map(|time| Duration::from_nanos(time.done_ns as u64));
            host.report(FrameWork {
                began,
                cpu,
                gpu: (self.latest).map(|time| Duration::from_nanos(time.gpu_ns as u64)),
                total: done.map_or(cpu, |done| done.max(cpu)),
            });
            self.state = State::Reporting;
        }
    }

    /// The app leaves the foreground.
    pub(crate) fn leave(&mut self) {
        if let Some(host) = &self.host
            && self.state != State::Closed
        {
            host.close();
            self.state = State::Closed;
        }
        self.latest = None;
    }
}

impl Runtime {
    /// The host's performance hints (S67); call on the frame loop's thread, which joins them, as
    /// the raster worker's does.
    pub fn set_work_hints(&mut self, hints: Arc<dyn WorkHints>) {
        hints.joined();
        self.raster.join(Arc::clone(&hints));
        self.hints.set(hints);
    }

    /// Frames are timed on the GPU from this frame call on while a benchmark or the hints want it.
    pub(crate) fn time_frames(&mut self, idle: bool) {
        let on = self.bench_timing || self.hints.timing(idle);
        self.core.time_frames(on);
    }

    /// A frame call that began at `began` ended (`drawn` or skipped): the timed frames read back
    /// go to the hints and, while a benchmark times them, to it; then what the hints are told.
    pub(crate) fn hint(&mut self, began: Instant, drawn: bool) {
        let times = self.core.frame_times();
        self.hints.read(&times);
        if self.bench_timing {
            self.timed.extend(times);
        }
        let idle = self.idle();
        self.hints
            .called(idle, drawn.then(|| (began, began.elapsed())));
    }

    /// A benchmark times the frames on the GPU from now on (`on`), or no longer.
    pub(crate) fn bench_time_frames(&mut self, on: bool) {
        self.bench_timing = on;
        self.core.time_frames(on);
    }

    /// The GPU time of each frame timed for a benchmark since the last call, ms.
    pub(crate) fn bench_frame_times(&mut self) -> Vec<f64> {
        let times = self.core.frame_times();
        self.hints.read(&times);
        self.timed.extend(times);
        (self.timed.drain(..))
            .map(|time| time.gpu_ns / 1e6)
            .collect()
    }
}

#[cfg(test)]
#[path = "hints_tests.rs"]
mod tests;
