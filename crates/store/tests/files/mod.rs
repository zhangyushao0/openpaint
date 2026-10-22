//! OpenPaint files and kept versions (DECISIONS D62, D63, D95), on the sample artwork: a file
//! holds the artwork exactly (every record, the head without a save id), opens as a new artwork
//! in any library; damaged and foreign files and files of another format are refused without a
//! change; kept versions are capped, listed newest first and restored as copies.

use openpaint_app::{KEPT_VERSIONS, PortError, ProjectId, ProjectStore, Title};
use openpaint_core::{Artwork, ArtworkCommand, LayerCommand};
use openpaint_store::{FORMAT, Library};

use crate::common::{fresh_dir, records};
use crate::sample::{parts, sample_thumbnail, sample_title};

/// The file layout written out by hand, as `artwork_file.rs` documents it: the magic, the
/// format, the title, the records (keys without the project id) and the CRC-32.
fn file_of(format: u32, title: &str, records: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = b"OpenPaint\0".to_vec();
    let blob = |bytes: &mut Vec<u8>, part: &[u8]| {
        bytes.extend_from_slice(&u32::try_from(part.len()).expect("len").to_le_bytes());
        bytes.extend_from_slice(part);
    };
    bytes.extend_from_slice(&format.to_le_bytes());
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

/// A library holding the sample artwork with its title and thumbnail.
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
    // Opened, its head holds a save id, which names a save of this library, not the file's
    // content: the file stays the same.
    let again = other.artwork_file(copy).expect("once opened");
    assert_eq!(again, file, "no save id in a file");
    // In the library it came from it is a second artwork; the first stays as it was.
    let second = library.import_file(&file).expect("opens beside");
    assert_ne!(second, id);
    assert_eq!(library.list().expect("list").len(), 2);
    assert_eq!(library.load(id).expect("first").artwork.to_parts(), parts());
}

/// `file` marked with `format`, its check sum made again.
fn marked(file: &[u8], format: u32) -> Vec<u8> {
    let mut body = file[..file.len() - 4].to_vec();
    body[10..14].copy_from_slice(&format.to_le_bytes());
    [&body[..], &crc32fast::hash(&body).to_le_bytes()].concat()
}

#[test]
fn foreign_damaged_and_other_format_files_are_refused_and_change_nothing() {
    let (mut library, id) = sample_library("file-refused");
    let file = library.artwork_file(id).expect("the file");
    assert_eq!(file[10..14], FORMAT.to_le_bytes(), "the file's format");
    let before = library.list().expect("list");
    let mut flipped = file.clone();
    flipped[file.len() / 2] ^= 1;
    // A whole file whose records break the artwork's rules: no head.
    let headless = file_of(FORMAT, "Hafen", &[]);
    let cases: [(&[u8], Option<u32>); 8] = [
        (b"Shopping list: ink, paper.\n", None),
        (b"", None),
        (&file[..9], None),
        (&flipped, None),
        (&file[..file.len() - 1], None),
        (&headless, None),
        // A later format, and a file made before 1.0 (its schema where the format lies).
        (&marked(&file, 2), Some(2)),
        (&marked(&file, 23), Some(23)),
    ];
    for (at, (bytes, format)) in cases.into_iter().enumerate() {
        let refused = library.import_file(bytes).expect_err("refused");
        let expected = match (at, format) {
            (0..=2, _) => matches!(refused, PortError::NotArtworkFile),
            (_, Some(found)) => refused == PortError::OtherFormat { found: Some(found) },
            _ => matches!(refused, PortError::Corrupt { .. }),
        };
        assert!(expected, "case {at}: {refused:?}");
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
