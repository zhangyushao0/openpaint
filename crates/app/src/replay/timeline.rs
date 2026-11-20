//! The replay's time (S163): when each step of the drawing order (`DrawingOrder`) begins and how
//! long it takes at 1×, in milliseconds, and what a moment shows. Pure; the numbers are the
//! record's own.
//!
//! The pace, at 1×:
//! - A stroke takes its pen's own time (`Step` `drawn_ms`, at least `LEAST_STROKE_MS`); its
//!   samples appear as their times come (the samples drawn are those whose time from the stroke's
//!   first is at most the time since its step began, at least one), so the tip moves as the hand
//!   did.
//! - Before a stroke, the pause the hand took (its start minus the end of the stroke before it
//!   with a time) cut to `PAUSE_MS`; a stroke without a time (kept before format 2), or after one,
//!   waits `PAUSE_MS`: an even pace. The first stroke waits for nothing.
//! - A stroke whose start lies before the latest start so far (a copy a duplicate or a paste
//!   renumbered) takes no time and no pause: copies appear at once with the step before them.
//! - An edit takes `EDIT_MS`, no pause before it; it shows from its step's start.
//! - A step that does not show (`Step::shows` false) takes no time.
//! - A sitting begins where the pause the hand took reached `SITTING_MS`; each but the first is a
//!   mark on the track at its first step's start.
//!
//! What a moment `t` shows (`at`): the steps done are the edits begun by `t` and the strokes ended
//! by `t`; the next step, a stroke begun by `t`, is drawn to the samples its time reached; during
//! a pause it is not begun. `total_ms` is the last step's end: the finished artwork.
//!
//! The facts the video's last line states (`ReplayFacts`): the record's strokes
//! (`Artwork::stroke_count`) and layers, the time spent (each stroke's own time plus every pause
//! shorter than `SITTING_MS`, over strokes with a time), the first and the last start; time and
//! dates only when a stroke has a time.

use openpaint_core::{DrawingOrder, Step, StepKind};

/// The longest pause kept between two strokes at 1×; a stroke without a time waits this long.
pub const PAUSE_MS: u64 = 300;
/// A pause from this long on begins a new sitting (a mark on the track).
pub const SITTING_MS: u64 = 30 * 60 * 1000;
/// An edit's step at 1×.
pub const EDIT_MS: u64 = 400;
/// The least time a stroke's step takes at 1× (a dot).
pub const LEAST_STROKE_MS: u64 = 30;

/// The replay's time of one drawing order.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// Per step: when it begins and ends at 1× (an edit's end is its begin plus `EDIT_MS`).
    steps: Vec<[u64; 2]>,
    /// Per step: from when it is done (an edit from its begin, a stroke from its end); never
    /// decreasing.
    done_from: Vec<u64>,
    /// Where each sitting after the first begins.
    sittings: Vec<u64>,
    total_ms: u64,
    facts: ReplayFacts,
}

/// The pace as it is laid step by step (see the module).
#[derive(Debug, Default)]
struct Pace {
    /// The end of the steps laid so far.
    at: u64,
    /// The latest start so far: a stroke begun before it is a copy.
    latest: Option<u64>,
    /// The stroke before (copies aside): when it ended, `None` without a time; `None` before
    /// the first.
    before: Option<Option<u64>>,
    /// A pause of a sitting was met: the next stroke shown marks its start.
    sitting: bool,
    steps: Vec<[u64; 2]>,
    done_from: Vec<u64>,
    sittings: Vec<u64>,
}

impl Pace {
    fn step(&mut self, step: &Step) {
        let span = match step.kind {
            StepKind::Edit(_) if step.shows => [self.at, self.at + EDIT_MS],
            StepKind::Edit(_) => [self.at, self.at],
            StepKind::Stroke {
                began_ms, drawn_ms, ..
            } => self.stroke(began_ms, drawn_ms, step.shows),
        };
        self.at = span[1];
        let done = match step.kind {
            StepKind::Edit(_) => span[0],
            StepKind::Stroke { .. } => span[1],
        };
        self.steps.push(span);
        self.done_from.push(done);
    }

    /// A stroke's span: a copy at once, one not shown at once (its pause and sitting kept), else
    /// after its pause, for its pen's time.
    fn stroke(&mut self, began: Option<u64>, drawn_ms: u32, shows: bool) -> [u64; 2] {
        if began
            .zip(self.latest)
            .is_some_and(|(began, latest)| began < latest)
        {
            return [self.at, self.at];
        }
        let pause = match (self.before, began) {
            (None, _) => 0,
            (Some(Some(end)), Some(began)) => {
                let gap = began.saturating_sub(end);
                self.sitting |= gap >= SITTING_MS;
                gap.min(PAUSE_MS)
            }
            _ => PAUSE_MS,
        };
        let end = began.map(|began| {
            let end = began + u64::from(drawn_ms);
            self.before.flatten().map_or(end, |before| before.max(end))
        });
        (self.before, self.latest) = (Some(end), began.or(self.latest));
        if !shows {
            return [self.at, self.at];
        }
        let begin = self.at + pause;
        if std::mem::take(&mut self.sitting) {
            self.sittings.push(begin);
        }
        [begin, begin + u64::from(drawn_ms).max(LEAST_STROKE_MS)]
    }
}

/// The facts of `order` (see the module): time spent counts the strokes' own time and the pauses
/// shorter than a sitting's, once where strokes overlap (mirrored copies); copies count no time.
fn facts(order: &DrawingOrder) -> ReplayFacts {
    let (mut spent, mut end, mut latest, mut range) = (0, None::<u64>, None, None::<[u64; 2]>);
    for step in order.steps() {
        let StepKind::Stroke {
            began_ms: Some(began),
            drawn_ms,
            ..
        } = step.kind
        else {
            continue;
        };
        range = Some(range.map_or([began, began], |[first, last]| {
            [first.min(began), last.max(began)]
        }));
        if latest.is_some_and(|latest| began < latest) {
            continue;
        }
        latest = Some(began);
        let finish = began + u64::from(drawn_ms);
        spent += match end {
            Some(end) if began >= end && began - end < SITTING_MS => finish - end,
            Some(end) if began >= end => finish - began,
            Some(end) => finish.saturating_sub(end),
            None => finish - began,
        };
        end = Some(end.map_or(finish, |end| end.max(finish)));
    }
    let artwork = order.artwork();
    ReplayFacts {
        strokes: artwork.stroke_count(),
        layers: artwork.layers().len(),
        spent_ms: range.map(|_| spent),
        began_ms: range,
    }
}

/// What the replay shows at one moment: the steps done, and the samples of the next drawn so far
/// (0: not begun; `DrawingOrder::frame` takes both).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct At {
    pub done: usize,
    pub samples: usize,
}

/// The numbers the video's last line states (the UI words them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReplayFacts {
    /// Every stroke of the record (`Artwork::stroke_count`).
    pub strokes: usize,
    /// The artwork's layers.
    pub layers: usize,
    /// The time spent drawing, in ms; `None` without a stroke with a time.
    pub spent_ms: Option<u64>,
    /// The first and the last stroke's starts, ms since 1970 UTC; `None` without one.
    pub began_ms: Option<[u64; 2]>,
}

impl Timeline {
    /// The time of `order`, by the pace the module states.
    #[must_use]
    pub fn of(order: &DrawingOrder) -> Self {
        let mut pace = Pace::default();
        for step in order.steps() {
            pace.step(step);
        }
        Self {
            steps: pace.steps,
            done_from: pace.done_from,
            sittings: pace.sittings,
            total_ms: pace.at,
            facts: facts(order),
        }
    }

    /// The finished artwork's moment: the last step's end.
    #[must_use]
    pub const fn total_ms(&self) -> u64 {
        self.total_ms
    }

    /// Where each sitting after the first begins.
    #[must_use]
    pub fn sittings(&self) -> &[u64] {
        &self.sittings
    }

    /// When step `index` begins at 1×, if there is one.
    #[must_use]
    pub fn begins(&self, index: usize) -> Option<u64> {
        self.steps.get(index).map(|[begins, _]| *begins)
    }

    /// What the replay shows at `ms` (clamped to the whole), given `order` the timeline was
    /// made of.
    #[must_use]
    pub fn at(&self, order: &DrawingOrder, ms: u64) -> At {
        let at = ms.min(self.total_ms);
        let done = self.done_from.partition_point(|from| *from <= at);
        let begun = (self.steps.get(done)).is_some_and(|[begin, _]| *begin <= at);
        let live = order.steps().get(done).is_some_and(|step| step.live);
        let samples = match order.stroke(done) {
            Some(stroke) if begun && live => {
                let samples = stroke.samples();
                let first = samples.first().map_or(0, |sample| sample.time_ms());
                let since = at - self.steps[done][0];
                let reached =
                    samples.partition_point(|sample| u64::from(sample.time_ms() - first) <= since);
                reached.max(1)
            }
            _ => 0,
        };
        At { done, samples }
    }

    /// The numbers the video's last line states.
    #[must_use]
    pub const fn facts(&self) -> ReplayFacts {
        self.facts
    }
}
