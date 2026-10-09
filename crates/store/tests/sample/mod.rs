//! The data of the frozen samples (`tests/samples/schema-<n>.bin`): the sample artwork as each
//! schema held it, its title, thumbnail and toolbox, and the record dump format.

use std::path::PathBuf;

use openpaint_app::{Thumbnail, Title};
use openpaint_core::{
    AccumulationParts, ArtworkParts, BlendMode, BrushRecipe, EditParts, FillParts, GroupParts,
    LayerParts, MaskParts, PaperParts, PaperTexture, PatchParts, RecipeParts, SampleParts,
    StrokeBlend, StrokeParts, Tip, TransformParts,
};

mod dump;
mod editor;
mod schema_11;
mod schema_12;
mod schema_13;
mod schema_14;
mod schema_15;
mod schema_16;
mod schema_19;
pub use dump::{decode, encode};
pub use editor::{
    sample_editor, sample_editor_6, sample_editor_9, sample_editor_17, sample_editor_18,
    upgraded_editor,
};
pub use schema_11::schema_11_parts;
pub use schema_12::schema_12_parts;
pub use schema_13::schema_13_parts;
pub use schema_14::schema_14_parts;
pub use schema_15::schema_15_parts;
pub use schema_16::schema_16_parts;
pub use schema_19::parts;

pub type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

pub fn sample_path(schema: u32) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/samples/schema-{schema}.bin"))
}

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

/// A recipe with every stage set: soft, curves of several points, speed, grain.
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
        // No tilt and no taper: as every recipe before schema 19.
        ..BrushRecipe::plain().to_parts()
    }
}

/// A recipe building up graphite on the sketch paper, as the S16 pencil does.
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

/// An accumulating recipe with a bell tip and no paper, as the S16 soft brush.
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

/// The sample artwork as schemas 1 and 2 stored it, opened now: every stroke plain.
pub fn legacy_parts() -> ArtworkParts {
    let mut parts = schema_3_parts();
    for layer in &mut parts.layers {
        for stroke in &mut layer.strokes {
            stroke.recipe = BrushRecipe::plain().to_parts();
        }
    }
    parts
}

/// A layer of the schema-3 sample: no Alpha Lock, no fills.
fn layer(
    id: u64,
    name: &str,
    visible: bool,
    opacity: f32,
    locked: bool,
    strokes: Vec<StrokeParts>,
) -> LayerParts {
    LayerParts {
        id,
        name: name.to_owned(),
        visible,
        opacity,
        locked,
        alpha_locked: false,
        clipping: false,
        blend: BlendMode::Normal,
        group: None,
        strokes,
        edits: Vec::new(),
        mask: None,
    }
}

/// Three layers (one deleted before, so ids skip 2), a hidden locked one with a non-ASCII
/// name, half opacity, coloured and translucent strokes of two recipes, undone stroke ids
/// skipped: the sample of schema 3, every stroke painting over, no layer alpha-locked.
pub fn schema_3_parts() -> ArtworkParts {
    let stroke = |id, recipe, size, opacity, color, samples| StrokeParts {
        id,
        blend: StrokeBlend::Over,
        recipe,
        size,
        opacity,
        color,
        samples,
        clip: None,
    };
    let sketch = vec![
        stroke(
            1,
            grainy(),
            12.0,
            1.0,
            [0.0, 0.0, 0.0],
            vec![
                sample(4.0, 5.0, 0.25, 0),
                sample(30.5, 9.25, 1.0, 16),
                sample(60.0, -3.0, 0.5, 16),
            ],
        ),
        stroke(
            3,
            BrushRecipe::plain().to_parts(),
            3.5,
            0.25,
            [0.2, 0.4, 0.6],
            vec![sample(10.0, 40.0, 0.125, 7), sample(11.0, 41.0, 0.0, 9)],
        ),
    ];
    let ink = vec![stroke(
        4,
        grainy(),
        1000.0,
        0.5,
        [1.0, 0.5, 0.0],
        vec![sample(32.0, 24.0, 1.0, 0)],
    )];
    ArtworkParts {
        width: 64,
        height: 48,
        layers: vec![
            layer(1, "Sketch", true, 0.5, false, sketch),
            layer(3, "Tinte ✒ 墨", false, 1.0, true, ink),
            layer(4, "Layer 4", true, 1.0, false, Vec::new()),
        ],
        groups: Vec::new(),
        active: 3,
        on_mask: false,
        next_layer_id: 5,
        next_stroke_id: 6,
    }
}

/// The schema-3 sample with what schema 4 adds: the sketch layer alpha-locked with its second
/// stroke painted atop, and an eraser stroke on the ink layer.
pub fn schema_4_parts() -> ArtworkParts {
    let mut parts = schema_3_parts();
    parts.layers[0].alpha_locked = true;
    parts.layers[0].strokes[1].blend = StrokeBlend::Atop;
    let mut erase = parts.layers[0].strokes[0].clone();
    erase.id = 5;
    erase.blend = StrokeBlend::Erase;
    parts.layers[1].strokes.push(erase);
    parts
}

/// The schema-5 sample with what schema 6 adds: the sketch's second stroke builds up graphite
/// on paper, the ink layer's dot is an airbrush.
pub fn schema_6_parts() -> ArtworkParts {
    let mut parts = schema_5_parts();
    parts.layers[0].strokes[1].recipe = graphite();
    parts.layers[1].strokes[0].recipe = airbrush();
    parts
}

/// The schema-6 sample with what schema 7 adds: both sketch strokes drawn inside one selection
/// with a hole, and the ink layer's eraser stroke inside another.
pub fn schema_7_parts() -> ArtworkParts {
    let mut parts = schema_6_parts();
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

/// The schema-4 sample with what schema 5 adds: a fill with a one-pixel hole between the
/// sketch's strokes, alpha-locked like its layer, and a fill on the empty top layer whose mask
/// ends inside.
pub fn schema_5_parts() -> ArtworkParts {
    let mut parts = schema_4_parts();
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
    parts
}

/// The schema-9 sample with what schema 10 adds: the ink and top layers in a hidden, faded
/// group with a non-ASCII name, the top layer clipping to the ink layer, the ink layer by
/// Multiply and the sketch by Overlay.
pub fn schema_10_parts() -> ArtworkParts {
    let mut parts = schema_9_parts();
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

/// The schema-8 sample with what schema 9 adds: a smudge on the alpha-locked sketch layer after
/// its transform, and a smudge on the top layer over its transformed fill.
pub fn schema_9_parts() -> ArtworkParts {
    let mut parts = schema_8_parts();
    let mut locked = parts.layers[0].strokes[0].clone();
    locked.id = 6;
    locked.blend = StrokeBlend::SmudgeLocked;
    locked.clip = None;
    parts.layers[0].strokes.push(locked);
    let mut smudge = parts.layers[0].strokes[1].clone();
    smudge.id = 7;
    smudge.blend = StrokeBlend::Smudge;
    smudge.clip = None;
    parts.layers[2].strokes.push(smudge);
    parts.next_stroke_id = 8;
    parts
}

/// The schema-7 sample with what schema 8 adds: on the sketch layer, after both strokes, the
/// pixels of a selection turned a quarter and moved; on the top layer, after its fill, the
/// whole layer scaled and mirrored.
pub fn schema_8_parts() -> ArtworkParts {
    let mut parts = schema_7_parts();
    let pixels = |count: u8| -> Vec<u8> {
        (0..count)
            .flat_map(|i| [i * 9, 255 - i * 9, i * 3, 128 + i])
            .collect()
    };
    parts.layers[0]
        .edits
        .push(EditParts::Transform(TransformParts {
            after: 2,
            lift: Some(MaskParts {
                origin: [4, 6],
                size: [4, 3],
                runs: vec![1, 10, 1],
            }),
            patch: Some(PatchParts {
                origin: [4, 6],
                size: [4, 3],
                pixels: pixels(12),
            }),
            matrix: [0.0, -1.0, 1.0, 0.0],
            translation: [10.5, -2.25],
        }));
    parts.layers[2]
        .edits
        .push(EditParts::Transform(TransformParts {
            after: 0,
            lift: None,
            patch: Some(PatchParts {
                origin: [40, 30],
                size: [3, 2],
                pixels: pixels(6),
            }),
            matrix: [-1.5, 0.0, 0.0, 1.5],
            translation: [100.0, -15.0],
        }));
    parts
}

pub fn sample_title() -> Title {
    Title::new("Hafen ✒ 墨").expect("title")
}

/// Two pixels: opaque red, half-transparent grey.
pub fn sample_thumbnail() -> Thumbnail {
    Thumbnail::new([2, 1], vec![255, 0, 0, 255, 128, 128, 128, 128]).expect("thumbnail")
}
