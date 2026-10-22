//! The sample artwork's layers and paint: three layers (one deleted before, so ids skip 2), a
//! hidden locked one with a non-ASCII name, half opacity, coloured and translucent strokes of
//! several recipes, undone stroke ids skipped; Alpha Lock, an eraser stroke, fills, graphite on
//! paper and an airbrush, strokes drawn inside selections, pixels moved and a picture placed,
//! smudges, a faded hidden group with a clipping layer and blend modes. `records.rs` adds the
//! rest.

use openpaint_core::{
    AccumulationParts, ArtworkParts, BlendMode, BrushRecipe, EditParts, FillParts, GroupParts,
    LayerParts, MaskParts, PaperParts, PaperTexture, PatchParts, RecipeParts, SampleParts,
    StrokeBlend, StrokeParts, Tip, TransformParts,
};

fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> SampleParts {
    SampleParts {
        x,
        y,
        pressure,
        tilt: 0.75,
        azimuth: -2.5,
        time_ms,
    }
}

/// A recipe with these stages set: soft, curves of several points, speed, grain.
fn grainy() -> RecipeParts {
    RecipeParts {
        softness: 0.375,
        size_pressure: vec![[0.0, 0.25], [0.5, 0.75], [1.0, 1.0]],
        size_speed: vec![[0.0, 1.0], [4.0, 0.5]],
        opacity_pressure: vec![[0.0, 0.125], [1.0, 1.0]],
        opacity_speed: vec![[0.0, 1.0]],
        grain_strength: 0.625,
        grain_scale: 3,
        deposit: None,
        ..BrushRecipe::plain().to_parts()
    }
}

/// A recipe building up graphite on the sketch paper, as the pencil does.
fn graphite() -> RecipeParts {
    RecipeParts {
        softness: 0.0,
        size_pressure: vec![[0.0, 0.3], [1.0, 1.0]],
        size_speed: vec![[0.0, 1.0]],
        opacity_pressure: vec![[0.0, 0.25], [1.0, 1.0]],
        opacity_speed: vec![[0.0, 1.0]],
        grain_strength: 0.0,
        grain_scale: 1,
        deposit: Some(AccumulationParts {
            tip: Tip::Flat,
            flow: 3.125,
            max_density: 0.875,
            paper: Some(PaperParts {
                texture: PaperTexture::Sketch,
                contact: vec![[0.0, 0.375], [1.0, 0.9375]],
                scale: 4.5,
                depth: 0.25,
                dust: 0.375,
            }),
        }),
        ..BrushRecipe::plain().to_parts()
    }
}

/// An accumulating recipe with a bell tip and no paper, as the soft brush.
fn airbrush() -> RecipeParts {
    RecipeParts {
        deposit: Some(AccumulationParts {
            tip: Tip::Bell,
            flow: 1.25,
            max_density: 1.0,
            paper: None,
        }),
        ..BrushRecipe::plain().to_parts()
    }
}

fn layer(id: u64, name: &str, opacity: f32, strokes: Vec<StrokeParts>) -> LayerParts {
    LayerParts {
        id,
        name: name.to_owned(),
        visible: true,
        opacity,
        locked: false,
        alpha_locked: false,
        clipping: false,
        blend: BlendMode::Normal,
        group: None,
        strokes,
        edits: Vec::new(),
        mask: None,
    }
}

/// The three layers and their strokes: the sketch layer alpha-locked, its second stroke painted
/// atop in graphite; the hidden, locked ink layer an airbrush dot and an eraser stroke.
fn layers() -> ArtworkParts {
    let stroke = |id, recipe, size, opacity, color, samples| StrokeParts {
        id,
        blend: StrokeBlend::Over,
        recipe,
        size,
        opacity,
        color,
        samples,
        clip: None,
        seed: 0,
    };
    let first = vec![
        sample(4.0, 5.0, 0.25, 0),
        sample(30.5, 9.25, 1.0, 16),
        sample(60.0, -3.0, 0.5, 16),
    ];
    let mut atop = stroke(3, graphite(), 3.5, 0.25, [0.2, 0.4, 0.6], Vec::new());
    atop.samples = vec![sample(10.0, 40.0, 0.125, 7), sample(11.0, 41.0, 0.0, 9)];
    atop.blend = StrokeBlend::Atop;
    let sketch = vec![
        stroke(1, grainy(), 12.0, 1.0, [0.0; 3], first.clone()),
        atop,
    ];
    let dot = vec![sample(32.0, 24.0, 1.0, 0)];
    let mut erase = stroke(5, grainy(), 12.0, 1.0, [0.0; 3], first);
    erase.blend = StrokeBlend::Erase;
    let ink = vec![
        stroke(4, airbrush(), 1000.0, 0.5, [1.0, 0.5, 0.0], dot),
        erase,
    ];
    let mut layers = vec![
        layer(1, "Sketch", 0.5, sketch),
        layer(3, "Tinte ✒ 墨", 1.0, ink),
        layer(4, "Layer 4", 1.0, Vec::new()),
    ];
    layers[0].alpha_locked = true;
    (layers[1].visible, layers[1].locked) = (false, true);
    ArtworkParts {
        width: 64,
        height: 48,
        layers,
        groups: Vec::new(),
        active: 3,
        on_mask: false,
        next_layer_id: 5,
        next_stroke_id: 6,
    }
}

/// Fills (one with a one-pixel hole between the sketch's strokes, alpha-locked like its layer;
/// one on the empty top layer whose mask ends inside), strokes drawn inside selections (both
/// sketch strokes in one with a hole, the eraser stroke in another).
fn filled() -> ArtworkParts {
    let mut parts = layers();
    parts.layers[0].edits.push(EditParts::Fill(FillParts {
        after: 1,
        color: [0.75, 0.25, 0.5],
        alpha_locked: true,
        origin: [10, 8],
        size: [6, 3],
        runs: vec![0, 8, 1, 9],
    }));
    parts.layers[2].edits.push(EditParts::Fill(FillParts {
        after: 0,
        color: [0.0, 0.5, 1.0],
        alpha_locked: false,
        origin: [40, 30],
        size: [3, 2],
        runs: vec![1, 5],
    }));
    let holed = MaskParts {
        origin: [2, 1],
        size: [30, 20],
        runs: vec![0, 300, 5, 295],
    };
    parts.layers[0].strokes[0].clip = Some(holed.clone());
    parts.layers[0].strokes[1].clip = Some(holed);
    parts.layers[1].strokes[1].clip = Some(MaskParts {
        origin: [60, 40],
        size: [4, 8],
        runs: vec![3, 29],
    });
    parts
}

/// Pixels moved: on the sketch layer, after both strokes, a selection's pixels turned a quarter
/// and moved; on the top layer, after its fill, a picture placed scaled and mirrored. Then
/// smudges: one on the alpha-locked sketch layer, one on the top layer over its picture.
fn moved() -> ArtworkParts {
    let mut parts = filled();
    let pixels = |count: u8| -> Vec<u8> {
        (0..count)
            .flat_map(|i| [i * 9, 255 - i * 9, i * 3, 128 + i])
            .collect()
    };
    let lift = MaskParts {
        origin: [4, 6],
        size: [4, 3],
        runs: vec![1, 10, 1],
    };
    let patch = |origin, size: [u32; 2]| PatchParts {
        origin,
        size,
        pixels: pixels((size[0] * size[1]) as u8),
    };
    parts.layers[0]
        .edits
        .push(EditParts::Transform(TransformParts {
            after: 2,
            lift: Some(lift),
            patch: Some(patch([4, 6], [4, 3])),
            matrix: [0.0, -1.0, 1.0, 0.0],
            translation: [10.5, -2.25],
        }));
    parts.layers[2]
        .edits
        .push(EditParts::Transform(TransformParts {
            after: 0,
            lift: None,
            patch: Some(patch([40, 30], [3, 2])),
            matrix: [-1.5, 0.0, 0.0, 1.5],
            translation: [100.0, -15.0],
        }));
    let mut locked = parts.layers[0].strokes[0].clone();
    (locked.id, locked.blend, locked.clip) = (6, StrokeBlend::SmudgeLocked, None);
    parts.layers[0].strokes.push(locked);
    let mut smudge = parts.layers[0].strokes[1].clone();
    (smudge.id, smudge.blend, smudge.clip) = (7, StrokeBlend::Smudge, None);
    parts.layers[2].strokes.push(smudge);
    parts.next_stroke_id = 8;
    parts
}

/// The ink and top layers in a hidden, faded group with a non-ASCII name, the top layer clipping
/// to the ink layer, the ink layer by Multiply and the sketch by Overlay.
pub fn painted() -> ArtworkParts {
    let mut parts = moved();
    parts.layers[0].blend = BlendMode::Overlay;
    parts.layers[1].blend = BlendMode::Multiply;
    parts.layers[1].group = Some(5);
    parts.layers[2].group = Some(5);
    parts.layers[2].clipping = true;
    parts.groups.push(GroupParts {
        id: 5,
        name: "Gruppe ✒ 组".to_owned(),
        visible: false,
        opacity: 0.625,
    });
    parts.next_layer_id = 6;
    parts
}
