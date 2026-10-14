//! OpenPaint files and kept versions (DECISIONS D62, D63), on the sample artworks: a file holds
//! the artwork exactly (every record), opens as a new artwork in any library, files of older
//! schemas open upgraded, damaged, foreign and newer files are refused without a change; kept
//! versions are capped, listed newest first and restored as copies.

use openpaint_app::{KEPT_VERSIONS, PortError, ProjectId, ProjectStore, Title};
use openpaint_core::{Artwork, ArtworkCommand, LayerCommand};
use openpaint_store::{Library, SCHEMA};

use crate::common::{fresh_dir, records};
use crate::sample::{
    decode, legacy_parts, parts, sample_path, sample_thumbnail, sample_title, schema_3_parts,
    schema_4_parts, schema_5_parts, schema_6_parts, schema_7_parts, schema_8_parts, schema_9_parts,
    schema_10_parts, schema_11_parts, schema_12_parts, schema_13_parts, schema_14_parts,
    schema_15_parts, schema_16_parts, schema_19_parts,
};

/// The file layout written out by hand, as `artwork_file.rs` documents it: the magic, the
/// schema, the title, the records (keys without the project id) and the CRC-32.
fn file_of(schema: u32, title: &str, records: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = b"OpenPaint\0".to_vec();
    let blob = |bytes: &mut Vec<u8>, part: &[u8]| {
        bytes.extend_from_slice(&u32::try_from(part.len()).expect("len").to_le_bytes());
        bytes.extend_from_slice(part);
    };
    bytes.extend_from_slice(&schema.to_le_bytes());
    blob(&mut bytes, title.as_bytes());
    bytes.extend_from_slice(&u32::try_from(records.len()).expect("count").to_le_bytes());
    for (key, value) in records {
        blob(&mut bytes, key);
        blob(&mut bytes, value);
    }
    let check = crc32fast::hash(&bytes);
    bytes.extend_from_slice(&check.to_le_bytes());
    bytes
}

/// A library holding the sample artwork (schema 20 to 23's) with its title and thumbnail.
fn sample_library(name: &str) -> (Library, ProjectId) {
    let mut library = Library::at(fresh_dir(name));
    let artwork = Artwork::from_storage(parts()).expect("the sample artwork");
    let id = library.create(&artwork, &sample_title()).expect("create");
    library
        .save_thumbnail(id, &sample_thumbnail())
        .expect("thumbnail");
    (library, id)
}

#[test]
fn a_file_holds_the_artwork_exactly_and_opens_as_a_new_one() {
    let (mut library, id) = sample_library("file-source");
    let file = library.artwork_file(id).expect("the file");
    assert!(file.starts_with(b"OpenPaint\0"));
    let mut other = Library::at(fresh_dir("file-target"));
    let copy = other.import_file(&file).expect("opens");
    assert_eq!(
        other.artwork_file(copy).expect("again"),
        file,
        "every record as it was"
    );
    let opened = other.load(copy).expect("load");
    assert_eq!(opened.artwork.to_parts(), parts());
    assert_eq!(opened.title, sample_title());
    let listed = other.list().expect("list");
    assert_eq!(listed[0].thumbnail, Some(sample_thumbnail()));
    assert!(listed[0].versions.is_empty());
    // Opened, its head holds a save id (S54), which a file of it carries; nothing else changed.
    let again = other.artwork_file(copy).expect("once opened");
    assert_eq!((again.len(), &again[..14]), (file.len(), &file[..14]));
    assert_ne!(again, file, "the head's save id");
    // In the library it came from it is a second artwork; the first stays as it was.
    let second = library.import_file(&file).expect("opens beside");
    assert_ne!(second, id);
    assert_eq!(library.list().expect("list").len(), 2);
    assert_eq!(library.load(id).expect("first").artwork.to_parts(), parts());
}

/// The projects records of the frozen sample `schema`, keys without the project id.
fn older_records(schema: u32) -> Vec<(Vec<u8>, Vec<u8>)> {
    let frozen = decode(&std::fs::read(sample_path(schema)).expect("the older sample"));
    frozen
        .into_iter()
        .filter(|(keyspace, key, _)| keyspace == "projects" && key[..8] == 1u64.to_be_bytes())
        .map(|(_, key, value)| (key[8..].to_vec(), value))
        .collect()
}

#[test]
fn files_of_older_schemas_open_upgraded() {
    for schema in 1..SCHEMA {
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
            12 => schema_12_parts(),
            13 => schema_13_parts(),
            14 => schema_14_parts(),
            15 => schema_15_parts(),
            19 => schema_19_parts(),
            20..=22 => parts(),
            _ => schema_16_parts(),
        };
        let file = file_of(schema, "Hafen", &older_records(schema));
        let mut library = Library::at(fresh_dir(&format!("file-{schema}")));
        let id = library.import_file(&file).expect("opens");
        let opened = library.load(id).expect("load");
        assert_eq!(opened.artwork.to_parts(), expected, "schema {schema}");
        assert_eq!(opened.title.as_str(), "Hafen");
        let thumbnail = library.list().expect("list")[0].thumbnail.clone();
        assert_eq!(thumbnail.is_some(), schema >= 2, "schema {schema}");
    }
}

#[test]
fn foreign_damaged_and_newer_files_are_refused_and_change_nothing() {
    let (mut library, id) = sample_library("file-refused");
    let file = library.artwork_file(id).expect("the file");
    let before = library.list().expect("list");
    let mut flipped = file.clone();
    flipped[file.len() / 2] ^= 1;
    let mut newer = file[..file.len() - 4].to_vec();
    newer[10..14].copy_from_slice(&(SCHEMA + 1).to_le_bytes());
    let newer = [&newer[..], &crc32fast::hash(&newer).to_le_bytes()].concat();
    // A whole file whose records break the artwork's rules: no head.
    let headless = file_of(SCHEMA, "Hafen", &[]);
    let cases: [(&[u8], &str); 7] = [
        (b"Shopping list: ink, paper.\n", "foreign"),
        (b"", "foreign"),
        (&file[..9], "foreign"),
        (&flipped, "damaged"),
        (&file[..file.len() - 1], "damaged"),
        (&headless, "damaged"),
        (&newer, "newer"),
    ];
    for (bytes, kind) in cases {
        let refused = library.import_file(bytes).expect_err(kind);
        let expected = match kind {
            "foreign" => matches!(refused, PortError::NotArtworkFile),
            "newer" => {
                refused
                    == PortError::NewerSchema {
                        found: SCHEMA + 1,
                        newest: SCHEMA,
                    }
            }
            _ => matches!(refused, PortError::Corrupt { .. }),
        };
        assert!(expected, "{kind}: {refused:?}");
    }
    assert_eq!(library.list().expect("list"), before);
}

/// `artwork` with its first layer at `opacity`.
fn faded(artwork: &Artwork, opacity: f32) -> Artwork {
    let layer = artwork.layers()[0].id();
    let command = ArtworkCommand::Layer(LayerCommand::SetOpacity { layer, opacity });
    artwork.apply(command).expect("opacity").artwork
}

#[test]
fn versions_are_capped_listed_newest_first_and_restored_as_copies() {
    let (mut library, id) = sample_library("versions");
    let sample = Artwork::from_storage(parts()).expect("the sample artwork");
    let mut kept = Vec::new();
    for at in 0..KEPT_VERSIONS + 2 {
        let artwork = faded(&sample, 0.2 + 0.05 * at as f32);
        library.save(id, &artwork).expect("save");
        library.keep_version(id, 1_000 + at as u64).expect("keep");
        kept.push(artwork);
    }
    let versions = library.list().expect("list")[0].versions.clone();
    assert_eq!(versions.len(), KEPT_VERSIONS, "the oldest two dropped");
    let times: Vec<u64> = versions.iter().map(|version| version.kept_at).collect();
    let newest = 1_000 + KEPT_VERSIONS as u64 + 1;
    assert_eq!(times, (1_002..=newest).rev().collect::<Vec<_>>());
    let oldest = versions[KEPT_VERSIONS - 1];
    let title = Title::new("Hafen (old)").expect("title");
    let copy = library
        .restore_version(id, oldest.id, &title)
        .expect("restore");
    let restored = library.load(copy).expect("the copy");
    assert_eq!(restored.artwork.to_parts(), kept[2].to_parts());
    assert_eq!(restored.title, title);
    let listed = library.list().expect("list");
    assert_eq!(listed[0].id, copy, "the copy is listed first");
    assert!(
        listed[0].versions.is_empty(),
        "a copy starts without versions"
    );
    assert_eq!(listed[1].versions, versions, "the original's versions stay");
    let original = library.load(id).expect("original").artwork;
    assert_eq!(original.to_parts(), kept[KEPT_VERSIONS + 1].to_parts());
    let duplicate = library.duplicate(id).expect("duplicate");
    let listed = library.list().expect("list");
    let copied = listed
        .iter()
        .find(|item| item.id == duplicate)
        .expect("listed");
    assert!(
        copied.versions.is_empty(),
        "a duplicate does not copy versions"
    );
    let missing = library.restore_version(id, openpaint_app::VersionId::new(1), &title);
    assert!(matches!(missing, Err(PortError::Missing(_))), "dropped");
}

#[test]
fn deleting_an_artwork_deletes_its_versions() {
    let dir = fresh_dir("versions-deleted");
    let mut library = Library::at(&dir);
    let artwork = Artwork::from_storage(parts()).expect("the sample artwork");
    let id = library.create(&artwork, &sample_title()).expect("create");
    let other = library.create(&artwork, &sample_title()).expect("create");
    for at in 0..3 {
        library.keep_version(id, at).expect("keep");
    }
    library.keep_version(other, 9).expect("keep other");
    library.delete(id).expect("delete");
    let listed = library.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].versions.len(),
        1,
        "the other artwork's version stays"
    );
    drop(library);
    let gone = |key: &[u8], keyspace: &str| match keyspace {
        "library" => key.first() == Some(&b'v') && key[1..9] == id.get().to_be_bytes(),
        _ => key[..8] == id.get().to_be_bytes(),
    };
    for (keyspace, key, _) in records(&dir) {
        assert!(!gone(&key, &keyspace), "{keyspace} {key:?} left behind");
    }
}
