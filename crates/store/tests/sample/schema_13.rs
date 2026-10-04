//! The sample artwork as schema 13 holds it.

use openpaint_core::{ArtworkParts, EditParts, MaskParts, TransformParts};

use super::schema_12_parts;

/// The schema-12 sample with what schema 13 adds, transform records without pixels: on the ink
/// layer's paint, after its dot and eraser stroke, a selection scaled up three times and
/// turned, then the whole layer moved and mirrored.
pub fn parts() -> ArtworkParts {
    let mut parts = schema_12_parts();
    let ink = &mut parts.layers[1].edits;
    ink.push(EditParts::Transform(TransformParts {
        after: 2,
        lift: Some(MaskParts {
            origin: [2, 3],
            size: [5, 2],
            runs: vec![1, 8, 1],
        }),
        patch: None,
        matrix: [0.0, -3.0, 3.0, 0.0],
        translation: [12.5, 4.0],
    }));
    ink.push(EditParts::Transform(TransformParts {
        after: 2,
        lift: None,
        patch: None,
        matrix: [-1.0, 0.0, 0.0, 1.0],
        translation: [60.0, -2.5],
    }));
    parts
}
