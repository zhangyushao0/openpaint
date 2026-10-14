//! Pen latency as software sees it on the device, logged once per stroke. For every pen event
//! from touch-down to lift: its event time (the kernel's input timestamp) to the start of the
//! frame that consumes it, and to the moment that frame's `present` call returns (the buffer
//! is queued to the compositor); and each such frame's own work.
//!
//! Not covered: the digitizer and its firmware before the kernel stamps the event, the GPU
//! finishing the frame, the compositor's latch and composition, and the display's scan-out.
//! All times are `CLOCK_MONOTONIC` nanoseconds (Android's `eventTimeNanos`, `System.nanoTime`).
//!
//! S56: each frame's waits for the GPU and scene building, the frames skipped while the GPU was
//! behind (frame pacing) and the most frames in flight at a frame's start.

use std::fmt;

use openpaint_runtime::{MotionBatch, MotionPhase, ToolKind};

/// Collects one stroke's latencies.
#[derive(Debug, Default)]
pub struct PenLatency {
    /// Event times of pen events not yet consumed by a frame.
    waiting: Vec<i64>,
    /// The stroke since the pen touched down; `None` while the pen is up.
    stroke: Option<StrokeTimes>,
}

#[derive(Debug, Default)]
struct StrokeTimes {
    to_frame: Vec<i64>,
    to_present: Vec<i64>,
    frame_work: Vec<i64>,
    costs: Vec<FrameCost>,
    skipped: usize,
    lifted: bool,
}

/// What a drawn frame spent of its work (S56): evaluating the canvas and of that building scenes,
/// its waits for the GPU (acquiring, submitting, presenting, readbacks) and the longest one's
/// place, and the frames in flight as it began.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameCost {
    pub evaluated_ns: i64,
    pub scene_ns: i64,
    pub waited_ns: i64,
    pub longest: &'static str,
    pub in_flight: u64,
}

impl PenLatency {
    /// A platform event that happened at `event_ns`.
    pub fn input(&mut self, batch: &MotionBatch, event_ns: i64) {
        let pen = batch
            .pointers
            .iter()
            .any(|pointer| matches!(pointer.tool, ToolKind::Stylus | ToolKind::Eraser));
        if !pen {
            return;
        }
        match batch.phase {
            MotionPhase::Down => {
                self.waiting.clear();
                self.stroke = Some(StrokeTimes::default());
            }
            MotionPhase::Up | MotionPhase::Cancel => {
                if let Some(stroke) = &mut self.stroke {
                    stroke.lifted = true;
                }
            }
            MotionPhase::Move => {}
            _ => return,
        }
        if self.stroke.is_some() {
            self.waiting.push(event_ns);
        }
    }

    /// A frame call was skipped while the GPU was behind (S56): the stroke's events wait for the
    /// next frame drawn.
    pub fn skipped(&mut self) {
        if let Some(stroke) = &mut self.stroke {
            stroke.skipped += 1;
        }
    }

    /// A frame ran from `start_ns` until its present call returned at `end_ns`, spending `cost`
    /// of it so; it consumed every event before it. Returns the stroke's summary once the frame
    /// took its lift.
    pub fn frame(&mut self, (start_ns, end_ns): (i64, i64), cost: FrameCost) -> Option<Summary> {
        let stroke = self.stroke.as_mut()?;
        if self.waiting.is_empty() {
            return None;
        }
        for event in self.waiting.drain(..) {
            stroke.to_frame.push(start_ns - event);
            stroke.to_present.push(end_ns - event);
        }
        stroke.frame_work.push(end_ns - start_ns);
        stroke.costs.push(cost);
        if !stroke.lifted {
            return None;
        }
        let stroke = self.stroke.take()?;
        Some(Summary {
            events: stroke.to_frame.len(),
            frames: stroke.frame_work.len(),
            skipped: stroke.skipped,
            in_flight: (stroke.costs.iter())
                .map(|c| c.in_flight)
                .max()
                .unwrap_or(0),
            to_frame: Spread::of(stroke.to_frame),
            to_present: Spread::of(stroke.to_present),
            waited: Spread::of(stroke.costs.iter().map(|c| c.waited_ns).collect()),
            slowest: slowest(&stroke.frame_work, &stroke.costs),
            frame_work: Spread::of(stroke.frame_work),
        })
    }
}

/// The stroke's three slowest frames, slowest first: which they were (0: the pen touching down),
/// their work and what they spent beside it (S55, S56: which frames hold a stroke's 95th
/// percentile, and why).
fn slowest(work: &[i64], costs: &[FrameCost]) -> [Slow; 3] {
    let mut ranked: Vec<(usize, i64)> = work.iter().copied().enumerate().collect();
    ranked.sort_by_key(|&(at, ns)| (std::cmp::Reverse(ns), at));
    let mut top = [Slow::default(); 3];
    for (slot, (at, ns)) in top.iter_mut().zip(ranked) {
        let cost = costs.get(at).copied().unwrap_or_default();
        *slot = Slow { at, ns, cost };
    }
    top
}

/// One of a stroke's slowest frames.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Slow {
    pub at: usize,
    pub ns: i64,
    pub cost: FrameCost,
}

/// One stroke's latencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub events: usize,
    pub frames: usize,
    /// Frame calls skipped while the GPU was behind, and the most frames in flight at a drawn
    /// frame's start (S56).
    pub skipped: usize,
    pub in_flight: u64,
    /// Event time to the start of the frame consuming it.
    pub to_frame: Spread,
    /// Event time to the return of that frame's present call.
    pub to_present: Spread,
    /// Frame start to present return, per frame that consumed pen events.
    pub frame_work: Spread,
    /// Per such frame, its waits for the GPU (S56).
    pub waited: Spread,
    /// The three slowest of those frames: their place in the stroke (0 first), work and cost.
    pub slowest: [Slow; 3],
}

/// Median, 95th percentile (nearest rank) and maximum, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    pub p50: i64,
    pub p95: i64,
    pub max: i64,
}

impl Spread {
    pub(crate) fn of(mut values: Vec<i64>) -> Self {
        values.sort_unstable();
        let rank = |percent: usize| {
            let index = (values.len() * percent).div_ceil(100).max(1) - 1;
            values.get(index).copied().unwrap_or(0)
        };
        Self {
            p50: rank(50),
            p95: rank(95),
            max: values.last().copied().unwrap_or(0),
        }
    }
}

impl fmt::Display for Spread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = |ns: i64| ns as f64 / 1e6;
        write!(
            f,
            "p50 {:.1} p95 {:.1} max {:.1} ms",
            ms(self.p50),
            ms(self.p95),
            ms(self.max)
        )
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = |ns: i64| ns as f64 / 1e6;
        write!(
            f,
            "pen latency, {} events in {} frames ({} skipped for the GPU, at most {} in \
             flight): event to frame start {}; event to present call {}; frame work {}; GPU \
             waits {}; slowest",
            self.events,
            self.frames,
            self.skipped,
            self.in_flight,
            self.to_frame,
            self.to_present,
            self.frame_work,
            self.waited
        )?;
        for Slow { at, ns, cost } in self.slowest {
            write!(
                f,
                " #{at} {:.1} (evaluation {:.1}, scene {:.1}, waits {:.1}",
                ms(ns),
                ms(cost.evaluated_ns),
                ms(cost.scene_ns),
                ms(cost.waited_ns)
            )?;
            if !cost.longest.is_empty() {
                write!(f, " most in {}", cost.longest)?;
            }
            write!(f, ")")?;
        }
        write!(f, " ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_runtime::{Buttons, Pointer, PointerSample};

    const MS: i64 = 1_000_000;

    fn batch(phase: MotionPhase, tool: ToolKind) -> MotionBatch {
        MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples: vec![PointerSample {
                    position: [0.0, 0.0],
                    pressure: 0.5,
                    tilt: 0.0,
                    orientation: 0.0,
                    time_ms: 0,
                }],
            }],
        }
    }

    fn cost(waited_ms: i64, scene_ms: i64, in_flight: u64) -> FrameCost {
        FrameCost {
            evaluated_ns: 2 * scene_ms * MS,
            scene_ns: scene_ms * MS,
            waited_ns: waited_ms * MS,
            longest: if waited_ms > 0 { "present" } else { "" },
            in_flight,
        }
    }

    #[test]
    fn a_stroke_is_summarised_when_the_frame_takes_its_lift() {
        let mut latency = PenLatency::default();
        latency.input(&batch(MotionPhase::Down, ToolKind::Stylus), 0);
        assert_eq!(latency.frame((4 * MS, 6 * MS), cost(1, 1, 0)), None);
        latency.input(&batch(MotionPhase::Move, ToolKind::Stylus), 10 * MS);
        latency.skipped();
        latency.input(&batch(MotionPhase::Move, ToolKind::Stylus), 12 * MS);
        assert_eq!(latency.frame((20 * MS, 23 * MS), cost(2, 0, 1)), None);
        latency.input(&batch(MotionPhase::Up, ToolKind::Stylus), 30 * MS);
        let summary = latency
            .frame((36 * MS, 38 * MS), cost(0, 1, 1))
            .expect("summary");
        assert_eq!((summary.events, summary.frames), (4, 3));
        assert_eq!((summary.skipped, summary.in_flight), (1, 1));
        // To frame start: 4, 10, 8, 6 ms; to present call: 6, 13, 11, 8; work: 2, 3, 2.
        let spread = |p50, p95, max| Spread {
            p50: p50 * MS,
            p95: p95 * MS,
            max: max * MS,
        };
        assert_eq!(summary.to_frame, spread(6, 10, 10));
        assert_eq!(summary.to_present, spread(8, 13, 13));
        assert_eq!(summary.frame_work, spread(2, 3, 3));
        assert_eq!(summary.waited, spread(1, 2, 2));
        let slow = |at, ms, cost| Slow {
            at,
            ns: ms * MS,
            cost,
        };
        assert_eq!(
            summary.slowest,
            [
                slow(1, 3, cost(2, 0, 1)),
                slow(0, 2, cost(1, 1, 0)),
                slow(2, 2, cost(0, 1, 1))
            ]
        );
        let line = summary.to_string();
        assert!(
            line.contains("(1 skipped for the GPU, at most 1 in flight)"),
            "{line}"
        );
        assert!(
            line.ends_with(
                "slowest #1 3.0 (evaluation 0.0, scene 0.0, waits 2.0 most in present) #0 2.0 \
                 (evaluation 2.0, scene 1.0, waits 1.0 most in present) #2 2.0 (evaluation 2.0, \
                 scene 1.0, waits 0.0) ms"
            ),
            "{line}"
        );
        assert_eq!(
            latency.frame((50 * MS, 52 * MS), FrameCost::default()),
            None,
            "one summary per stroke"
        );
    }

    #[test]
    fn fingers_hover_and_frames_without_pen_events_are_not_counted() {
        let mut latency = PenLatency::default();
        latency.input(&batch(MotionPhase::HoverMove, ToolKind::Stylus), 0);
        latency.input(&batch(MotionPhase::Down, ToolKind::Finger), 0);
        latency.input(&batch(MotionPhase::Up, ToolKind::Finger), 0);
        latency.skipped();
        assert_eq!(latency.frame((5 * MS, 6 * MS), FrameCost::default()), None);
        latency.input(&batch(MotionPhase::Down, ToolKind::Stylus), 10 * MS);
        latency.frame((12 * MS, 13 * MS), FrameCost::default());
        latency.frame((20 * MS, 21 * MS), FrameCost::default());
        latency.input(&batch(MotionPhase::Up, ToolKind::Stylus), 30 * MS);
        let summary = latency
            .frame((31 * MS, 32 * MS), FrameCost::default())
            .expect("summary");
        assert_eq!((summary.events, summary.frames, summary.skipped), (2, 2, 0));
    }

    #[test]
    fn the_log_line_reads_in_milliseconds() {
        let spread = Spread {
            p50: 8_240_000,
            p95: 12 * MS,
            max: 15_049_999,
        };
        assert_eq!(spread.to_string(), "p50 8.2 p95 12.0 max 15.0 ms");
    }
}
