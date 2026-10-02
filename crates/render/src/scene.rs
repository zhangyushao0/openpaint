//! `Scene`: a frame request flattened into ops in drawing order, with the bounds each segment
//! can reach. Stroke geometry lives here: a round continuous brush is a chain of tapered
//! capsules between consecutive samples (DECISIONS D07: no dabs).

use openpaint_core::{StrokeSample, StrokeStyle};

use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::{Bounds, PixelRect};
use crate::protocol::Op;

/// Antialiasing reaches half a pixel beyond the shape; one pixel keeps the bounds conservative
/// against float error.
const REACH: f32 = 1.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Segment { op: Op, bounds: Bounds },
    EndStroke(Op),
    EndLayer(Op),
}

impl Item {
    pub(crate) fn op(&self) -> &Op {
        match self {
            Self::Segment { op, .. } | Self::EndStroke(op) | Self::EndLayer(op) => op,
        }
    }
}

/// Bottom layer first; inside a layer, finished strokes oldest first, then the active stroke.
#[derive(Debug, Default)]
pub(crate) struct Scene {
    pub(crate) items: Vec<Item>,
}

impl Scene {
    /// Hidden layers and layers without strokes contribute no ops.
    pub(crate) fn build(request: &FrameRequest<'_>) -> Result<Self, RenderError> {
        if let Some(active) = request.active
            && request.artwork.layer(active.layer()).is_none()
        {
            return Err(RenderError::Invalid(
                "the active stroke's layer is not in the artwork",
            ));
        }
        let mut scene = Self::default();
        for layer in request.artwork.layers() {
            if !layer.props().visible() {
                continue;
            }
            let start = scene.items.len();
            for stroke in layer.strokes().iter() {
                scene.push_stroke(stroke.style(), stroke.samples());
            }
            if let Some(active) = request.active.filter(|a| a.layer() == layer.id()) {
                scene.push_stroke(active.style(), active.samples());
            }
            if scene.items.len() > start {
                let op = Op::end_layer(layer.props().opacity());
                scene.items.push(Item::EndLayer(op));
            }
        }
        Ok(scene)
    }

    fn push_stroke(&mut self, style: &StrokeStyle, samples: &[StrokeSample]) {
        for segment in segments(style, samples) {
            self.items.push(Item::Segment {
                op: segment.op(),
                bounds: segment.bounds(),
            });
        }
        let color = style.color();
        let op = Op::end_stroke([color.r(), color.g(), color.b()], style.opacity());
        self.items.push(Item::EndStroke(op));
    }
}

/// The pixels a stroke of `style` through `samples` can change. For an in-progress stroke, pass
/// the samples added since the last frame together with the one before them: the result is the
/// region to evaluate again.
#[must_use]
pub fn stroke_region(style: &StrokeStyle, samples: &[StrokeSample]) -> Option<PixelRect> {
    segments(style, samples)
        .map(|segment| segment.bounds().pixels())
        .reduce(|all, next| all.union(&next))
}

/// A tapered capsule between two samples, in target pixels.
struct Segment {
    a: [f32; 2],
    b: [f32; 2],
    radii: [f32; 2],
}

impl Segment {
    fn op(&self) -> Op {
        Op::segment(self.a, self.b, self.radii)
    }

    fn bounds(&self) -> Bounds {
        let reach = self.radii[0].max(self.radii[1]) + REACH;
        Bounds([
            self.a[0].min(self.b[0]) - reach,
            self.a[1].min(self.b[1]) - reach,
            self.a[0].max(self.b[0]) + reach,
            self.a[1].max(self.b[1]) + reach,
        ])
    }
}

/// One segment per consecutive pair; a single sample is a round dot.
fn segments<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
) -> impl Iterator<Item = Segment> + 'a {
    let pairs = samples.windows(2).map(|pair| (pair[0], pair[1]));
    let dot = match samples {
        [only] => Some((*only, *only)),
        _ => None,
    };
    pairs.chain(dot).map(|(a, b)| Segment {
        a: point(&a),
        b: point(&b),
        radii: [radius(style, &a), radius(style, &b)],
    })
}

fn point(sample: &StrokeSample) -> [f32; 2] {
    let position = sample.position();
    [position.x(), position.y()]
}

/// The round brush until recipes (S11): the diameter scales linearly with pressure.
fn radius(style: &StrokeStyle, sample: &StrokeSample) -> f32 {
    0.5 * style.size() * sample.pressure()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_core::{Artwork, ArtworkCommand, CanvasPoint, CanvasSize, Color, LayerCommand};

    fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
        let position = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
    }

    fn style() -> StrokeStyle {
        StrokeStyle::new(10.0, 1.0, Color::BLACK).expect("style")
    }

    fn codes(scene: &Scene) -> Vec<&'static str> {
        let name = |item: &Item| match item {
            Item::Segment { .. } => "segment",
            Item::EndStroke(_) => "stroke",
            Item::EndLayer(_) => "layer",
        };
        scene.items.iter().map(name).collect()
    }

    fn request(artwork: &Artwork) -> FrameRequest<'_> {
        FrameRequest {
            artwork,
            active: None,
            background: Color::WHITE,
        }
    }

    #[test]
    fn active_stroke_follows_its_layers_finished_strokes() {
        let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
        let mut session = blank
            .begin_stroke(style(), sample(1.0, 1.0, 1.0, 0))
            .expect("begin");
        session.extend(&[sample(9.0, 9.0, 1.0, 5)]).expect("extend");
        let one = session.finish().artwork;
        let session = one
            .begin_stroke(style(), sample(30.0, 30.0, 0.5, 0))
            .expect("begin");
        let scene = Scene::build(&FrameRequest {
            active: Some(session.active()),
            ..request(&one)
        })
        .expect("scene");
        assert_eq!(
            codes(&scene),
            ["segment", "stroke", "segment", "stroke", "layer"]
        );
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
        assert_eq!(
            codes(&Scene::build(&request(&blank)).expect("scene")),
            [""; 0]
        );
        assert_eq!(
            codes(&Scene::build(&request(&hidden)).expect("scene")),
            [""; 0]
        );
        let shown = codes(&Scene::build(&request(&drawn)).expect("scene"));
        assert_eq!(shown, ["segment", "stroke", "layer"]);
    }

    #[test]
    fn region_covers_the_widest_radius_plus_antialiasing() {
        let samples = [sample(10.0, 20.0, 1.0, 0), sample(30.0, 20.0, 0.2, 4)];
        let region = stroke_region(&style(), &samples).expect("region");
        assert_eq!(region.min, [4, 14]);
        assert_eq!(region.max, [37, 27]);
        assert_eq!(stroke_region(&style(), &[]), None);
    }
}
