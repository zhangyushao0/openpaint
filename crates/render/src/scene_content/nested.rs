//! A merged layer's content (a nested record) among its layer's ops: what
//! the layer holds so far is set aside (`BeginMove`, as a moved selection's start), the nested
//! content's own ops follow as a level of their own (its transform records, smudges, filters and
//! nested records drawn as they were on its layer), then its mask's, and its end (`EndNested`)
//! composites it onto what was set aside by the very expressions its layer's end used before the
//! merge: atop for a layer that clipped to the one below, else over by its mode. Later items of
//! the layer then act on the whole.

use openpaint_core::{LayerId, NestedLayer, RegionEdit, StrokeList};

use super::{Content, View, level_extents};
use crate::affine::Affine;
use crate::error::RenderError;
use crate::geom::Bounds;
use crate::protocol::{Masking, Op};
use crate::scene::{Item, Scene};

/// A frame no view equals (`NaN` never compares equal), so the next `set_frame` lists one.
const UNSET: (Affine, Option<Bounds>) = (Affine::IDENTITY, Some(Bounds([f32::NAN; 4])));

impl Scene {
    /// `nested` where it lies in its layer's content, seen in `view`.
    pub(super) fn push_nested(
        &mut self,
        nested: &NestedLayer,
        view: View,
    ) -> Result<(), RenderError> {
        self.items.push(Item::BeginMove(Op::begin_move()));
        let id = nested.id();
        self.push_part(id, (nested.strokes(), nested.edits()), false, view)?;
        let masking = match nested.mask() {
            Some(mask) => {
                self.items.push(Item::BeginMask(Op::begin_mask()));
                self.push_part(id, (mask.strokes(), mask.edits()), true, view)?;
                if mask.inverted() {
                    Masking::Inverted
                } else {
                    Masking::Shown
                }
            }
            None => Masking::None,
        };
        let look = (nested.opacity(), nested.blend().into());
        let end = Op::end_nested(look, (nested.clipped(), masking));
        self.items.push(Item::EndNested(end));
        self.set_frame(view);
        Ok(())
    }

    /// One part of a nested content (its paint, or `on_mask` its mask's) as a level of its own,
    /// keyed by the merged layer's id, starting with its frame: a smudge's or filter's material
    /// is drawn from its level's first item.
    fn push_part(
        &mut self,
        layer: LayerId,
        (strokes, edits): (&StrokeList, &[RegionEdit]),
        on_mask: bool,
        view: View,
    ) -> Result<(), RenderError> {
        let content = Content {
            layer,
            strokes,
            edits,
            on_mask,
            start: self.items.len(),
            extents: level_extents(strokes, edits, self.canvas),
        };
        self.frame = UNSET;
        self.push_level(&content, (strokes.len(), edits.len()), view)
    }
}
