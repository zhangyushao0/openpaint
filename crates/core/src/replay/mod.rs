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
//!
//! The parts: `build.rs` makes the lists and the steps, `frame.rs` cuts them.

use std::sync::{Arc, Mutex, PoisonError};

use crate::artwork::Artwork;
use crate::id::{LayerId, StrokeId};
use crate::region::RegionEdit;
use crate::session::ActiveStroke;
use crate::stroke::Stroke;
use crate::stroke_list::StrokeList;

mod build;
mod frame;

use build::{Lists, Record};

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
/// step list; built once when the replay opens, O(records log records).
#[derive(Debug)]
pub struct DrawingOrder {
    artwork: Artwork,
    steps: Vec<Step>,
    lists: Arc<Lists>,
    /// The lists as the last frame cut them, which the next shares.
    cache: Mutex<frame::Cache>,
}

impl Clone for DrawingOrder {
    fn clone(&self) -> Self {
        Self {
            artwork: self.artwork.clone(),
            steps: self.steps.clone(),
            lists: Arc::clone(&self.lists),
            cache: Mutex::new(self.cache().clone()),
        }
    }
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
        let (steps, lists) = build::order(self);
        DrawingOrder {
            artwork: self.clone(),
            steps,
            lists: Arc::new(lists),
            cache: Mutex::default(),
        }
    }

    /// How many strokes its record holds: every layer's paint and mask, and the content of
    /// merged layers' records at any depth. O(layers + edits), never O(strokes).
    #[must_use]
    pub fn stroke_count(&self) -> usize {
        self.layers()
            .iter()
            .map(|layer| {
                let mask =
                    (layer.mask.as_ref()).map_or(0, |mask| strokes_in(&mask.strokes, &mask.edits));
                strokes_in(&layer.strokes, &layer.edits) + mask
            })
            .sum()
    }
}

/// The strokes of a list and of the merged records it holds, their masks' too.
fn strokes_in(strokes: &StrokeList, edits: &[RegionEdit]) -> usize {
    let nested = edits.iter().filter_map(RegionEdit::nested).map(|nested| {
        let layer = nested.layer();
        let mask = (layer.mask.as_ref()).map_or(0, |mask| strokes_in(&mask.strokes, &mask.edits));
        strokes_in(&layer.strokes, &layer.edits) + mask
    });
    strokes.len() + nested.sum::<usize>()
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

    /// The stroke step `index` draws (its samples' times pace the replay); `None` for an edit.
    #[must_use]
    pub fn stroke(&self, index: usize) -> Option<&Stroke> {
        match self.lists.records.get(index)? {
            &(list, Record::Stroke(at)) => self.lists.lists[list].strokes.get(at),
            (_, Record::Edit(_)) => None,
        }
    }

    /// The artwork after the first `done` steps, with step `done`'s stroke drawn to its first
    /// `samples` samples when it is live and `samples` is at least one (see the module); `done`
    /// at or past the step count gives the finished artwork.
    #[must_use]
    pub fn frame(&self, done: usize, samples: usize) -> ReplayFrame {
        if done >= self.steps.len() {
            return ReplayFrame {
                artwork: self.artwork.clone(),
                active: None,
            };
        }
        let artwork = frame::artwork(&self.artwork, &self.lists, done, &mut self.cache());
        ReplayFrame {
            artwork,
            active: self.active(done, samples),
        }
    }

    /// Step `done`'s stroke drawn to its first `samples` samples, when it is live.
    fn active(&self, done: usize, samples: usize) -> Option<ActiveStroke> {
        let step = self
            .steps
            .get(done)
            .filter(|step| step.live && samples > 0)?;
        let &(list, Record::Stroke(at)) = self.lists.records.get(done)? else {
            return None;
        };
        let list = &self.lists.lists[list];
        let (index, on_mask) = list.place?;
        let stroke = list.strokes.get(at)?;
        Some(ActiveStroke::replayed(
            (step.layer, index),
            stroke,
            samples,
            on_mask,
        ))
    }

    /// The frames' cache; a frame that panicked left it whole or unused.
    fn cache(&self) -> std::sync::MutexGuard<'_, frame::Cache> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
