//! Format 1 (DECISIONS D95) and its frozen sample (`tests/samples/format-1.bin`): every record of
//! a library holding one project with the sample artwork (every kind of record the format has:
//! strokes of every blend, recipe stage and clip, fills, cleared pixels, transforms with and
//! without pixels, adjustments and filters, masks, groups, a merged layer holding a merged layer)
//! with its title, thumbnail and toolbox (every field set).
//!
//! Storing them as a new project gives exactly the frozen bytes, so format 1 can never change by
//! accident; opening the sample gives them back, and storing what it opened gives the same bytes
//! again: every record round-trips byte for byte. A library of any other format, or one made
//! before 1.0, is refused and left exactly as it was.
//!
//! A sample file is a record dump: per record the keyspace index (u8: 0 `library`, 1
//! `projects`), then key and value, each as a u32 little-endian length and the bytes, sorted by
//! keyspace and key. A missing sample is written from the current encoding and the test fails, so
//! a new sample is reviewed before it is committed.

mod common;
mod files;
mod sample;

use std::path::Path;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{BrushLibrary, BrushSet, PortError, ProjectStore, Title, Toolbox};
use openpaint_core::Artwork;
use openpaint_store::{FORMAT, Library};
use sample::{
    Records, decode, encode, parts, sample_editor, sample_path, sample_thumbnail, sample_title,
};

/// The sample artwork, thumbnail and toolbox stored into a new library at `dir`, as its records.
fn stored(dir: &Path, artwork: &Artwork, title: &Title) -> Records {
    let mut library = Library::at(dir);
    let id = library.create(artwork, title).expect("create");
    library
        .save_thumbnail(id, &sample_thumbnail())
        .expect("thumbnail");
    library.save_editor(&sample_editor()).expect("toolbox");
    drop(library);
    records(dir)
}

#[test]
fn the_format_1_sample_round_trips() {
    assert_eq!(FORMAT, 1, "another format ships its own sample");
    let artwork = Artwork::from_storage(parts()).expect("valid sample artwork");
    let written = encode(&stored(
        &TestDir::new("sample-write"),
        &artwork,
        &sample_title(),
    ));
    let path = sample_path();
    let Ok(frozen) = std::fs::read(&path) else {
        let samples = path.parent().expect("the samples' directory");
        std::fs::create_dir_all(samples).expect("the samples' directory");
        std::fs::write(&path, &written).expect("write sample");
        panic!(
            "wrote a new sample at {}; review it and commit it",
            path.display()
        );
    };
    assert!(
        written == frozen,
        "storing the sample artwork no longer gives the format-1 bytes"
    );
    let dir = TestDir::new("sample-read");
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

#[test]
fn every_record_kind_round_trips_byte_for_byte() {
    let frozen = decode(&std::fs::read(sample_path()).expect("the sample"));
    let dir = TestDir::new("sample-reread");
    write_records(&dir, &frozen);
    let mut library = Library::at(&dir);
    let opened = library
        .load(openpaint_app::ProjectId::new(1))
        .expect("opens");
    assert_eq!(library.editor(), Ok(Some(sample_editor())));
    drop(library);
    let again = stored(
        &TestDir::new("sample-restored"),
        &opened.artwork,
        &opened.title,
    );
    for (record, other) in frozen.iter().zip(&again) {
        assert_eq!(record, other, "{}: {:?}", record.0, record.1);
    }
    assert_eq!(again.len(), frozen.len());
}

/// Every file under `dir` and its bytes, by path.
fn files(dir: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("a directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(files(&path));
        } else {
            let bytes = std::fs::read(&path).expect("a file");
            found.push((path, bytes));
        }
    }
    found.sort();
    found
}

/// The library at `dir` (closed) opened by this build: every call refused as `found`, nothing
/// written, every file byte for byte as it was. It is opened and closed once first, so the files
/// compared are what fjall itself leaves on opening (its journal recovered, old versions gone).
fn refused_untouched(dir: &Path, found: Option<u32>) {
    let held = records(dir);
    let before = files(dir);
    let other = Err(PortError::OtherFormat { found });
    let mut library = Library::at(dir);
    assert_eq!(library.open_last().map(|_| ()), other);
    assert_eq!(library.list().map(|_| ()), other);
    assert_eq!(library.editor().map(|_| ()), other);
    let artwork = Artwork::from_storage(parts()).expect("the sample artwork");
    assert_eq!(library.create(&artwork, &untitled()).map(|_| ()), other);
    assert_eq!(library.save_editor(&sample_editor()), other);
    drop(library);
    let after = files(dir);
    let changed: Vec<_> = (after.iter().filter(|file| !before.contains(file)))
        .map(|(path, bytes)| (path.clone(), bytes.len()))
        .collect();
    let gone: Vec<_> = (before.iter().filter(|file| !after.contains(file)))
        .map(|(path, bytes)| (path.clone(), bytes.len()))
        .collect();
    assert!(
        changed.is_empty() && gone.is_empty(),
        "{changed:?} {gone:?}"
    );
    assert_eq!(records(dir), held);
}

#[test]
fn a_library_of_another_format_is_refused_untouched() {
    let frozen = decode(&std::fs::read(sample_path()).expect("the sample"));
    // Format 2, as a later build might write it.
    let mut later = frozen.clone();
    let format = later.iter_mut().find(|(_, key, _)| key == b"format");
    format.expect("the format record").2 = 2u32.to_le_bytes().to_vec();
    let dir = TestDir::new("format-2");
    write_records(&dir, &later);
    refused_untouched(&dir, Some(2));
    // A library made before 1.0: its schema, a project, no format.
    let mut older: Records = frozen
        .into_iter()
        .filter(|(_, key, _)| key != b"format")
        .collect();
    older.push((
        "library".into(),
        b"schema".to_vec(),
        23u32.to_le_bytes().to_vec(),
    ));
    let dir = TestDir::new("schema-23");
    write_records(&dir, &older);
    refused_untouched(&dir, None);
}

#[test]
fn an_empty_library_is_made_format_1() {
    let dir = TestDir::new("empty");
    let mut library = Library::at(&dir);
    assert_eq!(library.list(), Ok(Vec::new()));
    drop(library);
    let made = records(&dir);
    let format = ("library".to_owned(), b"format".to_vec(), vec![1, 0, 0, 0]);
    assert_eq!(made, [format]);
}

#[test]
fn the_sample_s_toolbox_from_before_hard_paint_gets_it_first_in_painting() {
    // The frozen toolbox predates Hard Paint (S104). The record holds built-ins by key and a
    // brush in no stored set goes where a first start puts it (D74), so the bytes as they are
    // give it: no upgrade step, the arrangement kept (Textured Round where it was stored).
    let frozen = decode(&std::fs::read(sample_path()).expect("the sample"));
    let dir = TestDir::new("sample-toolbox");
    write_records(&dir, &frozen);
    let state = Library::at(&dir)
        .editor()
        .expect("read")
        .expect("a toolbox");
    let mut toolbox = Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"));
    toolbox.restore(&state);
    assert!(toolbox.dropped().is_empty());
    let names = |set: BrushSet| -> Vec<String> {
        let brush = toolbox.projection();
        brush.rows(set).iter().map(|row| row.name.clone()).collect()
    };
    assert_eq!(
        names(BrushSet::Painting),
        [
            "Hard Paint",
            "Speckle",
            "Hard Round",
            "Soft Brush",
            "Flat",
            "Opaque Paint"
        ]
    );
    assert_eq!(names(BrushSet::Texture), ["Textured Round"]);
}
