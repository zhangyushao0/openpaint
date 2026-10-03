//! `Scene`: an artwork flattened into ops in drawing order, with the bounds each segment can
//! reach, and `Fresh`: the in-progress stroke's part of one evaluation. The geometry of one
//! stroke is in `stroke.rs`.

use openpaint_core::{
    ActiveStroke, Artwork, Layer, LayerId, Patch, RegionMask, StrokeSample, StrokeStyle,
};

use crate::edits::restart;
use crate::error::RenderError;
use crate::geom::Bounds;
use crate::protocol::{MASK_HEADER, Op, Paint};
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
}

impl Item {
    /// For `Active`, an op no plan refers to, so item and op indices stay equal.
    pub(crate) fn op(&self) -> Op {
        match self {
            Self::Segment { op, .. }
            | Self::Fill { op, .. }
            | Self::EndStroke(op)
            | Self::EndLayer(op) => *op,
            Self::Active => Op::unused(),
        }
    }
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
        scene.push_layer(layer, false, 1.0)?;
        Ok(scene)
    }

    /// A transform of the whole layer clears everything before it, so the layer's ops start at
    /// the last one (`restart`): the same pixels, and earlier patches stay off the GPU.
    fn push_layer(&mut self, layer: &Layer, active: bool, opacity: f32) -> Result<(), RenderError> {
        let start = self.items.len();
        let (first, after) = restart(layer);
        let mut edits = layer.edits()[first..].iter().peekable();
        for (index, stroke) in layer.strokes().iter().enumerate().skip(after) {
            while let Some(edit) = edits.next_if(|edit| edit.after() == index) {
                self.push_edit(edit)?;
            }
            let clip = stroke.clip().map(|clip| self.clip(clip)).transpose()?;
            let mut paint = paint(stroke.style(), stroke.blend());
            paint.clip = clip.unwrap_or(0);
            self.push_stroke(stroke.style(), &paint, stroke.samples());
        }
        for edit in edits {
            self.push_edit(edit)?;
        }
        if active {
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
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::stroke::stroke_region;
    use openpaint_core::{
        ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, LayerCommand,
    };

    fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
        let position = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
    }

    fn style() -> StrokeStyle {
        let recipe = Arc::new(BrushRecipe::plain());
        StrokeStyle::new(recipe, 10.0, 1.0, Color::BLACK).expect("style")
    }

    fn codes(scene: &Scene) -> Vec<&'static str> {
        let name = |item: &Item| match item {
            Item::Segment { .. } => "segment",
            Item::EndStroke(_) => "stroke",
            Item::Fill { .. } => "fill",
            Item::EndLayer(_) => "layer",
            Item::Active => "active",
        };
        scene.items.iter().map(name).collect()
    }

    #[test]
    fn active_place_follows_its_layers_finished_strokes() {
        let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
        let mut session = blank
            .begin_stroke(style(), sample(1.0, 1.0, 1.0, 0))
            .expect("begin");
        session.extend(&[sample(9.0, 9.0, 1.0, 5)]).expect("extend");
        let one = session.finish().artwork;
        let scene = Scene::build(&one, Some(&place(one.active(), None))).expect("scene");
        assert_eq!(codes(&scene), ["segment", "stroke", "active", "layer"]);
        assert!(scene.has_active());
        let empty = Scene::build(&blank, Some(&place(blank.active(), None))).expect("scene");
        assert_eq!(codes(&empty), ["active", "layer"]);
    }

    fn place(layer: LayerId, clip: Option<RegionMask>) -> ActivePlace {
        ActivePlace { layer, clip }
    }

    #[test]
    fn strokes_inside_one_selection_share_its_mask_with_the_stroke_being_drawn() {
        let canvas = CanvasSize::new(64, 64).expect("size");
        let clip = RegionMask::polygon(canvas, &[[4.0, 4.0], [40.0, 4.0], [40.0, 40.0]])
            .expect("a triangle");
        let mut artwork = Artwork::new(canvas);
        for x in [5.0, 9.0] {
            let session = artwork
                .begin_stroke(style(), sample(x, 10.0, 1.0, 0))
                .expect("begin");
            artwork = session
                .clipped(clip.clone())
                .expect("clip")
                .finish()
                .artwork;
        }
        let words = MASK_HEADER as usize + clip.words().len();
        let place = place(artwork.active(), Some(clip.clone()));
        let scene = Scene::build(&artwork, Some(&place)).expect("scene");
        assert_eq!(scene.masks.len(), words, "one copy");
        assert_eq!(scene.active_clip, 1);
        let alone = Scene::build(&artwork, None).expect("scene");
        assert_eq!((alone.masks.len(), alone.active_clip), (words, 0));
    }

    #[test]
    fn hidden_and_empty_layers_add_nothing() {
        let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
        let session = blank
            .begin_stroke(style(), sample(1.0, 1.0, 1.0, 0))
            .expect("begin");
        let drawn = session.finish().artwork;
        let layer = drawn.active();
        let hide = LayerCommand::SetVisible {
            layer,
            visible: false,
        };
        let hidden = drawn
            .apply(ArtworkCommand::Layer(hide))
            .expect("hide")
            .artwork;
        let build = |artwork: &Artwork, active| Scene::build(artwork, active).expect("scene");
        assert_eq!(codes(&build(&blank, None)), [""; 0]);
        assert_eq!(codes(&build(&hidden, None)), [""; 0]);
        let place = place(layer, None);
        let hidden_place = Scene::build(&hidden, Some(&place)).expect("scene");
        assert!(!hidden_place.has_active());
        let shown = codes(&build(&drawn, None));
        assert_eq!(shown, ["segment", "stroke", "layer"]);
    }

    #[test]
    fn fresh_ops_start_with_the_paint_op() {
        let samples = [sample(10.0, 20.0, 1.0, 0), sample(30.0, 20.0, 0.2, 4)];
        let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
        let mut session = blank.begin_stroke(style(), samples[0]).expect("begin");
        let dot = Fresh::new(session.active(), 0, 0);
        assert_eq!(dot.bounds.len(), 1);
        session.extend(&samples[1..]).expect("extend");
        let fresh = Fresh::new(session.active(), 0, 7);
        assert_eq!(fresh.ops.len(), 2);
        let clipped = Paint {
            clip: 7,
            ..paint(&style(), openpaint_core::StrokeBlend::Over)
        };
        assert_eq!(fresh.ops[0], Op::end_active(&clipped), "with the clip");
        assert_eq!(fresh.bounds.len(), 1);
        assert_eq!(
            Some(fresh.bounds[0].pixels()),
            stroke_region(&style(), &samples, 0)
        );
        assert_eq!(Fresh::new(session.active(), 1, 0).bounds.len(), 0);
    }
}
