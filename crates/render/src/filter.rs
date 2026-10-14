//! Gaussian Blur and Sharpen (S33, DECISIONS D59): adjustments that read neighbouring pixels, so
//! no per-pixel op can make them. Each is a canvas-resolution raster boundary derived as a
//! smudge's residual is (D46): the layer's pixels just before it are evaluated by the single
//! evaluator from the layer's earlier ops in the target's own scene (`Material`), at canvas
//! resolution, then filtered on the GPU (`kernels.rs`: a separable Gaussian, across then down,
//! in bands of rows; sharpening adds `amount x STRENGTH` times the difference from it) into a
//! block of the target's arena: its rectangle, then one premultiplied RGBA8 word per pixel. The
//! evaluator's `Filter` op replaces the layer so far with that block, bilinear between pixel
//! centres at 2x and up (`filter_composite.wgsl`), inside its clip.
//!
//! A block covers only where the result can differ from nothing: the tiles the layer's earlier
//! ops reach, widened by the kernel's reach, cut to the canvas and to the clip's rectangle (and
//! a pixel more, for bilinear reads). Elsewhere the layer was transparent and stays so. A result
//! is keyed by what it depends on (`FilterKey`), kept while scenes show it, dropped with the
//! arena's other blocks when that is full, and evaluated again from the artwork when a scene
//! needs it; a pixel never depends on which regions are evaluated, so incremental equals cold.

mod kernels;

use std::ops::Range;
use std::sync::Arc;

use openpaint_core::{AdjustEdit, Adjustment, LayerId, RegionEdit, Sharpen, StrokeList};

use kernels::{Cells, FilterBuffers, FilterJob};
pub(crate) use kernels::{FilterPipeline, SHADER as FILTER_SHADER};

use crate::affine::Affine;
use crate::core::Core;
use crate::edits::mask_bounds;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::plan::Plan;
use crate::program::SceneOps;
use crate::protocol::{MASK_HEADER, TILE, encode};
use crate::scene::{Item, Scene};
use crate::smudge::{Arena, Keep, Key, Material};

/// The kernel reaches three standard deviations each way, at most this many pixels.
pub(crate) const MAX_REACH: u32 = 96;

/// What a filter's result depends on: the content before it on its layer's paint or mask (the
/// strokes before it and the edits up to it, itself last) and where its pixels lie on the
/// canvas (under transform records). The lists are kept, so the samples they share stay alive.
#[derive(Debug, Clone)]
pub(crate) struct FilterKey {
    pub(crate) layer: LayerId,
    pub(crate) on_mask: bool,
    strokes: StrokeList,
    edits: Vec<RegionEdit>,
    pub(crate) to_canvas: Affine,
}

impl FilterKey {
    /// The filter `edits.last()` of the content `strokes` and `edits`.
    pub(crate) fn new(
        (layer, on_mask): (LayerId, bool),
        (strokes, edits): (&StrokeList, &[RegionEdit]),
        to_canvas: Affine,
    ) -> Self {
        Self {
            layer,
            on_mask,
            strokes: strokes.clone(),
            edits: edits.to_vec(),
            to_canvas,
        }
    }

    /// The filter's record.
    pub(crate) fn edit(&self) -> Option<&AdjustEdit> {
        self.edits.last().and_then(RegionEdit::adjust)
    }

    /// The same filter in the same place: layer, mask and its index among the edits.
    pub(crate) fn place(&self) -> (LayerId, bool, usize) {
        (self.layer, self.on_mask, self.edits.len().saturating_sub(1))
    }

    /// The same filter with the values of `edit` (a preview's next value, S35).
    pub(crate) fn retuned(&self, edit: &AdjustEdit) -> Self {
        let mut key = self.clone();
        if let Some(last) = key.edits.last_mut() {
            *last = RegionEdit::Adjust(edit.clone());
        }
        key
    }

    /// Whether `other` names the same result: the same place over the same content.
    fn same(&self, other: &Self) -> bool {
        self.same_before(other)
            && self
                .edits
                .last()
                .zip(other.edits.last())
                .is_none_or(|(a, b)| a.same(b))
    }

    /// Whether `other` filters the same pixels: the same place over the same content before
    /// it, whatever its own values.
    fn same_before(&self, other: &Self) -> bool {
        let after = self.edits.last().map_or(0, RegionEdit::after);
        let strokes = after == 0
            || matches!(
                (self.strokes.get(after - 1), other.strokes.get(after - 1)),
                (Some(a), Some(b)) if a.shares_samples(b)
            );
        let before = self.edits.len().saturating_sub(1);
        self.place() == other.place()
            && self.to_canvas == other.to_canvas
            && strokes
            && self.edits[..before]
                .iter()
                .zip(&other.edits)
                .all(|(a, b)| a.same(b))
    }
}

/// A filter the scene draws, in scene order: its result must be in the target's arena before
/// the scene is evaluated. A filter a selection's record shows again, in the content it moved,
/// takes its first place's key and items.
#[derive(Debug, Clone)]
pub(crate) struct FilterItem {
    pub(crate) key: Arc<FilterKey>,
    /// The items of its layer before it: the pixels it filters.
    pub(crate) before: Range<usize>,
    /// Its item and its clip as its op names it.
    pub(crate) item: usize,
    pub(crate) clip: u32,
}

/// The standard deviation in canvas pixels and the sharpening strength (0: a blur) of
/// `adjustment`, when it reads neighbouring pixels.
pub(crate) fn kernel_of(adjustment: &Adjustment) -> Option<(f32, f32)> {
    match adjustment {
        Adjustment::Blur(blur) => Some((blur.radius, 0.0)),
        Adjustment::Sharpen(sharpen) => Some((Sharpen::RADIUS, sharpen.amount * Sharpen::STRENGTH)),
        Adjustment::Hsb(_) | Adjustment::Curves(_) => None,
    }
}

/// How far a kernel of standard deviation `sigma` reaches each way, in pixels.
pub(crate) fn reach(sigma: f32) -> u32 {
    if sigma > 0.0 {
        ((3.0 * sigma).ceil() as u32).min(MAX_REACH)
    } else {
        0
    }
}

/// How far `adjustment` changes pixels beyond the content it filters, in whole tiles (0 for a
/// point adjustment).
pub(crate) fn spread(adjustment: &Adjustment) -> u32 {
    kernel_of(adjustment).map_or(0, |(sigma, _)| extent(sigma).div_ceil(TILE))
}

/// How far a filter of deviation `sigma` reaches each way, in canvas pixels: its kernel's
/// reach, and past a reduced kernel's (S35) its cells' and their enlarging's pixels.
pub(crate) fn extent(sigma: f32) -> u32 {
    let factor = kernels::factor(sigma);
    reach(sigma) + if factor > 1 { 2 * factor } else { 0 }
}

/// The longest `extent`: a radius of `Blur::MAX_RADIUS`.
pub(crate) const MAX_EXTENT: u32 = MAX_REACH + 8;

/// The Gaussian's weights from `-reach` to `reach`, summing to 1.
pub(crate) fn weights(sigma: f32) -> Vec<f32> {
    let reach = reach(sigma);
    if reach == 0 {
        return vec![1.0];
    }
    let sigma = f64::from(sigma);
    let raw: Vec<f64> = (-i64::from(reach)..=i64::from(reach))
        .map(|k| (-((k * k) as f64) / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f64 = raw.iter().sum();
    raw.iter().map(|weight| (weight / total) as f32).collect()
}

/// A canvas target's filter results: which key names which arena block.
#[derive(Default)]
pub(crate) struct Filters {
    known: Vec<(u64, Arc<FilterKey>)>,
    next: u64,
    buffers: Option<FilterBuffers>,
    /// The filter whose layer's pixels before it the material holds, and over which pixels.
    held: Option<(Arc<FilterKey>, PixelRect)>,
}

impl Filters {
    /// The arena key of each of `items`, numbering the results not known yet.
    pub(crate) fn keys(&mut self, items: &[FilterItem]) -> Vec<Key> {
        items
            .iter()
            .map(|item| {
                let known = self.known.iter().find(|(_, key)| key.same(&item.key));
                let id = match known {
                    Some((id, _)) => *id,
                    None => {
                        self.next += 1;
                        self.known.push((self.next, Arc::clone(&item.key)));
                        self.next
                    }
                };
                Key::Filter(id)
            })
            .collect()
    }

    /// Bytes of its kernels' buffers (its results lie in the arena).
    pub(crate) fn bytes(&self) -> u64 {
        self.buffers.as_ref().map_or(0, FilterBuffers::bytes)
    }

    /// Forgets the results the arena no longer holds.
    pub(crate) fn retain(&mut self, arena: &Arena) {
        self.known
            .retain(|(id, _)| arena.base(&Key::Filter(*id)).is_some());
    }

    /// Makes `item`'s result present in `arena` as the block of `key`: the layer's pixels before
    /// it into `material` (the scene's other blocks and `empty`, an empty residual, bound;
    /// `held` while nothing but filters wrote the material since the last call), then filtered
    /// into a new block.
    ///
    /// # Errors
    /// `Admission` when the arena would exceed `limit` bytes; `Invalid` for an item that is no
    /// filter.
    pub(crate) fn prepare(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        (item, key): (&FilterItem, Key),
        (arena, material, empty, held): (&mut Arena, &mut Material, &wgpu::Buffer, bool),
        keep: &Keep<'_>,
    ) -> Result<(), RenderError> {
        if arena.base(&key).is_some() {
            return Ok(());
        }
        let edit = item.key.edit();
        let (sigma, strength) = edit
            .and_then(|edit| kernel_of(edit.adjustment()))
            .ok_or(RenderError::Invalid("a filter item holds no filter"))?;
        let clip = edit
            .and_then(AdjustEdit::clip)
            .map(|clip| mask_bounds(clip).pixels());
        let block = block_rect(scene, item, extent(sigma), clip);
        let size = [block.max[0] - block.min[0], block.max[1] - block.min[1]].map(|v| v as u32);
        let header = [block.min[0] as u32, block.min[1] as u32, size[0], size[1]];
        let words = u64::from(MASK_HEADER) + u64::from(size[0]) * u64::from(size[1]);
        if size.contains(&0) {
            let base = arena.reserve(&core.gpu, (key, None), words, keep)?;
            core.gpu
                .queue
                .write_buffer(arena.buffer(), base * 4, &encode(&[0u32; 4]));
            return Ok(());
        }
        let cells = Cells::of(block, scene.canvas, sigma);
        let tools = (&mut *material, [arena.buffer(), empty]);
        self.hold(core, (scene, ops), (item, clip, cells.read), tools, held)?;
        let base = arena.reserve(&core.gpu, (key, None), words, keep)?;
        core.gpu
            .queue
            .write_buffer(arena.buffer(), base * 4, &encode(&header));
        let job = FilterJob {
            block: header,
            cells,
            canvas: scene.canvas,
            base: u32::try_from(base + u64::from(MASK_HEADER))
                .map_err(|_| RenderError::Invalid("the arena exceeds 2^32 words"))?,
            weights: weights(cells.sigma),
            strength,
        };
        let buffers = self
            .buffers
            .get_or_insert_with(|| FilterBuffers::new(&core.gpu));
        core.filter
            .run(&core.gpu, buffers, &job, (material.view(), arena.buffer()));
        Ok(())
    }

    /// Makes `material` hold the layer's pixels before `item` over `read`: evaluated now, unless
    /// it still holds them (`held`: nothing but filters wrote it since) from the same content
    /// before a filter in the same place, a preview's earlier value (S35). Evaluated for such a
    /// preview, it holds what any radius reads.
    fn hold(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        (item, clip, read): (&FilterItem, Option<PixelRect>, PixelRect),
        (material, residuals): (&mut Material, [&wgpu::Buffer; 2]),
        held: bool,
    ) -> Result<(), RenderError> {
        let again = held
            && self
                .held
                .as_ref()
                .is_some_and(|(key, _)| key.same_before(&item.key));
        let kept = self.held.as_ref().filter(|_| again).map(|(_, rect)| *rect);
        if kept.is_some_and(|rect| rect.intersect(&read) == read) {
            return Ok(());
        }
        let rect = if again {
            let canvas = PixelRect::full(scene.canvas);
            widened(block_rect(scene, item, MAX_EXTENT, clip), MAX_EXTENT).intersect(&canvas)
        } else {
            read
        };
        let rect = if rect.intersect(&read) == read {
            rect
        } else {
            read
        };
        let range = (item.before.clone(), item.key.to_canvas);
        material.evaluate(core, (scene, ops), range, &[rect], residuals)?;
        self.held = Some((Arc::clone(&item.key), rect));
        Ok(())
    }
}

/// Where `item`'s result can differ from nothing: where the layer's ops before it reach,
/// widened by `reach`, cut to the canvas and to `clip` (widened by a pixel). Empty when nothing
/// lies there.
fn block_rect(scene: &Scene, item: &FilterItem, reach: u32, clip: Option<PixelRect>) -> PixelRect {
    let canvas = PixelRect::full(scene.canvas);
    let Some(content) = content_rect(scene, item) else {
        return NOTHING;
    };
    let mut block = widened(content, reach).intersect(&canvas);
    if let Some(clip) = clip {
        block = block.intersect(&widened(clip, 1));
    }
    if block.is_empty() { NOTHING } else { block }
}

/// Where the layer's ops before `item` can have painted, in its pixels: the union of their
/// bounds; under transform records (frames, moved selections) the tiles the material's plan
/// lists anything in, as the binner maps bounds there. `None` when nothing was painted.
fn content_rect(scene: &Scene, item: &FilterItem) -> Option<PixelRect> {
    let items = &scene.items[item.before.clone()];
    let moved = |each: &Item| matches!(each, Item::Frame { .. } | Item::BeginMove(_));
    if item.key.to_canvas.is_identity() && !items.iter().any(moved) {
        return items
            .iter()
            .filter_map(|each| match each {
                Item::Segment { bounds, .. }
                | Item::Fill { bounds, .. }
                | Item::Clear { bounds, .. }
                | Item::Reach(bounds)
                    if !bounds.is_empty() =>
                {
                    Some(bounds.pixels())
                }
                _ => None,
            })
            .reduce(|a, b| a.union(&b));
    }
    let canvas = PixelRect::full(scene.canvas);
    let range = (item.before.clone(), item.key.to_canvas);
    let plan = Plan::material(scene, range, scene.canvas, &[canvas]);
    // Every tile lists the layer's end; one that lists nothing else stays transparent.
    plan.tiles
        .iter()
        .filter(|tile| tile.count > 1)
        .map(|tile| {
            let [x, y] = tile.origin.map(|v| v as i32);
            let side = TILE as i32;
            PixelRect {
                min: [x, y],
                max: [x + side, y + side],
            }
        })
        .reduce(|a, b| a.union(&b))
}

/// No pixels.
const NOTHING: PixelRect = PixelRect {
    min: [0, 0],
    max: [0, 0],
};

/// `rect` widened by `by` pixels each way.
fn widened(rect: PixelRect, by: u32) -> PixelRect {
    let by = by as i32;
    PixelRect {
        min: [rect.min[0] - by, rect.min[1] - by],
        max: [rect.max[0] + by, rect.max[1] + by],
    }
}

#[cfg(test)]
mod tests;
