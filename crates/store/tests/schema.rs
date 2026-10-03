//! The frozen samples (`tests/samples/schema-<n>.bin`), one per schema (DECISIONS D09, D25):
//! every record of a library holding one project with the sample artwork.
//!
//! - Schema 1 (written by S7 to S9): opening it upgrades it to schema 2 and gives the sample
//!   artwork, titled "Untitled Artwork"; every old record is kept, the head only marked 2.
//! - Schema 2: storing the sample artwork with its title and thumbnail gives exactly these
//!   bytes, so schema 2 can never change by accident, and opening it gives them back.
//!
//! A file is a record dump: per record the keyspace index (u8: 0 `library`, 1 `projects`),
//! then key and value, each as a u32 little-endian length and the bytes, sorted by keyspace and
//! key. A missing schema-2 file is written from the current encoding and the test fails, so a
//! new sample is reviewed before it is committed.

mod common;

use std::path::PathBuf;

use common::{KEYSPACES, fresh_dir, records, untitled, write_records};
use openpaint_app::{ProjectStore, Thumbnail, Title};
use openpaint_core::{Artwork, ArtworkParts, LayerParts, SampleParts, StrokeParts};
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

/// Three layers (one deleted before, so ids skip 2), a hidden locked one with a non-ASCII
/// name, half opacity, coloured and translucent strokes, undone stroke ids skipped.
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
            size: 3.5,
            opacity: 0.25,
            color: [0.2, 0.4, 0.6],
            samples: vec![sample(10.0, 40.0, 0.125, 7), sample(11.0, 41.0, 0.0, 9)],
        },
    ];
    let ink = vec![StrokeParts {
        id: 4,
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

#[test]
fn the_schema_1_sample_opens_upgraded_and_keeps_its_records() {
    let frozen = decode(&std::fs::read(sample_path(1)).expect("the schema-1 sample"));
    let dir = fresh_dir("sample-1");
    write_records(&dir, &frozen);
    let mut library = Library::at(&dir);
    let project = library
        .open_last()
        .expect("the sample opens")
        .expect("the sample holds a project");
    assert_eq!(project.id.get(), 1);
    assert_eq!(project.artwork.to_parts(), parts());
    let listed = library.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].id, &listed[0].title), (project.id, &untitled()));
    assert_eq!(listed[0].thumbnail, None);
    drop(library);
    let upgraded = records(&dir);
    for (keyspace, key, value) in &frozen {
        let now = upgraded.iter().find(|(s, k, _)| s == keyspace && k == key);
        let (_, _, now) = now.expect("every schema-1 record is kept");
        if keyspace == "projects" && key.len() == 9 {
            assert_eq!(now[..4], 2u32.to_le_bytes(), "the head is marked schema 2");
            assert_eq!(now[4..], value[4..], "and otherwise unchanged");
        } else {
            assert_eq!(now, value);
        }
    }
    let added: Vec<&[u8]> = upgraded
        .iter()
        .filter(|(s, k, _)| !frozen.iter().any(|(fs, fk, _)| fs == s && fk == k))
        .map(|(_, key, _)| key.as_slice())
        .collect();
    let entry = [&b"a"[..], &1u64.to_be_bytes()].concat();
    assert_eq!(added, [&entry[..], b"clock", b"schema"]);
}

#[test]
fn the_schema_2_sample_round_trips() {
    assert_eq!(SCHEMA, 2, "a new schema ships its own sample and migration");
    let artwork = Artwork::from_storage(parts()).expect("valid sample artwork");
    let dir = fresh_dir("sample-write");
    let mut library = Library::at(&dir);
    let id = library.create(&artwork, &sample_title()).expect("create");
    library
        .save_thumbnail(id, &sample_thumbnail())
        .expect("thumbnail");
    drop(library);
    let written = encode(&records(&dir));
    let path = sample_path(2);
    let Ok(frozen) = std::fs::read(&path) else {
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the schema-2 bytes"
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
}
