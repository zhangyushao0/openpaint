//! The sample artwork as schema 19 holds it.

use openpaint_core::{ArtworkParts, StrokeBlend};

use super::schema_16_parts;

/// The schema-16 sample (schemas 17 and 18 changed only the toolbox) with what schema 19 adds,
/// tilt and taper in recipes: at the end of the ink layer's paint a stroke whose size widens and
/// whose opacity fades with tilt, tapered at both ends.
pub fn parts() -> ArtworkParts {
    let mut parts = schema_16_parts();
    let mut tilted = parts.layers[0].strokes[0].clone();
    tilted.id = parts.next_stroke_id;
    (tilted.blend, tilted.clip) = (StrokeBlend::Over, None);
    tilted.recipe.size_tilt = vec![[0.0, 1.0], [40.0, 1.0], [80.0, 2.5]];
    tilted.recipe.opacity_tilt = vec![[0.0, 1.0], [40.0, 1.0], [80.0, 0.375]];
    tilted.recipe.taper = [2.0, 3.5, 0.125];
    parts.layers[1].strokes.push(tilted);
    parts.next_stroke_id += 1;
    parts
}
