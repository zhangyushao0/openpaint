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
//!   waits `PAUSE_MS`: an even pace.
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

use openpaint_core::DrawingOrder;

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
    /// Where each sitting after the first begins.
    sittings: Vec<u64>,
    total_ms: u64,
    facts: ReplayFacts,
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
        // S163a: the pace, the sittings and the facts.
        let artwork = order.artwork();
        Self {
            steps: Vec::new(),
            sittings: Vec::new(),
            total_ms: 0,
            facts: ReplayFacts {
                strokes: artwork.stroke_count(),
                layers: artwork.layers().len(),
                spent_ms: None,
                began_ms: None,
            },
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
        // S163a: the steps done and the samples drawn so far.
        let _ = (order, ms);
        At::default()
    }

    /// The numbers the video's last line states.
    #[must_use]
    pub const fn facts(&self) -> ReplayFacts {
        self.facts
    }
}
