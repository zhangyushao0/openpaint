//! Plain, unvalidated artwork data for the store. `Artwork::to_parts` produces it;
//! `Artwork::from_storage` is the only way back and validates everything.

use crate::artwork::Artwork;
use crate::layer::Layer;
use crate::stroke::{Stroke, StrokeSample};

/// A whole artwork as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtworkParts {
    pub width: u32,
    pub height: u32,
    /// Bottom to top.
    pub layers: Vec<LayerParts>,
    pub active: u64,
    pub next_layer_id: u64,
    pub next_stroke_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerParts {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub locked: bool,
    /// Oldest first.
    pub strokes: Vec<StrokeParts>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrokeParts {
    pub id: u64,
    pub size: f32,
    pub opacity: f32,
    /// sRGB red, green, blue.
    pub color: [f32; 3],
    pub samples: Vec<SampleParts>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleParts {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt: f32,
    pub azimuth: f32,
    pub time_ms: u32,
}

impl Artwork {
    /// The artwork as plain data for storage.
    #[must_use]
    pub fn to_parts(&self) -> ArtworkParts {
        let state = self.state();
        ArtworkParts {
            width: state.canvas.width(),
            height: state.canvas.height(),
            layers: state.layers.iter().map(layer_parts).collect(),
            active: state.active.get(),
            next_layer_id: state.ids.next_layer,
            next_stroke_id: state.ids.next_stroke,
        }
    }
}

fn layer_parts(layer: &Layer) -> LayerParts {
    LayerParts {
        id: layer.id.get(),
        name: layer.name.as_str().to_owned(),
        visible: layer.props.visible,
        opacity: layer.props.opacity,
        locked: layer.props.locked,
        strokes: layer.strokes.iter().map(stroke_parts).collect(),
    }
}

fn stroke_parts(stroke: &Stroke) -> StrokeParts {
    let style = stroke.style();
    let color = style.color();
    StrokeParts {
        id: stroke.id().get(),
        size: style.size(),
        opacity: style.opacity(),
        color: [color.r(), color.g(), color.b()],
        samples: stroke.samples().iter().map(sample_parts).collect(),
    }
}

fn sample_parts(sample: &StrokeSample) -> SampleParts {
    let position = sample.position();
    SampleParts {
        x: position.x(),
        y: position.y(),
        pressure: sample.pressure(),
        tilt: sample.tilt(),
        azimuth: sample.azimuth(),
        time_ms: sample.time_ms(),
    }
}
