//! The drawing order (S163, D125): an artwork's records in the order they were made, and the
//! artwork as it stood after any number of them, which the replay and its video show. Pure: the
//! times are the ones the records keep, never a clock's.
//!
//! The steps. Every stroke and every edit of the record is one step, wherever it lies: a layer's
//! paint or its mask, or the content of a merged layer's record (`NestedEdit`, D66) at any depth;
//! the merged record itself is no step. A stroke's place in the order is its id: ids grow with
//! every stroke drawn along a version's history, so in one version they never repeat and a later
//! stroke has the higher one. An edit's place is the artwork's next stroke id when it was made
//! (format 2 keeps it, D125): it comes after every stroke of a lower id and before the stroke of
//! that id. Edits with the same place keep their list's order; across lists the lower layer comes
//! first, a layer's paint before its mask, a list before the merged records it holds. Strokes a
//! layer's duplicate or a paste renumbered take their place by their new ids (they keep their
//! start times: the timeline draws such copies at once, `Timeline`), and so do the edits among
//! them.
//!
//! What never shows: what is not in the record (undone steps, deleted layers, what a cut emptied).
//! Layers, groups and masks keep their final properties throughout; a step whose list does not
//! show in the finished artwork (its layer or group hidden, its mask disabled) is listed with
//! `Step::shows` false, and the timeline gives it no time.
//!
//! The artwork after `done` steps (`DrawingOrder::frame`): every list cut to the records of the
//! first `done` steps (a list's records are in the order's order, so each is cut to a prefix),
//! each edit's place among its list's strokes kept (`after`, at most the strokes kept); a merged
//! layer's record lies in its list as soon as any of its content does, its content cut alike. The
//! stroke of step `done`, drawn so far, is its first `samples` samples as the stroke being drawn
//! (`ActiveStroke`) on its own layer's paint or mask (`Step::live`), so the renderer draws it by its
//! live path; a stroke inside a merged record has no such place and shows whole once its step is
//! done. `done` equal to the step count gives the finished artwork, the very same version.
//! Consecutive frames share every stroke and edit both hold (the same `Arc`s, `Stroke::shares_samples`,
//! `RegionEdit::same`), so a frame that only adds strokes is an append for the renderer
//! (`changed_regions`, the scene's append).

use crate::artwork::Artwork;
use crate::id::{LayerId, StrokeId};
use crate::session::ActiveStroke;

/// One record of an artwork in the order it was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    /// The layer whose paint, mask or merged record holds it.
    pub layer: LayerId,
    /// Its list shows in the finished artwork (its layer and group shown, its mask enabled).
    pub shows: bool,
    /// A stroke on its layer's paint or mask, drawn along its samples (`DrawingOrder::frame`);
    /// false for an edit and for a stroke inside a merged layer's record.
    pub live: bool,
}

/// What a step makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Stroke {
        id: StrokeId,
        /// When its first sample was drawn, in milliseconds since 1970 UTC; `None` for a stroke
        /// kept before format 2 (D125).
        began_ms: Option<u64>,
        /// How many samples it holds (at least one).
        samples: usize,
        /// Its last sample's time from its first, in milliseconds: how long the pen took.
        drawn_ms: u32,
    },
    Edit(EditKind),
}

/// The kinds of edit a step makes (a merged layer's record is no step: its content is).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Fill,
    /// Pixels cleared: a cut, or what a copy left outside its selection.
    Clear,
    Transform,
    Adjust,
}

/// An artwork's records in the order they were made (see the module). Clone is cheap but for the
/// step list; built once when the replay opens, O(records).
#[derive(Debug, Clone)]
pub struct DrawingOrder {
    artwork: Artwork,
    steps: Vec<Step>,
}

/// What the replay shows at one moment: the artwork after the steps done, and the stroke being
/// drawn so far, as the renderer's `FrameRequest` takes them.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayFrame {
    pub artwork: Artwork,
    pub active: Option<ActiveStroke>,
}

impl Artwork {
    /// Its records in the order they were made.
    #[must_use]
    pub fn drawing_order(&self) -> DrawingOrder {
        // S163a: every record's step, in the order the module states.
        DrawingOrder {
            artwork: self.clone(),
            steps: Vec::new(),
        }
    }

    /// How many strokes its record holds: every layer's paint and mask, and the content of
    /// merged layers' records at any depth. O(layers + edits), never O(strokes).
    #[must_use]
    pub fn stroke_count(&self) -> usize {
        // S163a: masks and merged layers' records too.
        self.layers()
            .iter()
            .map(|layer| layer.strokes().len())
            .sum()
    }
}

impl DrawingOrder {
    /// The finished artwork, as the order was made from.
    #[must_use]
    pub const fn artwork(&self) -> &Artwork {
        &self.artwork
    }

    /// Every step, in order.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The artwork after the first `done` steps, with step `done`'s stroke drawn to its first
    /// `samples` samples when it is live and `samples` is at least one (see the module); `done`
    /// at or past the step count gives the finished artwork.
    #[must_use]
    pub fn frame(&self, done: usize, samples: usize) -> ReplayFrame {
        // S163a: the lists cut to the steps done, the stroke drawn so far.
        let _ = (done, samples);
        ReplayFrame {
            artwork: self.artwork.clone(),
            active: None,
        }
    }
}
