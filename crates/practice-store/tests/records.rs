//! Practice records in the library's record space (D112): a session starts with its sheet in one
//! write, its record round-trips and is replaced whole, its sheet (a project its record owns, the
//! runtime's gallery list leaves out) opens as the last artwork, the records before practice keep
//! their bytes, and a session removed (the test-session chore's) takes its record and sheet with
//! it and leaves every other byte; a record that cannot be read is left out.

#[path = "../../store/tests/common/mod.rs"]
mod common;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{ExerciseResult, Layout, Metric, SessionRecord, met};
use openpaint_practice_store::{remove, save, sessions, start};
use openpaint_store::Library;

fn sheet() -> Artwork {
    let [width, height] = Layout::of(&[None; 18]).size;
    Artwork::new(CanvasSize::new(width, height).expect("size"))
}

#[test]
fn a_session_round_trips_with_its_sheet_a_project_its_record_owns() {
    let dir = TestDir::new("practice");
    let mut library = Library::at(&dir);
    let artwork = library
        .create(
            &Artwork::new(CanvasSize::new(64, 48).expect("size")),
            &untitled(),
        )
        .expect("an artwork");
    drop(library);
    let before = records(&dir);
    let mut library = Library::at(&dir);
    let session = SessionRecord::stage_one(1, 1, 0x5EED_0045, 1_791_201_600, &[]);
    let title = Title::new("Practice 1").expect("title");
    let id = start(&mut library, (&sheet(), &title), &session).expect("started");
    assert_ne!(id, artwork);
    let stored = sessions(&mut library).expect("practice");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].canvas, id.get());
    assert_eq!(
        SessionRecord {
            canvas: 0,
            ..stored[0].clone()
        },
        session
    );
    // The library lists every project; the gallery leaves the sheet out (the runtime's list).
    let listed = library.list().expect("list");
    let ids: Vec<_> = listed.iter().map(|item| item.id).collect();
    assert_eq!(
        ids,
        [id, artwork],
        "the sheet, touched last, and the artwork"
    );
    let opened = library.open_last().expect("opens").expect("the sheet");
    assert_eq!(opened.id, id);
    assert_eq!(opened.title, title);

    let mut checked = stored[0].clone();
    checked.current = 3;
    checked.finished = Some(1_791_202_400);
    checked.results.push(ExerciseResult::first(
        12,
        1_791_201_900,
        5,
        vec![(Metric::FirstDelta, 0.012_5), (Metric::FinalHue, -0.3)],
    ));
    save(&mut library, &checked).expect("saved");
    drop(library);
    let after = records(&dir);
    for record in &before {
        let (keyspace, key, _) = record;
        // Saving and opening change the change counter, `last` and the catalogue; every other
        // record before practice keeps its bytes.
        let counters = key == b"clock" || key == b"next" || key == b"last";
        let changing = keyspace == "library" && (counters || key[0] == b'a');
        if !changing {
            assert!(after.contains(record), "{record:?} changed");
        }
    }
    let mut reopened = Library::at(&dir);
    assert_eq!(sessions(&mut reopened).expect("practice"), vec![checked]);
}

/// A sheet's title (a colour copy's name) is its project's in the library's list, renamed as any
/// project's.
#[test]
fn a_sheet_s_title_is_its_project_s_and_renamed() {
    let dir = TestDir::new("practice-titles");
    let mut library = Library::at(&dir);
    let session = SessionRecord::stage_one(1, 1, 5, 1_791_201_600, &[]);
    let title = Title::new("Colour copy 1").expect("title");
    let id = start(&mut library, (&sheet(), &title), &session).expect("started");
    let titled = |library: &mut Library| {
        let listed = library.list().expect("list");
        listed
            .into_iter()
            .map(|item| (item.id, item.title))
            .collect::<Vec<_>>()
    };
    assert_eq!(titled(&mut library), [(id, title)]);
    let named = Title::new("Lamp study").expect("title");
    library.rename(id, &named).expect("renamed");
    assert_eq!(titled(&mut library), [(id, named)]);
}

#[test]
fn a_session_removed_takes_its_record_and_sheet_and_leaves_every_other_byte() {
    let dir = TestDir::new("practice-removed");
    let mut library = Library::at(&dir);
    library
        .create(
            &Artwork::new(CanvasSize::new(64, 48).expect("size")),
            &untitled(),
        )
        .expect("an artwork");
    // The author's session, part done, then the gallery.
    let mut theirs = SessionRecord::stage_one(1, 1, 0x5EED_0045, 1_791_201_600, &[]);
    let title = Title::new("Practice 1").expect("title");
    let sheet_id = start(&mut library, (&sheet(), &title), &theirs).expect("started");
    theirs.canvas = sheet_id.get();
    library.save(sheet_id, &sheet()).expect("their sheet saved");
    theirs.current = 3;
    theirs
        .results
        .push(ExerciseResult::first(0, 1_791_201_700, 2, Vec::new()));
    save(&mut library, &theirs).expect("their record");
    library.close().expect("the gallery");
    drop(library);
    let before = records(&dir);

    // A test session beside it, worked on, then removed.
    let mut library = Library::at(&dir);
    let read = sessions(&mut library).expect("practice");
    let mut test = SessionRecord::stage_one(2, 2, 77, 1_791_202_000, &met(&read));
    assert_eq!(
        test.warm_up, 4,
        "beside the author's, as their next would be"
    );
    let test_title = Title::new("Practice 2").expect("title");
    let test_id = start(&mut library, (&sheet(), &test_title), &test).expect("started");
    test.canvas = test_id.get();
    library
        .save(test_id, &sheet())
        .expect("the test sheet saved");
    test.current = 5;
    save(&mut library, &test).expect("the test record");
    library
        .keep_version(test_id, 1_791_202_100)
        .expect("a version, were one kept");
    remove(&mut library, &test).expect("removed");
    assert_eq!(
        sessions(&mut library).expect("practice"),
        vec![theirs.clone()]
    );
    assert!(library.load(test_id).is_err(), "its sheet is gone");
    drop(library);
    let after = records(&dir);
    // Only the change counter and the next project id moved on.
    let counter = |(keyspace, key, _): &&(String, Vec<u8>, Vec<u8>)| {
        keyspace == "library" && (key == b"clock" || key == b"next")
    };
    let kept = |all: &[(String, Vec<u8>, Vec<u8>)]| -> Vec<(String, Vec<u8>, Vec<u8>)> {
        all.iter().filter(|r| !counter(r)).cloned().collect()
    };
    assert_eq!(
        kept(&after),
        kept(&before),
        "every other record byte for byte"
    );
}

#[test]
fn a_practice_record_that_cannot_be_read_is_left_out() {
    let dir = TestDir::new("practice-damaged");
    let mut library = Library::at(&dir);
    let session = SessionRecord::stage_one(1, 1, 3, 0, &[]);
    let title = Title::new("Practice 1").expect("title");
    start(&mut library, (&sheet(), &title), &session).expect("started");
    drop(library);
    let mut key = b"p".to_vec();
    key.extend_from_slice(&2u64.to_be_bytes());
    write_records(&dir, &[("library".to_owned(), key, vec![1, 2, 3])]);
    let mut library = Library::at(&dir);
    let read = sessions(&mut library).expect("practice");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].id, 1);
    assert_eq!(
        library.list().map(|listed| listed.len()),
        Ok(1),
        "its sheet"
    );
}
