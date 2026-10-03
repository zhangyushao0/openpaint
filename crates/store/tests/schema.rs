//! The frozen samples (`tests/samples/schema-<n>.bin`), one per schema (DECISIONS D09, D25,
//! D28, D29): every record of a library holding one project with the sample artwork.
//!
//! - Schema 1 (written by S7 to S9), 2 (S10) and 3 (S11): opening any of them upgrades it to
//!   schema 4 and gives the sample artwork as that schema held it, every stroke painting over
//!   and no layer alpha-locked; schemas 1 and 2 give every stroke the plain round brush, the
//!   only brush there was. Every old record is kept: heads marked 4 and given Alpha Lock off
//!   per layer, strokes given their blend (and before schema 3 the plain recipe) after their
//!   colour, the toolbox given the eraser and black.
//! - Schema 4: storing the sample artwork (an alpha-locked layer, a stroke painted atop, an
//!   eraser stroke) with its title, thumbnail and toolbox gives exactly these bytes, so schema 4
//!   can never change by accident, and opening it gives them back.
//!
//! A file is a record dump: per record the keyspace index (u8: 0 `library`, 1 `projects`),
//! then key and value, each as a u32 little-endian length and the bytes, sorted by keyspace and
//! key. A missing schema-4 file is written from the current encoding and the test fails, so a
//! new sample is reviewed before it is committed.

mod common;

use std::path::PathBuf;

use common::{KEYSPACES, fresh_dir, records, untitled, write_records};
use openpaint_app::{BrushChoice, BrushSetting, EditorState, ProjectStore, Thumbnail, Title, Tool};
use openpaint_core::{
    Artwork, ArtworkParts, BrushRecipe, LayerParts, RecipeParts, SampleParts, StrokeBlend,
    StrokeParts,
};
use openpaint_store::{Library, SCHEMA};

type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

fn sample_path(schema: u32) -> PathBuf {
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
    }
}

/// The sample artwork as schemas 1 and 2 stored it, opened by schema 4: every stroke plain.
fn legacy_parts() -> ArtworkParts {
    let mut parts = schema_3_parts();
    for layer in &mut parts.layers {
        for stroke in &mut layer.strokes {
            stroke.recipe = BrushRecipe::plain().to_parts();
        }
    }
    parts
}

/// Three layers (one deleted before, so ids skip 2), a hidden locked one with a non-ASCII
/// name, half opacity, coloured and translucent strokes of two recipes, undone stroke ids
/// skipped: the sample of schema 3, every stroke painting over, no layer alpha-locked.
fn schema_3_parts() -> ArtworkParts {
    let layer = |id, name: &str, visible, opacity, locked, strokes| LayerParts {
        id,
        name: name.to_owned(),
        visible,
        opacity,
        locked,
        alpha_locked: false,
        strokes,
    };
    let stroke = |id, recipe, size, opacity, color, samples| StrokeParts {
        id,
        blend: StrokeBlend::Over,
        recipe,
        size,
        opacity,
        color,
        samples,
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
fn parts() -> ArtworkParts {
    let mut parts = schema_3_parts();
    parts.layers[0].alpha_locked = true;
    parts.layers[0].strokes[1].blend = StrokeBlend::Atop;
    let mut erase = parts.layers[0].strokes[0].clone();
    erase.id = 5;
    erase.blend = StrokeBlend::Erase;
    parts.layers[1].strokes.push(erase);
    parts
}

fn encode(records: &Records) -> Vec<u8> {
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

fn decode(bytes: &[u8]) -> Records {
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

fn sample_title() -> Title {
    Title::new("Hafen ✒ 墨").expect("title")
}

/// Two pixels: opaque red, half-transparent grey.
fn sample_thumbnail() -> Thumbnail {
    Thumbnail::new([2, 1], vec![255, 0, 0, 255, 128, 128, 128, 128]).expect("thumbnail")
}

fn setting(key: &str, size: f32, opacity: f32) -> BrushSetting {
    BrushSetting {
        key: key.to_owned(),
        size,
        opacity,
    }
}

/// The brush tool's choice of the schema-3 sample: the pencil current, small and light, the
/// ink pen at its default.
fn pencil_choice() -> BrushChoice {
    BrushChoice {
        brush: "pencil".into(),
        brushes: vec![setting("pencil", 3.5, 0.25), setting("ink-pen", 12.0, 1.0)],
    }
}

/// The schema-3 sample's toolbox as schema 4 reads it after the upgrade.
fn upgraded_editor() -> EditorState {
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

/// The eraser current on the soft brush at half opacity; an orange colour, two recent ones.
fn sample_editor() -> EditorState {
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

/// Bytes of a stroke record before the place schema 3 puts the recipe and schema 4 the blend.
const STYLE_BYTES: usize = 28;
/// Layers in the sample's head, each of which schema 4 gives one Alpha Lock byte.
const LAYERS: usize = 3;

/// Opens the older sample `schema`: it upgrades to schema 4 and holds the sample artwork as
/// that schema held it. Every old record is kept, as the module documentation lists. Returns
/// the library (still open) and the keys the upgrade added.
fn open_older(schema: u32) -> (Library, Vec<Vec<u8>>) {
    let frozen = decode(&std::fs::read(sample_path(schema)).expect("the older sample"));
    let dir = fresh_dir(&format!("sample-{schema}"));
    write_records(&dir, &frozen);
    let mut library = Library::at(&dir);
    let project = library
        .open_last()
        .expect("the sample opens")
        .expect("the sample holds a project");
    assert_eq!(project.id.get(), 1);
    let expected = if schema < 3 {
        legacy_parts()
    } else {
        schema_3_parts()
    };
    assert_eq!(project.artwork.to_parts(), expected);
    drop(library);
    let upgraded = records(&dir);
    // Bytes schema 3 put after the colour: the plain recipe, when the record had none.
    let recipe = if schema < 3 { plain_recipe_length() } else { 0 };
    for (keyspace, key, value) in &frozen {
        let now = upgraded.iter().find(|(s, k, _)| s == keyspace && k == key);
        let (_, _, now) = now.expect("every older record is kept");
        match (keyspace.as_str(), key.get(8)) {
            ("projects", Some(b'h')) => {
                assert_eq!(now[..4], 4u32.to_le_bytes(), "the head is marked schema 4");
                assert_eq!(
                    now.len(),
                    value.len() + LAYERS,
                    "and given Alpha Lock per layer"
                );
            }
            ("projects", Some(b's')) => {
                assert_eq!(now[..STYLE_BYTES], value[..STYLE_BYTES]);
                assert_eq!(now[STYLE_BYTES], 0, "painting over");
                assert_eq!(now[STYLE_BYTES + 1 + recipe..], value[STYLE_BYTES..]);
            }
            ("library", _) if key == b"schema" => assert_eq!(now, &4u32.to_le_bytes()),
            ("library", _) if key == b"editor" => {
                assert_eq!(now[0], 0, "the brush tool");
                assert_eq!(now[1..=value.len()], value[..], "the brush choice kept");
            }
            _ => assert_eq!(now, value),
        }
    }
    let added = upgraded
        .iter()
        .filter(|(s, k, _)| !frozen.iter().any(|(fs, fk, _)| fs == s && fk == k))
        .map(|(_, key, _)| key.clone())
        .collect();
    (Library::at(&dir), added)
}

/// Bytes of the plain recipe in a stroke record: softness, four curves of five points in all,
/// grain.
fn plain_recipe_length() -> usize {
    4 + 4 * 4 + 5 * 8 + 4 + 4
}

#[test]
fn the_schema_1_sample_opens_upgraded_and_keeps_its_records() {
    let (mut library, added) = open_older(1);
    let listed = library.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].id.get(), &listed[0].title), (1, &untitled()));
    assert_eq!(listed[0].thumbnail, None);
    let entry = [&b"a"[..], &1u64.to_be_bytes()].concat();
    assert_eq!(added, [entry, b"clock".to_vec(), b"schema".to_vec()]);
}

#[test]
fn the_schema_2_sample_opens_upgraded_and_keeps_its_records() {
    let (mut library, added) = open_older(2);
    let listed = library.list().expect("list");
    assert_eq!(listed[0].title, sample_title());
    assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
    assert_eq!(
        library.editor(),
        Ok(None),
        "the toolbox starts at its defaults"
    );
    assert!(added.is_empty(), "{added:?}");
}

#[test]
fn the_schema_3_sample_opens_upgraded_and_keeps_its_records() {
    let (mut library, added) = open_older(3);
    let listed = library.list().expect("list");
    assert_eq!(listed[0].title, sample_title());
    assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
    assert_eq!(library.editor(), Ok(Some(upgraded_editor())));
    assert!(added.is_empty(), "{added:?}");
}

#[test]
fn the_schema_4_sample_round_trips() {
    assert_eq!(SCHEMA, 4, "a new schema ships its own sample and migration");
    let artwork = Artwork::from_storage(parts()).expect("valid sample artwork");
    let dir = fresh_dir("sample-write");
    let mut library = Library::at(&dir);
    let id = library.create(&artwork, &sample_title()).expect("create");
    library
        .save_thumbnail(id, &sample_thumbnail())
        .expect("thumbnail");
    library.save_editor(&sample_editor()).expect("toolbox");
    drop(library);
    let written = encode(&records(&dir));
    let path = sample_path(4);
    let Ok(frozen) = std::fs::read(&path) else {
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the schema-4 bytes"
    );

    let dir = fresh_dir("sample-read");
    write_records(&dir, &decode(&frozen));
    let mut library = Library::at(&dir);
    let project = library
        .open_last()
        .expect("the sample opens")
        .expect("the sample holds a project");
    assert_eq!(project.id.get(), 1);
    assert_eq!(project.artwork.to_parts(), parts());
    let listed = library.list().expect("list");
    assert_eq!(listed[0].title, sample_title());
    assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
    assert_eq!(library.editor(), Ok(Some(sample_editor())));
}
