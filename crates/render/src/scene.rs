//! `Scene`: an artwork flattened into ops in drawing order, with the bounds each segment can
//! reach, and `Fresh`: the in-progress stroke's part of one evaluation. The geometry of one
//! stroke is in `stroke.rs`.

use openpaint_core::{Artwork, LayerId, StrokeSample, StrokeStyle};

use crate::error::RenderError;
use crate::geom::Bounds;
use crate::protocol::Op;
use crate::stroke::{paint, segments};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Segment {
        op: Op,
        bounds: Bounds,
    },
    EndStroke(Op),
    EndLayer(Op),
    /// Where the in-progress stroke draws: above its layer's finished strokes. Its ops are the
    /// evaluation's `Fresh` ops, which change every frame while the scene stays.
    Active,
}

impl Item {
    /// For `Active`, an op no plan refers to, so item and op indices stay equal.
    pub(crate) fn op(&self) -> Op {
        match self {
            Self::Segment { op, .. } | Self::EndStroke(op) | Self::EndLayer(op) => *op,
            Self::Active => Op::unused(),
        }
    }
}

/// Bottom layer first; inside a layer, finished strokes oldest first, then the in-progress
/// stroke's place.
#[derive(Debug, Default)]
pub(crate) struct Scene {
    pub(crate) items: Vec<Item>,
}

impl Scene {
    /// The finished content of `artwork`, with the place of an in-progress stroke on `active`.
    /// Hidden layers, and layers with neither strokes nor that place, contribute no ops.
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
            if !layer.props().visible() {
                continue;
            }
            let start = scene.items.len();
            for stroke in layer.strokes().iter() {
                scene.push_stroke(stroke.style(), stroke.samples());
            }
            if active == Some(layer.id()) {
                scene.items.push(Item::Active);
            }
            if scene.items.len() > start {
                let op = Op::end_layer(layer.props().opacity());
                scene.items.push(Item::EndLayer(op));
            }
        }
        Ok(scene)
    }

    /// Whether the in-progress stroke has a place, that is, its layer is visible.
    pub(crate) fn has_active(&self) -> bool {
        self.items.contains(&Item::Active)
    }

    fn push_stroke(&mut self, style: &StrokeStyle, samples: &[StrokeSample]) {
        for segment in segments(style, samples, 0) {
            self.items.push(Item::Segment {
                op: segment.op(),
                bounds: segment.bounds(),
            });
        }
        self.items
            .push(Item::EndStroke(Op::end_stroke(&paint(style))));
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
    pub(crate) fn new(style: &StrokeStyle, samples: &[StrokeSample], from: usize) -> Self {
        let mut fresh = Self {
            ops: vec![Op::end_active(&paint(style))],
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
        let fresh = Fresh::new(&style(), &samples, 0);
        assert_eq!(fresh.ops.len(), 2);
        assert_eq!(fresh.bounds.len(), 1);
        assert_eq!(
            Some(fresh.bounds[0].pixels()),
            stroke_region(&style(), &samples, 0)
        );
        let dot = Fresh::new(&style(), &samples[..1], 0);
        assert_eq!(dot.bounds.len(), 1);
        assert_eq!(Fresh::new(&style(), &samples, 1).bounds.len(), 0);
    }
}
