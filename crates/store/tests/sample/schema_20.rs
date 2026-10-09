//! The sample artwork as schema 20 holds it.

use openpaint_core::{ArtworkParts, Rotation, ShapeParts, ShapeTexture, Stamp, StrokeBlend};

use super::schema_19_parts;

/// The schema-19 sample with what schema 20 adds, shapes in recipes and seeds in strokes: at
/// the end of the ink layer's paint a stroke of rough squares half as long as wide, an eighth of
/// its size apart, scattered, turned to the stroke's direction less 30 degrees, jittered in
/// turn and size by its own seed.
pub fn parts() -> ArtworkParts {
    let mut parts = schema_19_parts();
    let mut stamped = parts.layers[0].strokes[0].clone();
    stamped.id = parts.next_stroke_id;
    (stamped.blend, stamped.clip) = (StrokeBlend::Over, None);
    stamped.recipe.shape = ShapeParts {
        stamp: Stamp::Texture(ShapeTexture::RoughSquare),
        aspect: 0.5,
        spacing: 0.125,
        scatter: 0.75,
        rotation: Rotation::Direction,
        angle: -30.0,
        rotation_jitter: 15.0,
        size_jitter: 0.25,
    };
    stamped.seed = 0xC0FF_EE01;
    parts.layers[1].strokes.push(stamped);
    parts.next_stroke_id += 1;
    parts
}
