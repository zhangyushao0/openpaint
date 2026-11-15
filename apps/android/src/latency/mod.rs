//! Pen latency as software sees it on the device, logged once per stroke. For every pen event
//! from touch-down to lift: its event time (the kernel's input timestamp) to the start of the
//! frame that consumes it, and to the moment that frame's `present` call returns (the buffer
//! is queued to the compositor); and each such frame's own work.
//!
//! Not covered: the digitizer and its firmware before the kernel stamps the event, the GPU
//! finishing the frame, the compositor's latch and composition, and the display's scan-out.
//! All times are `CLOCK_MONOTONIC` nanoseconds (Android's `eventTimeNanos`, `System.nanoTime`).
//! Beside them: each frame's waits for the GPU and scene building, the frames skipped while the
//! GPU was behind (frame pacing) and the most frames in flight at a frame's start. For a stroke
//! that painted, where its frames went (`Painted`): the canvas's drawing and the view tier's
//! (CPU; the tier's apart at one chain and at more), the pass chains each drew (each slowest frame
//! names its own), the snapshot parts each took and the strokes each laid again over snapshots
//! that lagged. The GPU's time for a frame is not here: frames are timed on the GPU
//! only while a benchmark turns it on (`Core::time_frames`), an extra submission and readback a
//! frame.

mod line;
#[cfg(test)]
mod tests;

use openpaint_runtime::{MotionBatch, MotionPhase, Paced, Spent, StrokeDrew, ToolKind};

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
    /// What the stroke's drawing drew onto the canvas and onto the view tier shown in every
    /// frame drawn since the pen touched down, those consuming none of its events included.
    canvas: StrokeDrew,
    tier: StrokeDrew,
}

/// What a drawn frame spent of its work: drawing the canvas (and of that building scenes) and the
/// stroke into the view tier shown, its waits for the GPU (acquiring, submitting, presenting,
/// readbacks) and the longest one's place, the frames in flight as it began, and what the
/// stroke's drawing drew onto the canvas and onto the tier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameCost {
    pub drawn_ns: i64,
    pub tier_ns: i64,
    pub scene_ns: i64,
    pub waited_ns: i64,
    pub longest: &'static str,
    pub in_flight: u64,
    pub canvas: StrokeDrew,
    pub tier: StrokeDrew,
}

impl FrameCost {
    /// What the frame `paced` describes spent, as the frame loop counted it.
    #[must_use]
    pub fn of(spent: &Spent, paced: Paced) -> Self {
        let ns = |ns: u64| i64::try_from(ns).unwrap_or(i64::MAX);
        Self {
            drawn_ns: ns(spent.drawn_ns),
            tier_ns: ns(spent.tier_ns),
            scene_ns: ns(spent.scene_ns),
            waited_ns: ns(spent.waited_ns),
            longest: spent.longest,
            in_flight: paced.in_flight,
            canvas: spent.canvas,
            tier: spent.tier,
        }
    }
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

    /// A frame call was skipped while the GPU was behind: the stroke's events wait for the next
    /// frame drawn.
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
        stroke.canvas = stroke.canvas.plus(cost.canvas);
        stroke.tier = stroke.tier.plus(cost.tier);
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
        self.stroke.take().map(Summary::of)
    }
}

/// The stroke's three slowest frames, slowest first: which they were (0: the pen touching down),
/// their work and what they spent beside it (which frames hold a stroke's 95th percentile, and
/// why).
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
    /// frame's start.
    pub skipped: usize,
    pub in_flight: u64,
    /// Event time to the start of the frame consuming it.
    pub to_frame: Spread,
    /// Event time to the return of that frame's present call.
    pub to_present: Spread,
    /// Frame start to present return, per frame that consumed pen events.
    pub frame_work: Spread,
    /// Per such frame, its waits for the GPU.
    pub waited: Spread,
    /// Where its frames went, when the stroke painted (its drawing drew a region).
    pub painted: Option<Painted>,
    /// The three slowest of those frames: their place in the stroke (0 first), work and cost.
    pub slowest: [Slow; 3],
}

impl Summary {
    fn of(stroke: StrokeTimes) -> Self {
        let (painted, slowest) = (
            Painted::of(&stroke),
            slowest(&stroke.frame_work, &stroke.costs),
        );
        Self {
            events: stroke.to_frame.len(),
            frames: stroke.frame_work.len(),
            skipped: stroke.skipped,
            in_flight: (stroke.costs.iter())
                .map(|c| c.in_flight)
                .max()
                .unwrap_or(0),
            to_frame: Spread::of(stroke.to_frame),
            to_present: Spread::of(stroke.to_present),
            frame_work: Spread::of(stroke.frame_work),
            waited: Spread::of(stroke.costs.iter().map(|c| c.waited_ns).collect()),
            painted,
            slowest,
        }
    }
}

/// Where a painting stroke's frames went, per frame that consumed pen events: drawing the canvas
/// (the log's "evaluation") and the stroke into the view tier shown (CPU, their waits included),
/// the tier's apart for the frames it drew in one chain and in more (with how many of each), the
/// pass chains (regions) each drew and the frames the canvas drew any in; over the stroke, what
/// each drew of snapshots and strokes laid again (`StrokeDrew`'s, its regions unused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Painted {
    pub evaluation: Spread,
    pub tier_ns: Spread,
    pub tier_one: (Spread, usize),
    pub tier_more: (Spread, usize),
    pub canvas_chains: Spread,
    pub tier_chains: Spread,
    pub canvas_frames: usize,
    pub canvas: StrokeDrew,
    pub tier: StrokeDrew,
}

impl Painted {
    /// `None` when no frame of `stroke` drew a region.
    fn of(stroke: &StrokeTimes) -> Option<Self> {
        if stroke.canvas.regions == 0 && stroke.tier.regions == 0 {
            return None;
        }
        let each =
            |value: fn(&FrameCost) -> i64| Spread::of(stroke.costs.iter().map(value).collect());
        let tier = |chains: fn(u32) -> bool| {
            let times: Vec<i64> = (stroke.costs.iter())
                .filter(|c| chains(c.tier.regions))
                .map(|c| c.tier_ns)
                .collect();
            let frames = times.len();
            (Spread::of(times), frames)
        };
        Some(Self {
            evaluation: each(|c| c.drawn_ns),
            tier_ns: each(|c| c.tier_ns),
            tier_one: tier(|chains| chains == 1),
            tier_more: tier(|chains| chains > 1),
            canvas_chains: each(|c| i64::from(c.canvas.regions)),
            tier_chains: each(|c| i64::from(c.tier.regions)),
            canvas_frames: (stroke.costs.iter())
                .filter(|c| c.canvas.regions > 0)
                .count(),
            canvas: stroke.canvas,
            tier: stroke.tier,
        })
    }
}

/// Median, 95th percentile (nearest rank) and maximum, in nanoseconds (or a count).
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
