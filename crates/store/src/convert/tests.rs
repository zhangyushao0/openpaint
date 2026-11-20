//! The conversion from format 1 (D125) on the frozen format-1 sample (`tests/samples/format-1.bin`):
//! every stroke record gains an unknown start after its id and every edit record its place by the
//! rule, nothing else changes; a kept version and an imported file convert alike; a write failing
//! anywhere, or a damaged record, leaves the library format 1 byte for byte.

use openpaint_app::{ProjectId, ProjectStore, Title};

use crate::Library;
use crate::artwork_file::MAGIC;
use crate::batch::faults;
use crate::codec::Writer;
use crate::records::{LIBRARY, PROJECTS, mask_slot};
use crate::robustness_tests::{Dump, copied, dump, library_of, records};
use crate::test_dir::TestDir;

/// The format-1 sample's edit records' places by the rule, worked out by hand from
/// `tests/sample/artwork.rs` and `records.rs`: slot, index, place.
const PLACES: [(u64, u64, u64); 17] = [
    // The sketch layer's strokes 1, 3, 6: a fill after 1, a transform and an adjustment after 2,
    // a blur, sharpening, the merged layer and a cut after 3.
    (1, 0, 2),
    (1, 1, 4),
    (1, 2, 4),
    (1, 3, 7),
    (1, 4, 7),
    (1, 5, 7),
    (1, 6, 7),
    // The ink layer's strokes 4, 5, 14, 15: two transforms after 2.
    (3, 0, 6),
    (3, 1, 6),
    // The top layer's stroke 7: a fill and a picture placed before it.
    (4, 0, 7),
    (4, 1, 7),
    // Merged layer 6's stroke 11: a fill, merged layer 7 and the pixels cleared after it.
    (6, 0, 12),
    (6, 1, 12),
    (6, 2, 12),
    // The sketch's mask's strokes 8, 9: a fill after 1, curves after 2; the ink's mask's 10.
    (mask_slot(1), 0, 9),
    (mask_slot(1), 1, 10),
    (mask_slot(3), 0, 11),
];

/// The sample's title, which its catalogue entry holds.
fn title() -> Title {
    Title::new("Hafen ✒ 墨").expect("title")
}

/// The slot and index of a stroke or edit key under a project.
fn place(key: &[u8]) -> (u64, u64) {
    let slot = key[9..17].try_into().expect("a slot");
    let index = key[17..25].try_into().expect("an index");
    (u64::from_be_bytes(slot), u64::from_be_bytes(index))
}

#[test]
fn every_stroke_and_edit_record_converts_by_the_rule_and_nothing_else_changes() {
    let old = dump("format-1.bin");
    let dir = library_of("convert", &old);
    assert!(Library::at(&dir).list().is_ok(), "converted");
    let new = records(&dir);
    assert_eq!(new.len(), old.len());
    let mut edits = 0;
    for ((space, key, before), (_, same, after)) in old.iter().zip(&new) {
        assert_eq!(key, same);
        match (*space, key.get(8)) {
            (LIBRARY, _) if key == b"format" => assert_eq!(after[..], 2u32.to_le_bytes()),
            (PROJECTS, Some(b's')) => {
                assert_eq!(*after, [&before[..8], &[0; 8], &before[8..]].concat());
            }
            (PROJECTS, Some(b'e')) => {
                let (slot, index) = place(key);
                let listed = PLACES.iter().find(|(s, i, _)| (*s, *i) == (slot, index));
                let stroke = listed.expect("every edit is listed").2;
                let placed = [&before[..5], &stroke.to_le_bytes(), &before[5..]].concat();
                assert_eq!(*after, placed, "slot {slot} index {index}");
                edits += 1;
            }
            _ => assert_eq!(after, before),
        }
    }
    assert_eq!(edits, PLACES.len());
    let artwork = Library::at(&dir).artwork(ProjectId::new(1));
    assert!(artwork.is_ok(), "it opens: {artwork:?}");
}

#[test]
fn a_write_failing_anywhere_leaves_the_library_format_1() {
    let old = dump("format-1.bin");
    let base = library_of("convert-base", &old);
    let counted = copied(&base, "convert-count");
    let start = faults::writes();
    assert!(Library::at(&counted).list().is_ok());
    let writes = faults::writes() - start;
    assert!(
        writes > PLACES.len(),
        "every record and the format: {writes}"
    );
    for n in 1..=writes {
        let dir = copied(&base, "convert-failing");
        faults::fail_write(n);
        assert!(Library::at(&dir).list().is_err(), "write {n} of {writes}");
        assert!(records(&dir) == old, "write {n}: format 1 as it was");
        assert!(
            Library::at(&dir).list().is_ok(),
            "write {n}: converted later"
        );
    }
}

#[test]
fn a_damaged_record_converts_nothing() {
    let mut old = dump("format-1.bin");
    let stroke = old.iter_mut().find(|(_, key, _)| key.get(8) == Some(&b's'));
    stroke.expect("a stroke").2.truncate(6);
    let dir = library_of("convert-damaged", &old);
    assert!(Library::at(&dir).list().is_err());
    assert!(records(&dir) == old, "format 1 as it was");
}

/// Project 1 of `records` as a format-1 OpenPaint file titled as the sample: its records but its
/// versions, keys without the project's id.
fn format_1_file(records: &Dump) -> Vec<u8> {
    let held: Vec<_> = records
        .iter()
        .filter(|(space, key, _)| *space == PROJECTS && key[..8] == 1u64.to_be_bytes())
        .filter(|(_, key, _)| key[8] != b'v')
        .collect();
    let mut writer = Writer::with_capacity(1024);
    writer.bytes(MAGIC);
    writer.u32(1);
    writer.str(title().as_str());
    writer.len(held.len());
    for (_, key, value) in held {
        writer.blob(&key[8..]);
        writer.blob(value);
    }
    let mut file = writer.finish();
    let sum = crc32fast::hash(&file);
    file.extend_from_slice(&sum.to_le_bytes());
    file
}

#[test]
fn a_kept_version_and_an_imported_file_convert_as_the_library_does() {
    let mut old = dump("format-1.bin");
    let file = format_1_file(&old);
    let entry = [&b"v"[..], &1u64.to_be_bytes(), &1u64.to_be_bytes()].concat();
    let kept = [&1u64.to_be_bytes()[..], b"v", &1u64.to_be_bytes()].concat();
    old.push((LIBRARY, entry, 100u64.to_le_bytes().to_vec()));
    old.push((PROJECTS, kept.clone(), file.clone()));
    old.sort();
    let dir = library_of("convert-versions", &old);
    let mut library = Library::at(&dir);
    let versions = library.list().expect("converted")[0].versions.clone();
    assert_eq!(versions.len(), 1);
    let one = ProjectId::new(1);
    let converted = library.artwork(one).expect("converted").to_parts();
    let made = library.artwork_file(one).expect("the file it makes now");
    let copy = library
        .restore_version(one, versions[0].id, &title())
        .expect("restored");
    assert_eq!(library.artwork(copy).expect("copy").to_parts(), converted);
    drop(library);
    let version = records(&dir).into_iter().find(|(_, key, _)| *key == kept);
    let version = version.expect("the version").2;
    assert_eq!(version[10..14], 2u32.to_le_bytes(), "format 2");
    assert!(
        version == made,
        "the version is the file of the converted project"
    );
    let target = TestDir::new("convert-import");
    let mut other = Library::at(&target);
    let imported = other.import_file(&file).expect("a format-1 file converts");
    assert_eq!(other.artwork_file(imported).expect("again"), made);
}
