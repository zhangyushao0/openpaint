//! The sample artwork as schema 11 holds it.

use openpaint_core::{
    ArtworkParts, EditParts, FillParts, LayerMaskParts, MaskParts, StrokeBlend, StrokeParts,
};

use super::schema_10_parts;

/// The schema-10 sample with what schema 11 adds: the sketch layer's mask inverted, with a grey
/// stroke inside a selection, a smudge and a grey fill between them; the ink layer's mask
/// white with one black stroke, and the target (the ink layer is active); the clipping top
/// layer's mask disabled and empty.
pub fn schema_11_parts() -> ArtworkParts {
    let mut parts = schema_10_parts();
    let model = parts.layers[0].strokes[1].clone();
    let grey = |id, color: f32, blend| StrokeParts {
        id,
        blend,
        color: [color; 3],
        ..model.clone()
    };
    let mut clipped = grey(8, 0.25, StrokeBlend::Over);
    clipped.clip = Some(MaskParts {
        origin: [1, 2],
        size: [9, 3],
        runs: vec![2, 20, 5],
    });
    parts.layers[0].mask = Some(LayerMaskParts {
        enabled: true,
        inverted: true,
        strokes: vec![clipped, grey(9, 1.0, StrokeBlend::Smudge)],
        edits: vec![EditParts::Fill(FillParts {
            after: 1,
            color: [0.5; 3],
            alpha_locked: false,
            origin: [20, 10],
            size: [4, 4],
            runs: vec![0, 16],
        })],
    });
    parts.layers[1].mask = Some(LayerMaskParts {
        enabled: true,
        inverted: false,
        strokes: vec![grey(10, 0.0, StrokeBlend::Over)],
        edits: Vec::new(),
    });
    parts.layers[2].mask = Some(LayerMaskParts {
        enabled: false,
        inverted: false,
        strokes: Vec::new(),
        edits: Vec::new(),
    });
    parts.on_mask = true;
    parts.next_stroke_id = 11;
    parts
}
