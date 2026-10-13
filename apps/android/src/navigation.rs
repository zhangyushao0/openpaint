//! Frames while fingers navigate or work waits (S53), logged once per episode as the pen's
//! latency is per stroke (`latency.rs`). An episode runs from the first frame with a finger on
//! the glass or work waiting (the canvas or a view tier filling in, something moving) to the
//! first frame with neither; its line gives the frames, how many had a finger down, each frame's
//! work (frame start to present return, as the pen's line) as p50 / p95 / max, the background
//! work's GPU time a frame by the frame budget's corrected estimate and in all as GPU timestamps
//! measured it, and what render counted over it: dispatches and their op entries, plans binned
//! and the entries they listed, smudge residuals derived, view tier tiles filled. An episode
//! longer than `LONGEST` is logged and starts over, so work that never ends still shows.

use std::fmt;

use openpaint_runtime::{Background, Counts, MotionBatch, MotionPhase, ToolKind};

use crate::latency::Spread;

/// The longest episode logged as one, in frame start nanoseconds.
const LONGEST: i64 = 10_000_000_000;

#[derive(Debug, Default)]
pub struct NavigationLog {
    /// Fingers on the glass after the last batch.
    fingers: usize,
    episode: Option<Episode>,
    /// Render's counters after the last frame: what the next episode counts from.
    last: Counts,
}

#[derive(Debug)]
struct Episode {
    started_ns: i64,
    work: Vec<i64>,
    /// Per frame, the background work's corrected estimate in ns.
    background: Vec<i64>,
    /// GPU time measured for background work, and its estimate, ns.
    measured: f64,
    touched: usize,
    first: Counts,
}

/// One frame as the log takes it: when it started and its work (ns), whether work still waits
/// after it, render's counters after it, and the background work of the frame before it.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub start_ns: i64,
    pub work_ns: i64,
    pub waiting: bool,
    pub counts: Counts,
    pub background: Background,
}

impl NavigationLog {
    /// A platform event: counts the fingers on the glass.
    pub fn input(&mut self, batch: &MotionBatch) {
        let fingers = batch
            .pointers
            .iter()
            .filter(|pointer| pointer.tool == ToolKind::Finger)
            .count();
        self.fingers = match batch.phase {
            MotionPhase::Down | MotionPhase::Move | MotionPhase::PointerDown { .. } => fingers,
            MotionPhase::PointerUp { index } | MotionPhase::PointerCancel { index } => {
                let lifted = batch
                    .pointers
                    .get(index)
                    .is_some_and(|pointer| pointer.tool == ToolKind::Finger);
                fingers.saturating_sub(usize::from(lifted))
            }
            MotionPhase::Up | MotionPhase::Cancel => 0,
            _ => self.fingers,
        };
    }

    /// A frame ran; returns the episode's summary when it ended.
    pub fn frame(&mut self, frame: Frame) -> Option<Summary> {
        let busy = self.fingers > 0 || frame.waiting;
        let before = std::mem::replace(&mut self.last, frame.counts);
        let ended = match &mut self.episode {
            Some(episode) if busy && frame.start_ns - episode.started_ns < LONGEST => {
                episode.add(&frame, self.fingers > 0);
                return None;
            }
            Some(_) => (self.episode.take()).map(|episode| episode.summary(frame.start_ns, before)),
            None => None,
        };
        if busy {
            let mut episode = Episode {
                started_ns: frame.start_ns,
                work: Vec::new(),
                background: Vec::new(),
                measured: 0.0,
                touched: 0,
                first: before,
            };
            episode.add(&frame, self.fingers > 0);
            self.episode = Some(episode);
        }
        ended
    }
}

impl Episode {
    fn add(&mut self, frame: &Frame, touched: bool) {
        self.work.push(frame.work_ns);
        let background = &frame.background;
        self.background.push(background.spent as i64);
        self.measured += background.measured.unwrap_or(0.0);
        self.touched += usize::from(touched);
    }

    fn summary(self, end_ns: i64, counts: Counts) -> Summary {
        Summary {
            frames: self.work.len(),
            touched: self.touched,
            span_ms: (end_ns - self.started_ns) / 1_000_000,
            work: Spread::of(self.work),
            background: Spread::of(self.background),
            measured_ms: self.measured / 1e6,
            counts: counts.since(self.first),
        }
    }
}

/// One episode's frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub frames: usize,
    /// Frames with a finger on the glass.
    pub touched: usize,
    pub span_ms: i64,
    pub work: Spread,
    /// The background work a frame, by the budget's corrected estimate.
    pub background: Spread,
    /// GPU time measured for background work over the episode.
    pub measured_ms: f64,
    pub counts: Counts,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = &self.counts;
        write!(
            f,
            "navigation, {} frames ({} with fingers) over {} ms: frame work {}; background a \
             frame {}, measured {:.0} ms in all; {} dispatches of {} entries, {} plans binning \
             {} entries, {} residuals derived, {} tier tiles filled",
            self.frames,
            self.touched,
            self.span_ms,
            self.work,
            self.background,
            self.measured_ms,
            c.dispatches,
            c.entries,
            c.plans,
            c.binned,
            c.derived,
            c.filled
        )
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
