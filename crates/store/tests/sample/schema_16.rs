//! The sample artwork as schema 16 holds it, and schema 17, which changed only the toolbox.

use openpaint_core::{ArtworkParts, ClearParts, EditParts, MaskParts};

use super::schema_15_parts;

/// The schema-15 sample with what schema 16 adds, cleared pixels: a cut on the sketch layer's
/// paint after its strokes, and inside the nested record the pixels outside a selection (as a
/// pasted copy holds them).
pub fn parts() -> ArtworkParts {
    let mut parts = schema_15_parts();
    let sketch = &mut parts.layers[0];
    let after = sketch.strokes.len() as u64;
    if let Some(EditParts::Nested(nested)) = sketch.edits.last_mut() {
        nested.edits.push(EditParts::Clear(ClearParts {
            after: nested.strokes.len() as u64,
            mask: MaskParts {
                origin: [0, 0],
                size: [64, 3],
                runs: vec![0, 70, 4, 118],
            },
        }));
    }
    sketch.edits.push(EditParts::Clear(ClearParts {
        after,
        mask: MaskParts {
            origin: [10, 12],
            size: [6, 2],
            runs: vec![1, 4, 2, 5],
        },
    }));
    parts
}
