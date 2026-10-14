//! One layer's ops in a scene: its paint (strokes, region edits, smudges, the in-progress
//! stroke's place; `scene_content.rs`), then with an enabled mask `BeginMask` and the mask's own
//! ops, then its end, which applies the mask (S24).

use std::ops::Range;

use openpaint_core::{LayerId, LayerMask, RegionEdit, StrokeBlend, StrokeList};

use crate::error::RenderError;
use crate::protocol::{Masking, Op};
use crate::scene::{Item, Scene};

/// What one layer draws: its id, its paint's strokes and edits, and its mask, if it applies.
pub(crate) struct LayerOps<'a> {
    pub(crate) id: LayerId,
    pub(crate) strokes: &'a StrokeList,
    pub(crate) edits: &'a [RegionEdit],
    pub(crate) mask: Option<&'a LayerMask>,
}

impl<'a> LayerOps<'a> {
    /// `layer`'s paint with `mask`.
    pub(crate) fn of(layer: &'a openpaint_core::Layer, mask: Option<&'a LayerMask>) -> Self {
        Self {
            id: layer.id(),
            strokes: layer.strokes(),
            edits: layer.edits(),
            mask,
        }
    }
}

impl Scene {
    /// The layer's ops then `end` (an `EndLayer`, given the mask's `Masking`), when its paint
    /// has any op: a mask alone shows nothing. `active` is the in-progress stroke's place on
    /// it: `Some(false)` on its paint, `Some(true)` on its mask.
    pub(crate) fn push_layer(
        &mut self,
        layer: &LayerOps<'_>,
        active: Option<bool>,
        mut end: Item,
    ) -> Result<(), RenderError> {
        let start = self.items.len();
        let paint = (layer.strokes, layer.edits);
        self.push_content(layer.id, paint, active == Some(false), false)?;
        if self.items.len() == start {
            return Ok(());
        }
        if let Some(mask) = layer.mask {
            self.items.push(Item::BeginMask(Op::begin_mask()));
            let content = (mask.strokes(), mask.edits());
            self.push_content(layer.id, content, active == Some(true), true)?;
            let masking = if mask.inverted() {
                Masking::Inverted
            } else {
                Masking::Shown
            };
            if let Item::EndLayer { op, .. } = &mut end {
                *op = op.masked(masking);
            }
        }
        self.items.push(end);
        Ok(())
    }

    /// The items of `layer`'s paint, or (`on_mask`) its mask's, while it shows: before the
    /// in-progress stroke's place on it (S56: what a snapshot of it stands for).
    pub(crate) fn content_of(&self, layer: LayerId, on_mask: bool) -> Option<Range<usize>> {
        (self.contents.iter())
            .find(|(id, mask, _)| *id == layer && *mask == on_mask)
            .map(|(_, _, items)| items.clone())
    }

    /// The last mask words, one per smudge, then one per filter: where their blocks start in the
    /// arena, which each one's op names.
    pub(crate) fn place_bases(&mut self) -> Result<(), RenderError> {
        let count = self.smudges.len() + self.filters.len();
        let fits = |at: &u32| u32::try_from(count).is_ok_and(|n| at.checked_add(n).is_some());
        self.bases_at = u32::try_from(self.masks.len())
            .ok()
            .filter(fits)
            .ok_or(RenderError::Invalid("the masks exceed 2^32 words"))?;
        self.masks.resize(self.masks.len() + count, 0);
        for (number, smudge) in (self.bases_at..).zip(&self.smudges) {
            let locked = smudge.stroke.blend() == StrokeBlend::SmudgeLocked;
            self.items[smudge.item] = Item::Smudge(Op::smudge(Some(number), smudge.clip, locked));
        }
        let filters = self.bases_at + self.smudges.len() as u32;
        for (number, filter) in (filters..).zip(&self.filters) {
            if let Item::Fill { op, .. } = &mut self.items[filter.item] {
                *op = Op::filter(number, filter.clip, self.canvas);
            }
        }
        Ok(())
    }
}
