//! A layer's region edits as scene ops. A fill paints its colour inside its mask's smoothed
//! edge (`fill_edge.rs`). A transform with pixels clears what it lifted (an erasing fill over
//! its selection, a hard edge; for the whole layer nothing before it is drawn at all,
//! `restart`) and draws its patch through its placement (`Patch`); a transform record shows
//! the layer's earlier content where it moved it (`scene_content.rs`). An adjustment changes
//! the layer's colours so far, inside its clip (`scene_adjust.rs`).

use std::sync::Arc;

use openpaint_core::{Patch, Placement, RegionEdit, RegionMask, TransformEdit};

use crate::error::RenderError;
use crate::fill_edge::edge_words;
use crate::geom::{Bounds, PixelRect};
use crate::protocol::{Blend, MASK_HEADER, Op, Paint};
use crate::scene::{EdgeValues, Item, Scene};

impl Scene {
    /// A fill's or a transform's ops.
    pub(crate) fn push_edit(&mut self, edit: &RegionEdit) -> Result<(), RenderError> {
        match edit {
            // Adjustments know their place among the content (`push_adjust`, `push_filter`).
            RegionEdit::Adjust(_) => Err(RenderError::Invalid("an adjustment has its own place")),
            RegionEdit::Nested(_) => Err(RenderError::Invalid("a merged layer has its own place")),
            RegionEdit::Fill(fill) => {
                let color = fill.color();
                let blend = if fill.alpha_locked() {
                    Blend::Atop
                } else {
                    Blend::Over
                };
                let paint = Paint::solid([color.r(), color.g(), color.b()], blend);
                self.push_fill(fill.mask(), &paint)
            }
            RegionEdit::Transform(transform) => {
                // A record's ops come from the content it moves (`push_level`).
                let Some(patch) = transform.patch() else {
                    return Ok(());
                };
                if let Some(lift) = transform.lift() {
                    self.push_lift(lift)?;
                }
                self.push_patch(patch, transform.placement())
            }
        }
    }

    /// Clears `lift` from the layer: an erasing fill with a hard edge, as what a transform
    /// lifted holds exactly the mask's pixels.
    pub(crate) fn push_lift(&mut self, lift: &RegionMask) -> Result<(), RenderError> {
        let at = self.push_mask(lift)?;
        self.items.push(Item::Clear {
            op: Op::fill(&Paint::solid([0.0; 3], Blend::Erase), at, None),
            bounds: mask_bounds(lift),
        });
        Ok(())
    }

    /// `paint` over the layer inside `mask`'s smoothed edge: the mask's words, then its edge
    /// values, those of an earlier scene when it drew the same mask (`known`).
    fn push_fill(&mut self, mask: &RegionMask, paint: &Paint) -> Result<(), RenderError> {
        let at = self.push_mask(mask)?;
        let known = self.known.iter().chain(&self.edges);
        let words = match known.into_iter().find(|edge| edge.mask.shares(mask)) {
            Some(edge) => Arc::clone(&edge.words),
            None => {
                let started = std::time::Instant::now();
                let words = edge_words(mask, self.canvas);
                let taken = started.elapsed();
                if taken.as_millis() >= 2 {
                    log::info!(
                        "fill edge derived in {} ms: {:?} px box, {} words",
                        taken.as_millis(),
                        mask.size(),
                        words.len()
                    );
                }
                Arc::from(words)
            }
        };
        self.masks.extend_from_slice(&words);
        self.edges.push(EdgeValues {
            mask: mask.clone(),
            words,
        });
        let [x0, y0, x1, y1] = mask_bounds(mask).0;
        self.items.push(Item::Fill {
            op: Op::fill(paint, at, Some(self.canvas)),
            // The smoothed edge reaches a pixel past the mask's.
            bounds: Bounds([x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0]),
        });
        Ok(())
    }

    fn push_patch(&mut self, patch: &Patch, placement: Placement) -> Result<(), RenderError> {
        // Below 2^32, so its two halves stay exact.
        let at = u32::try_from(self.patch_words)
            .ok()
            .filter(|at| *at < u32::MAX)
            .ok_or(RenderError::Invalid("the patches exceed 2^32 words"))?;
        let [width, height] = patch.size();
        self.patch_words += (MASK_HEADER + width * height) as usize;
        self.patches.push(patch.clone());
        let rows = placement.inverse_rows();
        self.items.push(Item::Fill {
            op: Op::patch(rows, at),
            bounds: placed_bounds(patch_rect(patch), placement),
        });
        Ok(())
    }
}

/// Where content's ops start: past everything a transform of the whole layer with pixels
/// cleared, as the index of the last such edit among `edits` and the strokes under it; `(0, 0)`
/// without one.
pub(crate) fn restart(edits: &[RegionEdit]) -> (usize, usize) {
    edits
        .iter()
        .enumerate()
        .rev()
        .find(|(_, edit)| {
            edit.transform()
                .is_some_and(|t| t.lift().is_none() && t.patch().is_some())
        })
        .map_or((0, 0), |(index, edit)| (index, edit.after()))
}

/// A patch's rectangle as bounds: its pixels' area.
fn patch_rect(patch: &Patch) -> [f32; 4] {
    let ([x, y], [w, h]) = (patch.origin(), patch.size());
    [x as f32, y as f32, (x + w) as f32, (y + h) as f32]
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

/// Where the content of the rectangle `[x0, y0, x1, y1]` can show through `placement`: the
/// rectangle widened by one pixel (what bilinear sampling and an antialiased edge reach),
/// through the placement.
fn placed_bounds([x0, y0, x1, y1]: [f32; 4], placement: Placement) -> Bounds {
    let (x0, y0, x1, y1) = (x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0);
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

/// The pixels an edit can change on a canvas of `canvas` pixels: a fill's mask rectangle; for
/// a transform what it lifted (the whole canvas for the whole layer) and where its content
/// shows.
pub(crate) fn edit_rect(canvas: [u32; 2], edit: &RegionEdit) -> PixelRect {
    match edit {
        RegionEdit::Fill(fill) => {
            let rect = mask_bounds(fill.mask()).pixels();
            // Its smoothed edge reaches a pixel further.
            PixelRect {
                min: [rect.min[0] - 1, rect.min[1] - 1],
                max: [rect.max[0] + 1, rect.max[1] + 1],
            }
        }
        RegionEdit::Transform(transform) => {
            let lifted = transform.lift().map_or_else(
                || PixelRect::full(canvas),
                |lift| mask_bounds(lift).pixels(),
            );
            let moved = moved_rect(canvas, transform);
            lifted.union(&placed_rect(moved, transform.placement()))
        }
        RegionEdit::Adjust(adjust) => adjust.clip().map_or_else(
            || PixelRect::full(canvas),
            |clip| mask_bounds(clip).pixels(),
        ),
        // A merged layer's content may reach anywhere its records moved it.
        RegionEdit::Nested(_) => PixelRect::full(canvas),
    }
}

/// What a transform's content can cover before it moves, as `[x0, y0, x1, y1]`: its patch's
/// rectangle, or for a record its selection's rectangle or the canvas.
pub(crate) fn moved_rect([width, height]: [u32; 2], transform: &TransformEdit) -> [f32; 4] {
    match (transform.patch(), transform.lift()) {
        (Some(patch), _) => patch_rect(patch),
        (None, Some(lift)) => {
            let ([x, y], [w, h]) = (lift.origin(), lift.size());
            [x as f32, y as f32, (x + w) as f32, (y + h) as f32]
        }
        (None, None) => [0.0, 0.0, width as f32, height as f32],
    }
}

/// Where content within `rect` (`[x0, y0, x1, y1]`) shows through `placement`.
pub(crate) fn placed_rect(rect: [f32; 4], placement: Placement) -> PixelRect {
    placed_bounds(rect, placement).pixels()
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
