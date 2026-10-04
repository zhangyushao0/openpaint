//! The frozen samples (`tests/samples/schema-<n>.bin`), one per schema (DECISIONS D09, D36,
//! D38, D43, D45, D46, D49, D50, D51): every record of a library holding one project with the
//! sample artwork.
//!
//! - Schema 1 (written by S7 to S9), 2 (S10), 3 (S11), 4 (S12 to S14), 5 (S15), 6 (S16 and
//!   S17), 7 (S18 and S19), 8 (S20), 9 (S21 and S22), 10 (S23), 11 (S24 to S27) and 12 (S28 to
//!   S30): opening any of them upgrades it to schema 13 and gives the sample artwork as that
//!   schema held it; no old transform is a record without pixels; no old mask holds an
//!   adjustment; before schema 11 no layer has a
//!   mask; before schema 10 no layer clips, blends
//!   other than Normal or lies in a group; before schema 9 no stroke smudges; before schema 8
//!   no layer has a transform; before schema 7 no stroke has a clip (nothing was ever drawn
//!   inside a selection); before schema 6 every stroke covers (crossings never darken), as every
//!   stroke did then; before schema 5 no layer has fills; before schema 4 every stroke paints
//!   over and no layer is alpha-locked; schemas 1 and 2 give every stroke the plain round brush,
//!   the only brush there was. Every old record is kept: heads marked 13 and given (before 4)
//!   Alpha Lock off, (before 5) a fill count of 0 per layer, (before 10) no clipping, Normal and
//!   no group per layer and no groups, (before 11) no mask per layer and paint as the target;
//!   strokes given (before 4) their blend, (before 3) the
//!   plain recipe after their colour, (before 6) the deposit "cover" after their recipe, and
//!   (before 7) "no clip" after their samples; fills given (before 8) the kind "fill" in front;
//!   the toolbox given (before 4) the eraser and black, (before 6) StreamLine off for every
//!   brush, and (before 9) the smudge at its defaults.
//! - Schema 13: storing the sample artwork (schema 12's, with adjustments on the sketch's paint
//!   inside a selection and on two masks, and on the ink layer two transform records without
//!   pixels, a selection's and the whole layer's) with its title, thumbnail and toolbox gives
//!   exactly these bytes, so schema 13 can never change by accident, and opening it gives them
//!   back.
//!
//! A file is a record dump: per record the keyspace index (u8: 0 `library`, 1 `projects`),
//! then key and value, each as a u32 little-endian length and the bytes, sorted by keyspace and
//! key. A missing schema-13 file is written from the current encoding and the test fails, so a
//! new sample is reviewed before it is committed.

mod common;
mod sample;

use common::{fresh_dir, records, untitled, write_records};
use openpaint_app::ProjectStore;
use openpaint_core::Artwork;
use openpaint_store::{Library, SCHEMA};
use sample::{
    decode, encode, legacy_parts, parts, sample_editor, sample_editor_6, sample_editor_9,
    sample_path, sample_thumbnail, sample_title, schema_3_parts, schema_4_parts, schema_5_parts,
    schema_6_parts, schema_7_parts, schema_8_parts, schema_9_parts, schema_10_parts,
    schema_11_parts, schema_12_parts, upgraded_editor,
};

/// Bytes of a stroke record before the place schema 3 puts the recipe and schema 4 the blend.
const STYLE_BYTES: usize = 28;
/// Layers in the sample's head, each of which schema 4 gives one Alpha Lock byte, schema 5
/// a fill count of four bytes, schema 10 clipping, blend and group (ten bytes) and schema 11
/// "no mask" (one byte).
const LAYERS: usize = 3;

/// Opens the older sample `schema`: it upgrades to schema 13 and holds the sample artwork as
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
        5 => schema_5_parts(),
        6 => schema_6_parts(),
        7 => schema_7_parts(),
        8 => schema_8_parts(),
        9 => schema_9_parts(),
        10 => schema_10_parts(),
        11 => schema_11_parts(),
        _ => schema_12_parts(),
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
    // Clipping, blend and group per layer, and the group count.
    let groups = if schema < 10 { 10 * LAYERS + 4 } else { 0 };
    // No mask per layer, paint as the target.
    let masks = if schema < 11 { LAYERS + 1 } else { 0 };
    match (keyspace, key.get(8)) {
        ("projects", Some(b'h')) => {
            assert_eq!(
                now[..4],
                13u32.to_le_bytes(),
                "the head is marked schema 13"
            );
            let grown = alpha_lock + fill_counts + groups + masks;
            assert_eq!(now.len(), value.len() + grown);
            if schema < 11 {
                assert_eq!(now[now.len() - 1], 0, "paint is the target");
            } else {
                assert_eq!(now[4..], value[4..], "only the mark changes");
            }
            if schema < 10 {
                assert_eq!(now[now.len() - 5..now.len() - 1], [0; 4], "no groups");
            }
        }
        ("projects", Some(b's')) if schema >= 7 => assert_eq!(now, value, "the stroke"),
        ("projects", Some(b's')) => {
            let (clip, now) = now.split_last().expect("a stroke");
            assert_eq!(*clip, 0, "no clip");
            if schema == 6 {
                assert_eq!(now, value, "the rest of the stroke");
            } else {
                kept_stroke(schema, value, now);
            }
        }
        ("projects", Some(b'f')) if schema < 8 => {
            assert_eq!((now[0], &now[1..]), (0, value), "the kind, then the fill");
        }
        ("library", _) if key == b"schema" => assert_eq!(now, &13u32.to_le_bytes()),
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
fn the_schema_4_to_12_samples_open_upgraded_and_keep_their_records() {
    for schema in [4, 5, 6, 7, 8, 9, 10, 11, 12] {
        let (mut library, added) = open_older(schema);
        let listed = library.list().expect("list");
        assert_eq!(listed[0].title, sample_title());
        assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
        let editor = match schema {
            9.. => sample_editor_9(),
            6.. => sample_editor_6(),
            _ => sample_editor(),
        };
        assert_eq!(library.editor(), Ok(Some(editor)));
        assert!(added.is_empty(), "{added:?}");
    }
}

#[test]
fn the_schema_13_sample_round_trips() {
    assert_eq!(
        SCHEMA, 13,
        "a new schema ships its own sample and migration"
    );
    let artwork = Artwork::from_storage(parts()).expect("valid sample artwork");
    let dir = fresh_dir("sample-write");
    let mut library = Library::at(&dir);
    let id = library.create(&artwork, &sample_title()).expect("create");
    library
        .save_thumbnail(id, &sample_thumbnail())
        .expect("thumbnail");
    library.save_editor(&sample_editor_9()).expect("toolbox");
    drop(library);
    let written = encode(&records(&dir));
    let path = sample_path(13);
    let Ok(frozen) = std::fs::read(&path) else {
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the schema-13 bytes"
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
    assert_eq!(library.editor(), Ok(Some(sample_editor_9())));
}
