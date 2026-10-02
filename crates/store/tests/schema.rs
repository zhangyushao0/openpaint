//! The frozen schema-1 sample (`tests/samples/schema-1.bin`): every record of a library holding
//! one project. Opening it must give the sample artwork, and storing the sample artwork must
//! give exactly these bytes, so schema 1 can never change by accident (DECISIONS D09).
//!
//! The file is a record dump: per record the keyspace index (u8: 0 `library`, 1 `projects`),
//! then key and value, each as a u32 little-endian length and the bytes, sorted by keyspace and
//! key. A missing file is written from the current encoding and the test fails, so a new sample
//! is reviewed before it is committed.

mod common;

use std::path::PathBuf;

use common::{KEYSPACES, fresh_dir, records, write_records};
use openpaint_app::ProjectStore;
use openpaint_core::{Artwork, ArtworkParts, LayerParts, SampleParts, StrokeParts};
use openpaint_store::{Library, SCHEMA};

type Records = Vec<(String, Vec<u8>, Vec<u8>)>;

fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples/schema-1.bin")
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

#[test]
fn the_schema_1_sample_round_trips() {
    assert_eq!(SCHEMA, 1, "a new schema ships its own sample and migration");
    let artwork = Artwork::from_storage(parts()).expect("valid sample artwork");
    let dir = fresh_dir("sample-write");
    let mut library = Library::at(&dir);
    library.create(&artwork).expect("create");
    drop(library);
    let written = encode(&records(&dir));
    let path = sample_path();
    let Ok(frozen) = std::fs::read(&path) else {
        std::fs::create_dir_all(path.parent().expect("dir")).expect("samples dir");
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the schema-1 bytes"
    );

    let dir = fresh_dir("sample-read");
    write_records(&dir, &decode(&frozen));
    let project = Library::at(&dir)
        .open_last()
        .expect("the sample opens")
        .expect("the sample holds a project");
    assert_eq!(project.id.get(), 1);
    assert_eq!(project.artwork.to_parts(), parts());
}
