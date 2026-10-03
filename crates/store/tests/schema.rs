//! The frozen samples (`tests/samples/schema-<n>.bin`), one per schema (DECISIONS D09, D25,
//! D27): every record of a library holding one project with the sample artwork.
//!
//! - Schema 1 (written by S7 to S9) and schema 2 (S10): opening either upgrades it to schema 3
//!   and gives the sample artwork with every stroke drawn by the plain round brush, the only
//!   brush there was; every old record is kept, heads only marked 3, strokes only given that
//!   recipe after their colour.
//! - Schema 3: storing the sample artwork (strokes of two recipes) with its title, thumbnail and
//!   toolbox gives exactly these bytes, so schema 3 can never change by accident, and opening
//!   it gives them back.
//!
//! A file is a record dump: per record the keyspace index (u8: 0 `library`, 1 `projects`),
//! then key and value, each as a u32 little-endian length and the bytes, sorted by keyspace and
//! key. A missing schema-3 file is written from the current encoding and the test fails, so a
//! new sample is reviewed before it is committed.

mod common;

use std::path::PathBuf;

use common::{KEYSPACES, fresh_dir, records, untitled, write_records};
use openpaint_app::{BrushSetting, EditorState, ProjectStore, Thumbnail, Title};
use openpaint_core::{
    Artwork, ArtworkParts, BrushRecipe, LayerParts, RecipeParts, SampleParts, StrokeParts,
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

/// The sample artwork as schemas 1 and 2 stored it, opened by schema 3: every stroke plain.
fn legacy_parts() -> ArtworkParts {
    let mut parts = parts();
    for layer in &mut parts.layers {
        for stroke in &mut layer.strokes {
            stroke.recipe = BrushRecipe::plain().to_parts();
        }
    }
    parts
}

/// Three layers (one deleted before, so ids skip 2), a hidden locked one with a non-ASCII
/// name, half opacity, coloured and translucent strokes of two recipes, undone stroke ids
/// skipped.
fn parts() -> ArtworkParts {
    let layer = |id, name: &str, visible, opacity, locked, strokes| LayerParts {
        id,
        name: name.to_owned(),
        visible,
        opacity,
        locked,
        strokes,
    };
    let sketch = vec![
        StrokeParts {
            id: 1,
            recipe: grainy(),
            size: 12.0,
            opacity: 1.0,
            color: [0.0, 0.0, 0.0],
            samples: vec![
                sample(4.0, 5.0, 0.25, 0),
                sample(30.5, 9.25, 1.0, 16),
                sample(60.0, -3.0, 0.5, 16),
            ],
        },
        StrokeParts {
            id: 3,
            recipe: BrushRecipe::plain().to_parts(),
            size: 3.5,
            opacity: 0.25,
            color: [0.2, 0.4, 0.6],
            samples: vec![sample(10.0, 40.0, 0.125, 7), sample(11.0, 41.0, 0.0, 9)],
        },
    ];
    let ink = vec![StrokeParts {
        id: 4,
        recipe: grainy(),
        size: 1000.0,
        opacity: 0.5,
        color: [1.0, 0.5, 0.0],
        samples: vec![sample(32.0, 24.0, 1.0, 0)],
    }];
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

/// The pencil current; the pencil small and light, the ink pen at its default.
fn sample_editor() -> EditorState {
    let setting = |key: &str, size, opacity| BrushSetting {
        key: key.to_owned(),
        size,
        opacity,
    };
    EditorState {
        brush: "pencil".into(),
        brushes: vec![setting("pencil", 3.5, 0.25), setting("ink-pen", 12.0, 1.0)],
    }
}

/// Bytes of a schema-2 stroke record before the place schema 3 puts the recipe.
const STYLE_BYTES: usize = 28;

/// Opens the older sample `schema`: it upgrades to schema 3 and holds the sample artwork with
/// every stroke plain. Every old record is kept: heads only marked 3, strokes only given the
/// plain recipe after their colour, the library's schema 3. Returns the library (still open)
/// and the keys the upgrade added.
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
    assert_eq!(project.artwork.to_parts(), legacy_parts());
    drop(library);
    let upgraded = records(&dir);
    let plain = upgraded_recipe_length();
    for (keyspace, key, value) in &frozen {
        let now = upgraded.iter().find(|(s, k, _)| s == keyspace && k == key);
        let (_, _, now) = now.expect("every older record is kept");
        match (keyspace.as_str(), key.get(8)) {
            ("projects", Some(b'h')) => {
                assert_eq!(now[..4], 3u32.to_le_bytes(), "the head is marked schema 3");
                assert_eq!(now[4..], value[4..], "and otherwise unchanged");
            }
            ("projects", Some(b's')) => {
                assert_eq!(now[..STYLE_BYTES], value[..STYLE_BYTES]);
                assert_eq!(now[STYLE_BYTES + plain..], value[STYLE_BYTES..]);
            }
            ("library", _) if key == b"schema" => assert_eq!(now, &3u32.to_le_bytes()),
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
fn upgraded_recipe_length() -> usize {
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
fn the_schema_3_sample_round_trips() {
    assert_eq!(SCHEMA, 3, "a new schema ships its own sample and migration");
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
    let path = sample_path(3);
    let Ok(frozen) = std::fs::read(&path) else {
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the schema-3 bytes"
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
