//! `Scene`: an artwork flattened into ops in drawing order, with the bounds each segment can
//! reach, and `Fresh`: the in-progress stroke's part of one evaluation. The geometry of one
//! stroke is in `stroke.rs`.

use std::ops::Range;

use openpaint_core::{
    ActiveStroke, Artwork, Layer, LayerId, Patch, RegionMask, Stroke, StrokeBlend, StrokeSample,
    StrokeStyle, StrokeTool,
};

use crate::edits::restart;
use crate::error::RenderError;
use crate::geom::Bounds;
use crate::protocol::{MASK_HEADER, Op, Paint};
use crate::smudge::smudge_reach;
use crate::stroke::{paint, segments};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Segment {
        op: Op,
        bounds: Bounds,
    },
    EndStroke(Op),
    /// A region edit's op over its layer within `bounds`: a fill, a transform's lift (an
    /// erasing fill) or its patch.
    Fill {
        op: Op,
        bounds: Bounds,
    },
    EndLayer(Op),
    /// Where the in-progress stroke draws: above its layer's finished strokes. Its ops are the
    /// evaluation's `Fresh` ops, which change every frame while the scene stays.
    Active,
    /// Pixels a smudge can change, one box per pair of samples; listed nowhere itself.
    Reach(Bounds),
    /// A smudge's residual applied to its layer, in every tile its reach touched.
    Smudge(Op),
}

impl Item {
    /// For `Active` and `Reach`, an op no plan refers to, so item and op indices stay equal.
    pub(crate) fn op(&self) -> Op {
        match self {
            Self::Segment { op, .. }
            | Self::Fill { op, .. }
            | Self::EndStroke(op)
            | Self::EndLayer(op)
            | Self::Smudge(op) => *op,
            Self::Active | Self::Reach(_) => Op::unused(),
        }
    }
}

/// A finished smudge the scene draws, in scene order: its residual must be in the target's
/// arena before the scene is evaluated.
#[derive(Debug, Clone)]
pub(crate) struct SmudgeItem {
    pub(crate) layer: LayerId,
    pub(crate) stroke: Stroke,
    /// The items of its layer before it: the pixels it carries.
    pub(crate) before: Range<usize>,
    /// It is its layer's newest stroke.
    pub(crate) newest: bool,
}

/// Bottom layer first; inside a layer, finished strokes oldest first with each fill after the
/// strokes it was made over, then the in-progress stroke's place. `masks` holds the fills'
/// masks and the clips of strokes drawn inside a selection (`MASK_HEADER` words each, then
/// their rows), which the fill and paint ops point into; strokes in a row sharing one clip
/// share its words.
#[derive(Debug, Default)]
pub(crate) struct Scene {
    pub(crate) items: Vec<Item>,
    pub(crate) masks: Vec<u32>,
    /// The in-progress stroke's clip as its paint op names it (0: none).
    pub(crate) active_clip: u32,
    /// The clip pushed last and its paint op value.
    pub(crate) last_clip: Option<(RegionMask, u32)>,
    /// The transforms' patches in the patch words, in order, and how many words they take.
    pub(crate) patches: Vec<Patch>,
    pub(crate) patch_words: usize,
    /// The finished smudges, and where in the mask words their residuals' bases are written
    /// (one word each, in order, filled in after the scene is uploaded).
    pub(crate) smudges: Vec<SmudgeItem>,
    pub(crate) bases_at: u32,
    /// The items of the in-progress stroke's layer before its place.
    pub(crate) active_before: Option<Range<usize>>,
}

/// Where the in-progress stroke draws: its layer, and the selection it is drawn inside.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivePlace {
    pub(crate) layer: LayerId,
    pub(crate) clip: Option<RegionMask>,
}

impl ActivePlace {
    pub(crate) fn of(active: &ActiveStroke) -> Self {
        Self {
            layer: active.layer(),
            clip: active.clip().cloned(),
        }
    }
}

impl Scene {
    /// The finished content of `artwork`, with the place of an in-progress stroke `active`.
    /// Hidden layers, and layers with neither content nor that place, contribute no ops.
    pub(crate) fn build(
        artwork: &Artwork,
        active: Option<&ActivePlace>,
    ) -> Result<Self, RenderError> {
        if let Some(place) = active
            && artwork.layer(place.layer).is_none()
        {
            return Err(RenderError::Invalid(
                "the active stroke's layer is not in the artwork",
            ));
        }
        let mut scene = Self::default();
        let shown = artwork
            .layers()
            .iter()
            .filter(|layer| layer.props().visible());
        scene.reserve_bases(shown.map(smudge_count).sum());
        for layer in artwork.layers() {
            if layer.props().visible() {
                let place = active.filter(|place| place.layer == layer.id());
                if let Some(clip) = place.and_then(|place| place.clip.as_ref()) {
                    scene.active_clip = scene.clip(clip)?;
                }
                scene.push_layer(layer, place.is_some(), layer.props().opacity())?;
            }
        }
        Ok(scene)
    }

    /// `layer` of `artwork` alone, at full opacity, whether it is shown or not: what a fill
    /// samples.
    pub(crate) fn layer(artwork: &Artwork, layer: LayerId) -> Result<Self, RenderError> {
        let layer = artwork
            .layer(layer)
            .ok_or(RenderError::Invalid("the layer is not in the artwork"))?;
        let mut scene = Self::default();
        scene.reserve_bases(smudge_count(layer));
        scene.push_layer(layer, false, 1.0)?;
        Ok(scene)
    }

    /// The first mask words, one per smudge: where their residuals start in the arena.
    fn reserve_bases(&mut self, count: usize) {
        self.bases_at = 0;
        self.masks.resize(count, 0);
    }

    /// A transform of the whole layer clears everything before it, so the layer's ops start at
    /// the last one (`restart`): the same pixels, and earlier patches stay off the GPU.
    fn push_layer(&mut self, layer: &Layer, active: bool, opacity: f32) -> Result<(), RenderError> {
        let start = self.items.len();
        let (first, after) = restart(layer);
        let mut edits = layer.edits()[first..].iter().peekable();
        let newest = layer.strokes().len().checked_sub(1);
        for (index, stroke) in layer.strokes().iter().enumerate().skip(after) {
            while let Some(edit) = edits.next_if(|edit| edit.after() == index) {
                self.push_edit(edit)?;
            }
            let clip = stroke.clip().map(|clip| self.clip(clip)).transpose()?;
            if stroke.style().tool() == StrokeTool::Smudge {
                let item = SmudgeItem {
                    layer: layer.id(),
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
            self.push_edit(edit)?;
        }
        if active {
            self.active_before = Some(start..self.items.len());
            self.items.push(Item::Active);
        }
        if self.items.len() > start {
            self.items.push(Item::EndLayer(Op::end_layer(opacity)));
        }
        Ok(())
    }

    /// Appends `mask` to the mask words; returns where it starts.
    pub(crate) fn push_mask(&mut self, mask: &RegionMask) -> Result<u32, RenderError> {
        // Below 2^32 - 1, so a clip's value (one more) fits too.
        let at = u32::try_from(self.masks.len())
            .ok()
            .filter(|at| *at < u32::MAX)
            .ok_or(RenderError::Invalid("the masks exceed 2^32 words"))?;
        let ([x, y], [width, height]) = (mask.origin(), mask.size());
        self.masks.extend([x, y, width, height]);
        self.masks.extend_from_slice(mask.words());
        debug_assert_eq!(MASK_HEADER, 4);
        Ok(at)
    }

    /// The paint op value of the clip `mask`: one more than where its words start, shared with
    /// the clip pushed last when that is the same mask.
    fn clip(&mut self, mask: &RegionMask) -> Result<u32, RenderError> {
        if let Some((last, value)) = &self.last_clip
            && last.shares(mask)
        {
            return Ok(*value);
        }
        let value = self.push_mask(mask)? + 1;
        self.last_clip = Some((mask.clone(), value));
        Ok(value)
    }

    /// Whether the in-progress stroke has a place, that is, its layer is visible.
    pub(crate) fn has_active(&self) -> bool {
        self.items.contains(&Item::Active)
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

/// How many smudges of `layer` a scene draws: those after a transform of the whole layer.
fn smudge_count(layer: &Layer) -> usize {
    let (_, after) = restart(layer);
    layer
        .strokes()
        .since(after)
        .filter(|stroke| stroke.style().tool() == StrokeTool::Smudge)
        .count()
}

/// The in-progress stroke's ops for one evaluation: `ops[0]` paints it (`EndActive`), the rest
/// are the segments not yet in its coverage texture, with their bounds in `bounds`.
#[derive(Debug, Default)]
pub(crate) struct Fresh {
    pub(crate) ops: Vec<Op>,
    pub(crate) bounds: Vec<Bounds>,
}

impl Fresh {
    /// The segments of `samples` (every sample so far) after sample `from`: the new samples
    /// with the one before them, or the whole stroke (`from` 0) when its coverage starts over.
    /// `clip` is the stroke's clip as the scene placed it (`Scene::active_clip`).
    pub(crate) fn new(active: &ActiveStroke, from: usize, clip: u32) -> Self {
        let (style, samples) = (active.style(), active.samples());
        if style.tool() == StrokeTool::Smudge {
            // Its residual so far, applied in every tile (`Smudges::advance` extends it).
            let locked = active.blend() == StrokeBlend::SmudgeLocked;
            return Self {
                ops: vec![Op::smudge(None, clip, locked)],
                bounds: Vec::new(),
            };
        }
        let mut paint = paint(style, active.blend());
        paint.clip = clip;
        let mut fresh = Self {
            ops: vec![Op::end_active(&paint)],
            bounds: Vec::new(),
        };
        for segment in segments(style, samples, from) {
            fresh.ops.push(segment.op());
            fresh.bounds.push(segment.bounds());
        }
        fresh
    }
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
