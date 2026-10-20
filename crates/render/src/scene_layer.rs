//! One layer's ops in a scene: its paint (strokes, region edits, smudges, the in-progress
//! stroke's place; `scene_content.rs`), then with an enabled mask `BeginMask` and the mask's own
//! ops, then its end, which applies the mask (S24).

use openpaint_core::{LayerId, LayerMask, RegionEdit, StrokeList};

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
}
