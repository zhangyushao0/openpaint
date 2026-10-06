//! The sample artwork as schema 15 holds it.

use openpaint_core::{
    ArtworkParts, BlendMode, EditParts, FillParts, LayerMaskParts, NestedParts, StrokeBlend,
};

use super::schema_14_parts;

/// The schema-14 sample with what schema 15 adds, a layer merged down (a nested record): at the
/// end of the sketch layer's content, layer 6, clipped by Screen at 0.625, with one stroke, a
/// fill, a layer merged into it before (layer 7, one stroke at half opacity) and a mask with a
/// grey stroke.
pub fn parts() -> ArtworkParts {
    let mut parts = schema_14_parts();
    let model = parts.layers[0].strokes[1].clone();
    let stroke = |id, blend, color: [f32; 3]| {
        let mut stroke = model.clone();
        (stroke.id, stroke.blend, stroke.color, stroke.clip) = (id, blend, color, None);
        stroke
    };
    let inner = NestedParts {
        after: 1,
        id: 7,
        opacity: 0.5,
        blend: BlendMode::Normal,
        clipped: false,
        strokes: vec![stroke(12, StrokeBlend::Erase, [0.1, 0.2, 0.3])],
        edits: Vec::new(),
        mask: None,
    };
    let merged = NestedParts {
        after: parts.layers[0].strokes.len() as u64,
        id: 6,
        opacity: 0.625,
        blend: BlendMode::Screen,
        clipped: true,
        strokes: vec![stroke(11, StrokeBlend::Over, [0.9, 0.3, 0.1])],
        edits: vec![
            EditParts::Fill(FillParts {
                after: 1,
                color: [0.25, 0.5, 0.75],
                alpha_locked: false,
                origin: [30, 20],
                size: [5, 2],
                runs: vec![0, 10],
            }),
            EditParts::Nested(inner),
        ],
        mask: Some(LayerMaskParts {
            enabled: true,
            inverted: true,
            strokes: vec![stroke(13, StrokeBlend::Over, [0.375; 3])],
            edits: Vec::new(),
        }),
    };
    parts.layers[0].edits.push(EditParts::Nested(merged));
    parts.next_layer_id = 8;
    parts.next_stroke_id = 14;
    parts
}
