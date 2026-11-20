//! The sample artwork's masks and records over its paint (`artwork.rs`): masks, adjustments,
//! filters, transform records, a merged layer, cleared pixels and strokes of the later recipe
//! stages, so every kind of record the format holds is in it.

use openpaint_core::{
    AdjustParts, AdjustmentParts, ArtworkParts, BlendMode, ClearParts, EditParts, FillParts,
    LayerMaskParts, MaskParts, NestedParts, Rotation, ShapeParts, ShapeTexture, Stamp, StrokeBlend,
    StrokeParts, TransformParts,
};

use super::artwork::painted;

/// The sketch layer's mask inverted, with a grey stroke inside a selection, a smudge and a grey
/// fill between them; the ink layer's mask white with one black stroke, and the target (the ink
/// layer is active); the clipping top layer's mask disabled and empty.
fn masked() -> ArtworkParts {
    let mut parts = painted();
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
    let mask = |enabled, inverted, strokes, edits| LayerMaskParts {
        enabled,
        inverted,
        strokes,
        edits,
    };
    let fill = EditParts::Fill(FillParts {
        after: 1,
        before_stroke: 9,
        color: [0.5; 3],
        alpha_locked: false,
        origin: [20, 10],
        size: [4, 4],
        runs: vec![0, 16],
    });
    let sketch = vec![clipped, grey(9, 1.0, StrokeBlend::Smudge)];
    parts.layers[0].mask = Some(mask(true, true, sketch, vec![fill]));
    let ink = vec![grey(10, 0.0, StrokeBlend::Over)];
    parts.layers[1].mask = Some(mask(true, false, ink, Vec::new()));
    parts.layers[2].mask = Some(mask(false, false, Vec::new(), Vec::new()));
    parts.on_mask = true;
    parts.next_stroke_id = 11;
    parts
}

fn adjust(
    (after, before_stroke): (u64, u64),
    clip: Option<MaskParts>,
    inverted: bool,
    what: AdjustmentParts,
) -> EditParts {
    EditParts::Adjust(AdjustParts {
        after,
        before_stroke,
        clip,
        inverted,
        adjustment: what,
    })
}

/// Adjustments: on the sketch layer's paint hue, saturation and brightness inside a selection,
/// a blur inside another, then sharpening of the whole layer; on its inverted mask curves; on the
/// ink layer's mask hue, saturation and brightness over the whole mask. Transform records on the
/// ink layer's paint: a selection scaled up three times and turned, then the whole layer moved
/// and mirrored.
fn adjusted() -> ArtworkParts {
    let mut parts = masked();
    let selected = MaskParts {
        origin: [3, 4],
        size: [3, 2],
        runs: vec![0, 5, 1],
    };
    let blurred = MaskParts {
        origin: [4, 2],
        size: [6, 3],
        runs: vec![2, 4, 1, 6, 2, 3],
    };
    let curves = AdjustmentParts::Curves([
        vec![[0.0, 0.0], [0.25, 0.4], [1.0, 1.0]],
        vec![[0.0, 0.1], [1.0, 0.9]],
        vec![[0.0, 0.0], [1.0, 1.0]],
        vec![[0.0, 0.0], [0.5, 0.25], [0.75, 0.8], [1.0, 1.0]],
    ]);
    let sketch = &mut parts.layers[0];
    let after = sketch.strokes.len() as u64;
    sketch.edits.extend([
        adjust(
            (2, 4),
            Some(selected),
            false,
            AdjustmentParts::Hsb([0.625, 0.25, 0.75]),
        ),
        adjust((after, 7), Some(blurred), false, AdjustmentParts::Blur(6.5)),
        adjust((after, 7), None, false, AdjustmentParts::Sharpen(0.75)),
    ]);
    let mask = sketch.mask.as_mut().expect("the sketch's mask");
    mask.edits.push(adjust((2, 10), None, true, curves));
    let ink = parts.layers[1].mask.as_mut().expect("the ink's mask");
    ink.edits.push(adjust(
        (1, 11),
        None,
        false,
        AdjustmentParts::Hsb([0.5, 0.5, 0.375]),
    ));
    let record = |lift, matrix, translation| {
        EditParts::Transform(TransformParts {
            after: 2,
            before_stroke: 6,
            lift,
            patch: None,
            matrix,
            translation,
        })
    };
    let lift = MaskParts {
        origin: [2, 3],
        size: [5, 2],
        runs: vec![1, 8, 1],
    };
    parts.layers[1].edits.extend([
        record(Some(lift), [0.0, -3.0, 3.0, 0.0], [12.5, 4.0]),
        record(None, [-1.0, 0.0, 0.0, 1.0], [60.0, -2.5]),
    ]);
    parts
}

/// A layer merged down at the end of the sketch layer's content: layer 6, clipped by Screen at
/// 0.625, with one stroke, a fill, a layer merged into it before (layer 7, an eraser stroke at
/// half opacity), the pixels outside a selection cleared (as a pasted copy holds them) and a
/// mask with a grey stroke; then a cut on the sketch layer's paint.
fn merged() -> ArtworkParts {
    let mut parts = adjusted();
    let sketch = &mut parts.layers[0];
    let after = sketch.strokes.len() as u64;
    let layer = merged_layer(&sketch.strokes[1], (after, 7));
    sketch.edits.push(layer);
    sketch.edits.push(EditParts::Clear(ClearParts {
        after,
        before_stroke: 7,
        mask: MaskParts {
            origin: [10, 12],
            size: [6, 2],
            runs: vec![1, 4, 2, 5],
        },
    }));
    parts.next_layer_id = 8;
    parts.next_stroke_id = 14;
    parts
}

/// Layer 6 as `merged` describes it, its strokes made from `model`, at `(after, before_stroke)`.
fn merged_layer(model: &StrokeParts, (after, before_stroke): (u64, u64)) -> EditParts {
    let stroke = |id, blend, color: [f32; 3]| {
        let mut stroke = model.clone();
        (stroke.id, stroke.blend, stroke.color, stroke.clip) = (id, blend, color, None);
        stroke
    };
    let inner = NestedParts {
        after: 1,
        before_stroke: 12,
        id: 7,
        opacity: 0.5,
        blend: BlendMode::Normal,
        clipped: false,
        strokes: vec![stroke(12, StrokeBlend::Erase, [0.1, 0.2, 0.3])],
        edits: Vec::new(),
        mask: None,
    };
    let fill = EditParts::Fill(FillParts {
        after: 1,
        before_stroke: 12,
        color: [0.25, 0.5, 0.75],
        alpha_locked: false,
        origin: [30, 20],
        size: [5, 2],
        runs: vec![0, 10],
    });
    let outside = EditParts::Clear(ClearParts {
        after: 1,
        before_stroke: 12,
        mask: MaskParts {
            origin: [0, 0],
            size: [64, 3],
            runs: vec![0, 70, 4, 118],
        },
    });
    EditParts::Nested(NestedParts {
        after,
        before_stroke,
        id: 6,
        opacity: 0.625,
        blend: BlendMode::Screen,
        clipped: true,
        strokes: vec![stroke(11, StrokeBlend::Over, [0.9, 0.3, 0.1])],
        edits: vec![fill, EditParts::Nested(inner), outside],
        mask: Some(LayerMaskParts {
            enabled: true,
            inverted: true,
            strokes: vec![stroke(13, StrokeBlend::Over, [0.375; 3])],
            edits: Vec::new(),
        }),
    })
}

/// The sample artwork whole: at the end of the ink layer's paint a stroke whose size widens and
/// whose opacity fades with tilt, tapered at both ends, and a stroke of rough squares half as
/// long as wide, an eighth of its size apart, scattered, turned to the stroke's direction less 30
/// degrees, jittered in turn and size by its own seed.
pub fn parts() -> ArtworkParts {
    let mut parts = merged();
    let mut tilted = parts.layers[0].strokes[0].clone();
    (tilted.id, tilted.blend, tilted.clip) = (14, StrokeBlend::Over, None);
    tilted.recipe.size_tilt = vec![[0.0, 1.0], [40.0, 1.0], [80.0, 2.5]];
    tilted.recipe.opacity_tilt = vec![[0.0, 1.0], [40.0, 1.0], [80.0, 0.375]];
    tilted.recipe.taper = [2.0, 3.5, 0.125];
    let mut stamped = parts.layers[0].strokes[0].clone();
    (stamped.id, stamped.blend, stamped.clip) = (15, StrokeBlend::Over, None);
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
    parts.layers[1].strokes.extend([tilted, stamped]);
    parts.next_stroke_id = 16;
    each_stroke(&mut parts, |stroke| {
        stroke.began_ms = if stroke.id == 4 {
            0
        } else {
            BEGAN_MS + stroke.id * 60_000
        };
    });
    parts
}

/// 2026-11-14 21:07 UTC: the sample's strokes began a minute apart from it, by their ids, but
/// stroke 4, whose start is unknown (as a stroke kept before format 2).
const BEGAN_MS: u64 = 1_794_690_420_000;

/// `set` on every stroke of `parts`: layers', masks' and merged layers' at any depth.
fn each_stroke(parts: &mut ArtworkParts, mut set: impl FnMut(&mut StrokeParts)) {
    fn list(
        strokes: &mut [StrokeParts],
        edits: &mut [EditParts],
        set: &mut dyn FnMut(&mut StrokeParts),
    ) {
        strokes.iter_mut().for_each(&mut *set);
        for edit in edits {
            if let EditParts::Nested(nested) = edit {
                list(&mut nested.strokes, &mut nested.edits, set);
                if let Some(mask) = nested.mask.as_mut() {
                    list(&mut mask.strokes, &mut mask.edits, set);
                }
            }
        }
    }
    for layer in &mut parts.layers {
        list(&mut layer.strokes, &mut layer.edits, &mut set);
        if let Some(mask) = layer.mask.as_mut() {
            list(&mut mask.strokes, &mut mask.edits, &mut set);
        }
    }
}
