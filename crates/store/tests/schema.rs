//! The frozen samples (`tests/samples/schema-<n>.bin`), one per schema (DECISIONS D09, D25,
//! D28, D29, D36, D38): every record of a library holding one project with the sample artwork.
//!
//! - Schema 1 (written by S7 to S9), 2 (S10), 3 (S11), 4 (S12 to S14) and 5 (S15): opening any
//!   of them upgrades it to schema 6 and gives the sample artwork as that schema held it; every
//!   old stroke covers (crossings never darken), as every stroke did before; before schema 5
//!   no layer has fills; before schema 4 every stroke paints over and no layer is
//!   alpha-locked; schemas 1 and 2 give every stroke the plain round brush, the only brush
//!   there was. Every old record is kept: heads marked 6 and given (before 4) Alpha Lock off
//!   and (before 5) a fill count of 0 per layer; strokes given (before 4) their blend, (before
//!   3) the plain recipe after their colour, and the deposit "cover" after their recipe; the
//!   toolbox given (before 4) the eraser and black, and StreamLine off for every brush.
//! - Schema 6: storing the sample artwork (schema 5's with a stroke on paper and an airbrush
//!   dot) with its title, thumbnail and toolbox gives exactly these bytes, so schema 6 can never
//!   change by accident, and opening it gives them back.
//!
//! A file is a record dump: per record the keyspace index (u8: 0 `library`, 1 `projects`),
//! then key and value, each as a u32 little-endian length and the bytes, sorted by keyspace and
//! key. A missing schema-6 file is written from the current encoding and the test fails, so a
//! new sample is reviewed before it is committed.

mod common;
mod sample;

use common::{fresh_dir, records, untitled, write_records};
use openpaint_app::ProjectStore;
use openpaint_core::Artwork;
use openpaint_store::{Library, SCHEMA};
use sample::{
    decode, encode, legacy_parts, parts, sample_editor, sample_editor_6, sample_path,
    sample_thumbnail, sample_title, schema_3_parts, schema_4_parts, schema_5_parts,
    upgraded_editor,
};

/// Bytes of a stroke record before the place schema 3 puts the recipe and schema 4 the blend.
const STYLE_BYTES: usize = 28;
/// Layers in the sample's head, each of which schema 4 gives one Alpha Lock byte and schema 5
/// a fill count of four bytes.
const LAYERS: usize = 3;

/// Opens the older sample `schema`: it upgrades to schema 6 and holds the sample artwork as
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
    let expected = match schema {
        1 | 2 => legacy_parts(),
        3 => schema_3_parts(),
        4 => schema_4_parts(),
        _ => schema_5_parts(),
    };
    assert_eq!(project.artwork.to_parts(), expected);
    drop(library);
    let upgraded = records(&dir);
    for (keyspace, key, value) in &frozen {
        let now = upgraded.iter().find(|(s, k, _)| s == keyspace && k == key);
        let (_, _, now) = now.expect("every older record is kept");
        kept(schema, keyspace, key, value, now);
    }
    let added = upgraded
        .iter()
        .filter(|(s, k, _)| !frozen.iter().any(|(fs, fk, _)| fs == s && fk == k))
        .map(|(_, key, _)| key.clone())
        .collect();
    (Library::at(&dir), added)
}

/// One record of the older sample `schema` (`value`) as the upgrade left it (`now`). The
/// toolbox is checked by what it opens as.
fn kept(schema: u32, keyspace: &str, key: &[u8], value: &[u8], now: &[u8]) {
    let alpha_lock = if schema < 4 { LAYERS } else { 0 };
    let fill_counts = if schema < 5 { 4 * LAYERS } else { 0 };
    match (keyspace, key.get(8)) {
        ("projects", Some(b'h')) => {
            assert_eq!(now[..4], 6u32.to_le_bytes(), "the head is marked schema 6");
            assert_eq!(now.len(), value.len() + alpha_lock + fill_counts);
        }
        ("projects", Some(b's')) => kept_stroke(schema, value, now),
        ("library", _) if key == b"schema" => assert_eq!(now, &6u32.to_le_bytes()),
        ("library", _) if key == b"editor" => {}
        _ => assert_eq!(now, value),
    }
}

/// A stroke keeps its style, blend (from 4), recipe (from 3) and samples, given the blend
/// (before 4), the plain recipe (before 3) and the deposit "cover" right before its samples.
fn kept_stroke(schema: u32, value: &[u8], now: &[u8]) {
    assert_eq!(now[..STYLE_BYTES], value[..STYLE_BYTES]);
    if schema < 4 {
        assert_eq!(now[STYLE_BYTES], 0, "painting over");
    } else {
        assert_eq!(now[STYLE_BYTES], value[STYLE_BYTES], "its blend");
    }
    // Where the old record's recipe starts and ends (none before schema 3).
    let start = STYLE_BYTES + usize::from(schema >= 4);
    let end = if schema >= 3 {
        recipe_end(value, start)
    } else {
        start
    };
    let samples = &value[end..];
    let deposit = now.len() - samples.len() - 1;
    assert_eq!(now[deposit + 1..], *samples, "the samples");
    assert_eq!(now[deposit], 0, "covering");
    let recipe = &now[STYLE_BYTES + 1..deposit];
    if schema < 3 {
        assert_eq!(recipe.len(), plain_recipe_length(), "the plain recipe");
    } else {
        assert_eq!(recipe, &value[start..end], "its recipe");
    }
}

/// Where the recipe starting at `at` ends: softness, four curves of a count and points, grain.
fn recipe_end(record: &[u8], at: usize) -> usize {
    let mut at = at + 4;
    for _ in 0..4 {
        let count = u32::from_le_bytes(record[at..at + 4].try_into().expect("count"));
        at += 4 + 8 * count as usize;
    }
    at + 8
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
fn the_schema_4_and_5_samples_open_upgraded_and_keep_their_records() {
    for schema in [4, 5] {
        let (mut library, added) = open_older(schema);
        let listed = library.list().expect("list");
        assert_eq!(listed[0].title, sample_title());
        assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
        assert_eq!(library.editor(), Ok(Some(sample_editor())));
        assert!(added.is_empty(), "{added:?}");
    }
}

#[test]
fn the_schema_6_sample_round_trips() {
    assert_eq!(SCHEMA, 6, "a new schema ships its own sample and migration");
    let artwork = Artwork::from_storage(parts()).expect("valid sample artwork");
    let dir = fresh_dir("sample-write");
    let mut library = Library::at(&dir);
    let id = library.create(&artwork, &sample_title()).expect("create");
    library
        .save_thumbnail(id, &sample_thumbnail())
        .expect("thumbnail");
    library.save_editor(&sample_editor_6()).expect("toolbox");
    drop(library);
    let written = encode(&records(&dir));
    let path = sample_path(6);
    let Ok(frozen) = std::fs::read(&path) else {
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the schema-6 bytes"
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
    assert_eq!(library.editor(), Ok(Some(sample_editor_6())));
}
