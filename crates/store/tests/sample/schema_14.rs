//! The sample artwork as schema 14 holds it.

use openpaint_core::{AdjustParts, AdjustmentParts, ArtworkParts, EditParts, MaskParts};

use super::schema_13_parts;

/// The schema-13 sample with what schema 14 adds, a Gaussian blur and sharpening: on the
/// sketch layer's paint after its strokes, a blur inside a selection, then sharpening of the
/// whole layer.
pub fn parts() -> ArtworkParts {
    let mut parts = schema_13_parts();
    let sketch = &mut parts.layers[0];
    let after = sketch.strokes.len() as u64;
    sketch.edits.push(EditParts::Adjust(AdjustParts {
        after,
        clip: Some(MaskParts {
            origin: [4, 2],
            size: [6, 3],
            runs: vec![2, 4, 1, 6, 2, 3],
        }),
        inverted: false,
        adjustment: AdjustmentParts::Blur(6.5),
    }));
    sketch.edits.push(EditParts::Adjust(AdjustParts {
        after,
        clip: None,
        inverted: false,
        adjustment: AdjustmentParts::Sharpen(0.75),
    }));
    parts
}
