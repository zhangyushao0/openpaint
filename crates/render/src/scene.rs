//! `Scene`: an artwork flattened into ops in drawing order, with the bounds each segment can
//! reach, and `Fresh`: the in-progress stroke's part of one evaluation. The geometry of one
//! stroke is in `stroke.rs`.

use openpaint_core::{
    ActiveStroke, Artwork, Layer, LayerId, RegionEdit, StrokeBlend, StrokeSample, StrokeStyle,
};

use crate::error::RenderError;
use crate::geom::Bounds;
use crate::protocol::{Blend, MASK_HEADER, Op, Paint};
use crate::stroke::{paint, segments};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Segment {
        op: Op,
        bounds: Bounds,
    },
    EndStroke(Op),
    /// A fill, painted over its layer inside its mask's rectangle.
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
/// masks (`MASK_HEADER` words each, then their rows), which the fill ops point into.
#[derive(Debug, Default)]
pub(crate) struct Scene {
    pub(crate) items: Vec<Item>,
    pub(crate) masks: Vec<u32>,
}

impl Scene {
    /// The finished content of `artwork`, with the place of an in-progress stroke on `active`.
    /// Hidden layers, and layers with neither content nor that place, contribute no ops.
    pub(crate) fn build(artwork: &Artwork, active: Option<LayerId>) -> Result<Self, RenderError> {
        if let Some(layer) = active
            && artwork.layer(layer).is_none()
        {
            return Err(RenderError::Invalid(
                "the active stroke's layer is not in the artwork",
            ));
        }
        let mut scene = Self::default();
        for layer in artwork.layers() {
            if layer.props().visible() {
                let place = active == Some(layer.id());
                scene.push_layer(layer, place, layer.props().opacity())?;
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

    fn push_layer(&mut self, layer: &Layer, active: bool, opacity: f32) -> Result<(), RenderError> {
        let start = self.items.len();
        let mut edits = layer.edits().iter().peekable();
        for (index, stroke) in layer.strokes().iter().enumerate() {
            while let Some(edit) = edits.next_if(|edit| edit.after() == index) {
                self.push_fill(edit)?;
            }
            self.push_stroke(stroke.style(), stroke.blend(), stroke.samples());
        }
        for edit in edits {
            self.push_fill(edit)?;
        }
        if active {
            self.items.push(Item::Active);
        }
        if self.items.len() > start {
            self.items.push(Item::EndLayer(Op::end_layer(opacity)));
        }
        Ok(())
    }

    fn push_fill(&mut self, edit: &RegionEdit) -> Result<(), RenderError> {
        let mask = edit.mask();
        let at = u32::try_from(self.masks.len())
            .map_err(|_| RenderError::Invalid("the fill masks exceed 2^32 words"))?;
        let ([x, y], [width, height]) = (mask.origin(), mask.size());
        self.masks.extend([x, y, width, height]);
        self.masks.extend_from_slice(mask.words());
        debug_assert_eq!(MASK_HEADER, 4);
        let color = edit.color();
        let paint = Paint {
            color: [color.r(), color.g(), color.b()],
            opacity: 1.0,
            grain: 0,
            grain_scale: 1,
            blend: if edit.alpha_locked() {
                Blend::Atop
            } else {
                Blend::Over
            },
        };
        // The mask's pixels, from their top-left corner to past their bottom-right one.
        let bounds = Bounds([
            x as f32,
            y as f32,
            (x + width) as f32 - 0.5,
            (y + height) as f32 - 0.5,
        ]);
        self.items.push(Item::Fill {
            op: Op::fill(&paint, at),
            bounds,
        });
        Ok(())
    }

    /// Whether the in-progress stroke has a place, that is, its layer is visible.
    pub(crate) fn has_active(&self) -> bool {
        self.items.contains(&Item::Active)
    }

    fn push_stroke(&mut self, style: &StrokeStyle, blend: StrokeBlend, samples: &[StrokeSample]) {
        for segment in segments(style, samples, 0) {
            self.items.push(Item::Segment {
                op: segment.op(),
                bounds: segment.bounds(),
            });
        }
        self.items
            .push(Item::EndStroke(Op::end_stroke(&paint(style, blend))));
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
    pub(crate) fn new(active: &ActiveStroke, from: usize) -> Self {
        let (style, samples) = (active.style(), active.samples());
        let mut fresh = Self {
            ops: vec![Op::end_active(&paint(style, active.blend()))],
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
        let scene = Scene::build(&one, Some(one.active())).expect("scene");
        assert_eq!(codes(&scene), ["segment", "stroke", "active", "layer"]);
        assert!(scene.has_active());
        let empty = Scene::build(&blank, Some(blank.active())).expect("scene");
        assert_eq!(codes(&empty), ["active", "layer"]);
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
        assert!(!build(&hidden, Some(layer)).has_active());
        let shown = codes(&build(&drawn, None));
        assert_eq!(shown, ["segment", "stroke", "layer"]);
    }

    #[test]
    fn fresh_ops_start_with_the_paint_op() {
        let samples = [sample(10.0, 20.0, 1.0, 0), sample(30.0, 20.0, 0.2, 4)];
        let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
        let mut session = blank.begin_stroke(style(), samples[0]).expect("begin");
        let dot = Fresh::new(session.active(), 0);
        assert_eq!(dot.bounds.len(), 1);
        session.extend(&samples[1..]).expect("extend");
        let fresh = Fresh::new(session.active(), 0);
        assert_eq!(fresh.ops.len(), 2);
        assert_eq!(fresh.bounds.len(), 1);
        assert_eq!(
            Some(fresh.bounds[0].pixels()),
            stroke_region(&style(), &samples, 0)
        );
        assert_eq!(Fresh::new(session.active(), 1).bounds.len(), 0);
    }
}
