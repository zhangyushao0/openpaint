//! The sample artwork as schema 12 holds it.

use openpaint_core::{AdjustParts, AdjustmentParts, ArtworkParts, EditParts, MaskParts};

use super::schema_11_parts;

/// The schema-11 sample with what schema 12 adds, adjustments: on the sketch layer's paint,
/// after both its strokes, hue, saturation and brightness inside a selection; on its inverted
/// mask, after both its strokes, curves; on the ink layer's mask, after its black stroke, hue,
/// saturation and brightness over the whole mask.
pub fn parts() -> ArtworkParts {
    let mut parts = schema_11_parts();
    let adjust = |after, clip, inverted, adjustment| {
        EditParts::Adjust(AdjustParts {
            after,
            clip,
            inverted,
            adjustment,
        })
    };
    let selected = MaskParts {
        origin: [3, 4],
        size: [3, 2],
        runs: vec![0, 5, 1],
    };
    let curves = AdjustmentParts::Curves([
        vec![[0.0, 0.0], [0.25, 0.4], [1.0, 1.0]],
        vec![[0.0, 0.1], [1.0, 0.9]],
        vec![[0.0, 0.0], [1.0, 1.0]],
        vec![[0.0, 0.0], [0.5, 0.25], [0.75, 0.8], [1.0, 1.0]],
    ]);
    parts.layers[0].edits.push(adjust(
        2,
        Some(selected),
        false,
        AdjustmentParts::Hsb([0.625, 0.25, 0.75]),
    ));
    let sketch = parts.layers[0].mask.as_mut().expect("the sketch's mask");
    sketch.edits.push(adjust(2, None, true, curves));
    let ink = parts.layers[1].mask.as_mut().expect("the ink's mask");
    ink.edits.push(adjust(
        1,
        None,
        false,
        AdjustmentParts::Hsb([0.5, 0.5, 0.375]),
    ));
    parts
}
