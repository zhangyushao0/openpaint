//! A layer's region edits as scene ops. A fill paints its colour inside its mask. A transform
//! clears what it lifted (an erasing fill over its selection; for the whole layer nothing
//! before it is drawn at all, `restart`) and draws its patch through its placement (`Patch`).
//! An adjustment changes the layer's colours so far, inside its clip (`scene_adjust.rs`).

use openpaint_core::{CanvasSize, Layer, RegionEdit, RegionMask, TransformEdit};

use crate::error::RenderError;
use crate::geom::{Bounds, PixelRect};
use crate::protocol::{Blend, MASK_HEADER, Op, Paint};
use crate::scene::{Item, Scene};

impl Scene {
    /// `edit`'s ops; `on_mask` when it lies on a mask, which an adjustment needs to know.
    pub(crate) fn push_edit(
        &mut self,
        edit: &RegionEdit,
        on_mask: bool,
    ) -> Result<(), RenderError> {
        match edit {
            RegionEdit::Adjust(adjust) => self.push_adjust(adjust, on_mask),
            RegionEdit::Fill(fill) => {
                let color = fill.color();
                let blend = if fill.alpha_locked() {
                    Blend::Atop
                } else {
                    Blend::Over
                };
                let paint = Paint::solid([color.r(), color.g(), color.b()], blend);
                self.push_mask_op(fill.mask(), &paint)
            }
            RegionEdit::Transform(transform) => {
                if let Some(lift) = transform.lift() {
                    self.push_mask_op(lift, &Paint::solid([0.0; 3], Blend::Erase))?;
                }
                self.push_patch(transform)
            }
        }
    }

    /// `paint` over the layer wherever `mask` holds a pixel.
    fn push_mask_op(&mut self, mask: &RegionMask, paint: &Paint) -> Result<(), RenderError> {
        let at = self.push_mask(mask)?;
        self.items.push(Item::Fill {
            op: Op::fill(paint, at),
            bounds: mask_bounds(mask),
        });
        Ok(())
    }

    fn push_patch(&mut self, transform: &TransformEdit) -> Result<(), RenderError> {
        let patch = transform.patch();
        // Below 2^32, so its two halves stay exact.
        let at = u32::try_from(self.patch_words)
            .ok()
            .filter(|at| *at < u32::MAX)
            .ok_or(RenderError::Invalid("the patches exceed 2^32 words"))?;
        let [width, height] = patch.size();
        self.patch_words += (MASK_HEADER + width * height) as usize;
        self.patches.push(patch.clone());
        let rows = transform.placement().inverse_rows();
        self.items.push(Item::Fill {
            op: Op::patch(rows, at),
            bounds: placed_bounds(transform),
        });
        Ok(())
    }
}

/// Where a layer's ops start: past everything a transform of the whole layer cleared, as the
/// index of the last such edit and the strokes under it; `(0, 0)` without one.
pub(crate) fn restart(layer: &Layer) -> (usize, usize) {
    layer
        .edits()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, edit)| edit.transform().is_some_and(|t| t.lift().is_none()))
        .map_or((0, 0), |(index, edit)| (index, edit.after()))
}

/// A mask's pixels, from their top-left corner to past their bottom-right one.
pub(crate) fn mask_bounds(mask: &RegionMask) -> Bounds {
    let ([x, y], [width, height]) = (mask.origin(), mask.size());
    Bounds([
        x as f32,
        y as f32,
        (x + width) as f32 - 0.5,
        (y + height) as f32 - 0.5,
    ])
}

/// Where a transform's patch can show: its rectangle widened by one pixel (what bilinear
/// sampling reaches), through its placement.
fn placed_bounds(transform: &TransformEdit) -> Bounds {
    let patch = transform.patch();
    let ([x, y], [w, h]) = (patch.origin(), patch.size());
    let (x0, y0) = (x as f32 - 1.0, y as f32 - 1.0);
    let (x1, y1) = ((x + w) as f32 + 1.0, (y + h) as f32 + 1.0);
    let placement = transform.placement();
    let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]].map(|p| placement.apply(p));
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for [px, py] in corners {
        bounds = [
            bounds[0].min(px),
            bounds[1].min(py),
            bounds[2].max(px),
            bounds[3].max(py),
        ];
    }
    Bounds(bounds)
}

/// The pixels an edit can change on `canvas`: a fill's mask rectangle; for a transform what it
/// lifted (the whole canvas for the whole layer) and where its patch shows.
pub(crate) fn edit_rect(canvas: CanvasSize, edit: &RegionEdit) -> PixelRect {
    match edit {
        RegionEdit::Fill(fill) => mask_bounds(fill.mask()).pixels(),
        RegionEdit::Transform(transform) => {
            let lifted = transform.lift().map_or_else(
                || PixelRect::full([canvas.width(), canvas.height()]),
                |lift| mask_bounds(lift).pixels(),
            );
            lifted.union(&placed_rect(transform))
        }
        RegionEdit::Adjust(adjust) => adjust.clip().map_or_else(
            || PixelRect::full([canvas.width(), canvas.height()]),
            |clip| mask_bounds(clip).pixels(),
        ),
    }
}

/// Where a transform's patch shows.
pub(crate) fn placed_rect(transform: &TransformEdit) -> PixelRect {
    placed_bounds(transform).pixels()
}

/// The patch words of `scene`'s patches as GPU bytes: per patch its header, then its pixels.
pub(crate) fn patch_bytes(scene: &Scene) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(scene.patch_words * 4);
    for patch in &scene.patches {
        let ([x, y], [width, height]) = (patch.origin(), patch.size());
        for word in [x, y, width, height] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend_from_slice(patch.pixels());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use openpaint_core::{Artwork, Patch, Placement};

    use crate::scene::Scene;

    #[test]
    fn a_whole_layer_transform_leaves_out_everything_before_it() {
        let canvas = openpaint_core::CanvasSize::new(8, 8).expect("size");
        let patch = || Patch::new(canvas, [1, 1], [2, 2], vec![255; 16]).expect("patch");
        let moved = Placement::moved([2.0, 0.0]).expect("move");
        let mut artwork = Artwork::new(canvas);
        for _ in 0..3 {
            artwork = artwork
                .transform_region(None, patch(), moved)
                .expect("transform")
                .artwork;
        }
        let scene = Scene::build(&artwork, None).expect("scene");
        assert_eq!(scene.patches.len(), 1, "only the last patch");
        assert_eq!(scene.patch_words, 4 + 4);
        assert_eq!(scene.items.len(), 2, "its patch and the layer's end");
    }
}
