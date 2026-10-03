//! The data of the frozen samples (`tests/samples/schema-<n>.bin`): the sample artwork as each
//! schema held it, its title, thumbnail and toolbox, and the record dump format.

use std::path::PathBuf;

use openpaint_app::{BrushChoice, BrushSetting, EditorState, Thumbnail, Title, Tool};
use openpaint_core::{
    AccumulationParts, ArtworkParts, BrushRecipe, FillParts, LayerParts, MaskParts, PaperParts,
    PaperTexture, RecipeParts, SampleParts, StrokeBlend, StrokeParts, Tip,
};

use crate::common::KEYSPACES;

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
        strokes,
        fills: Vec::new(),
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
        active: 3,
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
pub fn parts() -> ArtworkParts {
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
    parts.layers[0].fills.push(FillParts {
        after: 1,
        color: [0.75, 0.25, 0.5],
        alpha_locked: true,
        origin: [10, 8],
        size: [6, 3],
        runs: vec![0, 8, 1, 9],
    });
    parts.layers[2].fills.push(FillParts {
        after: 0,
        color: [0.0, 0.5, 1.0],
        alpha_locked: false,
        origin: [40, 30],
        size: [3, 2],
        runs: vec![1, 5],
    });
    parts
}

pub fn encode(records: &Records) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (keyspace, key, value) in records {
        let index = KEYSPACES
            .iter()
            .position(|name| name == keyspace)
            .expect("known");
        bytes.push(u8::try_from(index).expect("small"));
        for part in [key, value] {
            bytes.extend_from_slice(&u32::try_from(part.len()).expect("len").to_le_bytes());
            bytes.extend_from_slice(part);
        }
    }
    bytes
}

fn take<'a>(bytes: &'a [u8], at: &mut usize, len: usize) -> &'a [u8] {
    let part = &bytes[*at..*at + len];
    *at += len;
    part
}

pub fn decode(bytes: &[u8]) -> Records {
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        let index = take(bytes, &mut at, 1)[0];
        let mut part = || {
            let len = take(bytes, &mut at, 4).try_into().expect("length");
            take(bytes, &mut at, u32::from_le_bytes(len) as usize).to_vec()
        };
        let (key, value) = (part(), part());
        out.push((KEYSPACES[usize::from(index)].to_owned(), key, value));
    }
    out
}

pub fn sample_title() -> Title {
    Title::new("Hafen ✒ 墨").expect("title")
}

/// Two pixels: opaque red, half-transparent grey.
pub fn sample_thumbnail() -> Thumbnail {
    Thumbnail::new([2, 1], vec![255, 0, 0, 255, 128, 128, 128, 128]).expect("thumbnail")
}

fn setting(key: &str, size: f32, opacity: f32) -> BrushSetting {
    BrushSetting {
        key: key.to_owned(),
        size,
        opacity,
        streamline: 0.0,
    }
}

/// The brush tool's choice of the schema-3 sample: the pencil current, small and light, the
/// ink pen at its default; StreamLine off, as every brush had it before schema 6.
fn pencil_choice() -> BrushChoice {
    BrushChoice {
        brush: "pencil".into(),
        brushes: vec![setting("pencil", 3.5, 0.25), setting("ink-pen", 12.0, 1.0)],
    }
}

/// The schema-3 sample's toolbox as schema 4 reads it after the upgrade.
pub fn upgraded_editor() -> EditorState {
    EditorState {
        tool: Tool::Brush,
        paint: pencil_choice(),
        erase: BrushChoice {
            brush: String::new(),
            brushes: Vec::new(),
        },
        color: [0.0; 3],
        recent: Vec::new(),
    }
}

/// The toolbox of the schema-6 sample: the schema-5 one with StreamLine on the ink pen and on
/// the eraser's soft brush.
pub fn sample_editor_6() -> EditorState {
    let mut editor = sample_editor();
    editor.paint.brushes[1].streamline = 0.625;
    editor.erase.brushes[0].streamline = 1.0;
    editor
}

/// The eraser current on the soft brush at half opacity; an orange colour, two recent ones.
/// The toolbox of the schema-4 and schema-5 samples.
pub fn sample_editor() -> EditorState {
    EditorState {
        tool: Tool::Eraser,
        paint: pencil_choice(),
        erase: BrushChoice {
            brush: "soft-brush".into(),
            brushes: vec![setting("soft-brush", 64.0, 0.5)],
        },
        color: [1.0, 0.5, 0.125],
        recent: vec![[1.0, 0.5, 0.125], [0.0, 0.25, 0.75]],
    }
}
