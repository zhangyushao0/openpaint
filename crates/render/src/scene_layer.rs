//! One layer's ops in a scene: its paint (strokes, region edits, smudges, the in-progress
//! stroke's place), then with an enabled mask `BeginMask` and the mask's own ops, then its end,
//! which applies the mask (S24).

use openpaint_core::{
    LayerId, LayerMask, RegionEdit, StrokeBlend, StrokeList, StrokeSample, StrokeStyle, StrokeTool,
};

use crate::edits::restart;
use crate::error::RenderError;
use crate::protocol::{Masking, Op, Paint};
use crate::scene::{Item, Scene, SmudgeItem};
use crate::smudge::smudge_reach;
use crate::stroke::{paint, segments};

/// What one layer draws: its id, its paint's strokes and edits (from where a transform of the
/// whole layer restarts them), and its mask, if it applies.
pub(crate) struct LayerOps<'a> {
    pub(crate) id: LayerId,
    pub(crate) strokes: &'a StrokeList,
    pub(crate) edits: &'a [RegionEdit],
    /// The first stroke drawn (after a transform of the whole layer).
    pub(crate) after: usize,
    pub(crate) mask: Option<&'a LayerMask>,
}

impl<'a> LayerOps<'a> {
    /// `layer`'s paint from where its last whole-layer transform restarts it, with `mask`.
    pub(crate) fn of(layer: &'a openpaint_core::Layer, mask: Option<&'a LayerMask>) -> Self {
        let (first, after) = restart(layer);
        Self {
            id: layer.id(),
            strokes: layer.strokes(),
            edits: &layer.edits()[first..],
            after,
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
        self.push_content(layer.id, paint, layer.after, active == Some(false), false)?;
        if self.items.len() == start {
            return Ok(());
        }
        if let Some(mask) = layer.mask {
            self.items.push(Item::BeginMask(Op::begin_mask()));
            let content = (mask.strokes(), mask.edits());
            self.push_content(layer.id, content, 0, active == Some(true), true)?;
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

    /// Strokes from `after` on with each edit after the strokes it was made over, then the
    /// in-progress stroke's place when `active`. `on_mask` for a mask's ops.
    pub(crate) fn push_content(
        &mut self,
        layer: LayerId,
        (strokes, edits): (&StrokeList, &[RegionEdit]),
        after: usize,
        active: bool,
        on_mask: bool,
    ) -> Result<(), RenderError> {
        let start = self.items.len();
        let mut edits = edits.iter().peekable();
        let newest = strokes.len().checked_sub(1);
        for (index, stroke) in strokes.iter().enumerate().skip(after) {
            while let Some(edit) = edits.next_if(|edit| edit.after() == index) {
                self.push_edit(edit, on_mask)?;
            }
            let clip = stroke.clip().map(|clip| self.clip(clip)).transpose()?;
            if stroke.style().tool() == StrokeTool::Smudge {
                let item = SmudgeItem {
                    layer,
                    on_mask,
                    stroke: stroke.clone(),
                    before: start..self.items.len(),
                    newest: newest == Some(index),
                };
                self.push_smudge(item, clip.unwrap_or(0));
                continue;
            }
            let mut paint = paint(stroke.style(), stroke.blend());
            paint.clip = clip.unwrap_or(0);
            self.push_stroke(stroke.style(), &paint, stroke.samples());
        }
        for edit in edits {
            self.push_edit(edit, on_mask)?;
        }
        if active {
            self.active_before = Some(start..self.items.len());
            self.items.push(Item::Active);
        }
        Ok(())
    }

    fn push_stroke(&mut self, style: &StrokeStyle, paint: &Paint, samples: &[StrokeSample]) {
        for segment in segments(style, samples, 0) {
            self.items.push(Item::Segment {
                op: segment.op(),
                bounds: segment.bounds(),
            });
        }
        self.items.push(Item::EndStroke(Op::end_stroke(paint)));
    }

    /// A finished smudge: its reach, then its op, which finds its residual's base in the mask
    /// word of its place among the scene's smudges.
    fn push_smudge(&mut self, item: SmudgeItem, clip: u32) {
        let stroke = &item.stroke;
        for bounds in smudge_reach(stroke.style(), stroke.samples(), 0) {
            self.items.push(Item::Reach(bounds));
        }
        let locked = stroke.blend() == StrokeBlend::SmudgeLocked;
        let at = self.bases_at + self.smudges.len() as u32;
        self.items
            .push(Item::Smudge(Op::smudge(Some(at), clip, locked)));
        self.smudges.push(item);
    }
}

/// How many smudges a scene can draw for `layer`'s ops (more is harmless: unused base words).
pub(crate) fn smudge_count(layer: &LayerOps<'_>) -> usize {
    let smudges = |strokes: &StrokeList, after: usize| {
        strokes
            .since(after)
            .filter(|stroke| stroke.style().tool() == StrokeTool::Smudge)
            .count()
    };
    smudges(layer.strokes, layer.after) + layer.mask.map_or(0, |mask| smudges(mask.strokes(), 0))
}
