//! Nested layers (S39, Merge Down): a layer merged into the one below it becomes a record among
//! that layer's edits, at its place after the strokes the lower layer held then. The record keeps
//! the merged layer's own content (strokes, edits, an enabled mask) and how it composited
//! (opacity, blend mode, whether it clipped to the lower layer), so the merged layer shows
//! exactly what the two showed, and strokes stay the truth: nothing is baked. It is evaluated
//! where it sits as a one-layer composite into the layer so far: its content alone, its mask,
//! its opacity, then by its mode over (or, clipped, atop) what the layer held before it. Later
//! strokes, fills, transforms, adjustments and filters of the merged layer act on the whole of
//! it, the nested part included. A nested record never changes; it may hold nested records of
//! its own (a merged layer merged again), at most `MAX_NESTING` deep.

use std::sync::Arc;

use crate::blend::BlendMode;
use crate::id::{IdCounters, LayerId};
use crate::mask::LayerMask;
use crate::region::RegionEdit;
use crate::stroke_list::StrokeList;

/// How deep nested records may lie inside one another (the evaluator sets each aside on a stack
/// it shares with moved selections).
pub const MAX_NESTING: usize = 8;

/// A merged layer's content and how it composited (S39).
#[derive(Debug, Clone, PartialEq)]
pub struct NestedLayer {
    /// The merged layer's id, never reused: its strokes and edits are stored under it.
    pub(crate) id: LayerId,
    pub(crate) opacity: f32,
    pub(crate) blend: BlendMode,
    /// It clipped to the lower layer: it shows atop what that holds before it, its alpha kept.
    pub(crate) clipped: bool,
    pub(crate) strokes: StrokeList,
    /// Oldest first, each after its `after` strokes; nested records among them.
    pub(crate) edits: Arc<[RegionEdit]>,
    /// Applied to the content alone, before its opacity; only an enabled one is kept.
    pub(crate) mask: Option<LayerMask>,
}

impl NestedLayer {
    #[must_use]
    pub const fn id(&self) -> LayerId {
        self.id
    }

    /// In `0..=1`.
    #[must_use]
    pub const fn opacity(&self) -> f32 {
        self.opacity
    }

    #[must_use]
    pub const fn blend(&self) -> BlendMode {
        self.blend
    }

    /// Whether it shows atop the content before it (it clipped to that layer) rather than over.
    #[must_use]
    pub const fn clipped(&self) -> bool {
        self.clipped
    }

    #[must_use]
    pub const fn strokes(&self) -> &StrokeList {
        &self.strokes
    }

    #[must_use]
    pub fn edits(&self) -> &[RegionEdit] {
        &self.edits
    }

    #[must_use]
    pub const fn mask(&self) -> Option<&LayerMask> {
        self.mask.as_ref()
    }

    /// How deep records lie in it, itself counted: 1 when it holds no nested record.
    #[must_use]
    pub fn depth(&self) -> usize {
        1 + nesting(&self.edits)
    }

    /// The same content under a new id and new stroke ids from `ids` (a duplicated layer's).
    pub(crate) fn renumbered(&self, ids: &mut IdCounters) -> Self {
        let mut copy = self.clone();
        copy.id = ids.take_layer();
        copy.strokes = self.strokes.renumbered(ids);
        copy.edits = renumbered(&self.edits, ids);
        if let Some(mask) = &mut copy.mask {
            mask.strokes = mask.strokes.renumbered(ids);
        }
        copy
    }
}

/// A nested layer at its place among a layer's strokes.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedEdit {
    pub(crate) after: usize,
    pub(crate) layer: Arc<NestedLayer>,
}

impl NestedEdit {
    /// How many of the layer's strokes lie under it.
    #[must_use]
    pub const fn after(&self) -> usize {
        self.after
    }

    #[must_use]
    pub fn layer(&self) -> &NestedLayer {
        &self.layer
    }

    /// The same record seen from two versions: one shared content at one place. O(1).
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        self.after == other.after && Arc::ptr_eq(&self.layer, &other.layer)
    }

    /// Bytes for history budgets: the record alone; its strokes are the merged layer's, shared
    /// with the version before the merge.
    pub(crate) fn bytes_since(&self, old: Option<&Self>) -> usize {
        if old.is_some_and(|old| Arc::ptr_eq(&old.layer, &self.layer)) {
            0
        } else {
            size_of::<NestedLayer>()
        }
    }
}

/// How deep nested records lie among `edits`: 0 without any.
pub(crate) fn nesting(edits: &[RegionEdit]) -> usize {
    edits
        .iter()
        .filter_map(RegionEdit::nested)
        .map(|nested| nested.layer.depth())
        .max()
        .unwrap_or(0)
}

/// `edits` with every nested record renumbered from `ids` (a duplicated layer's); the very same
/// list when it holds none.
pub(crate) fn renumbered(edits: &Arc<[RegionEdit]>, ids: &mut IdCounters) -> Arc<[RegionEdit]> {
    if edits.iter().all(|edit| edit.nested().is_none()) {
        return Arc::clone(edits);
    }
    edits
        .iter()
        .map(|edit| match edit {
            RegionEdit::Nested(nested) => RegionEdit::Nested(NestedEdit {
                after: nested.after,
                layer: Arc::new(nested.layer.renumbered(ids)),
            }),
            other => other.clone(),
        })
        .collect()
}
