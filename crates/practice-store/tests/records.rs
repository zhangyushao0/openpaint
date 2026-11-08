//! Practice records in the library's record space (D112): a session starts with its sheet in one
//! write, its record round-trips and is replaced whole, its sheet (a project its record owns, the
//! runtime's gallery list leaves out) opens as the last artwork, the records before practice keep
//! their bytes, and a session removed (the test-session chore's) takes its record and sheet with
//! it and leaves every other byte. Old records are upgraded once when the library opens through
//! `library_at`.

#[path = "../../store/tests/common/mod.rs"]
mod common;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{ExerciseResult, Metric, SessionRecord, Sheet, met};
use openpaint_practice_store::{library_at, remove, save, sessions, start};

fn sheet() -> Artwork {
    let [width, height] = Sheet::of(&[None; 18]).size;
    Artwork::new(CanvasSize::new(width, height).expect("size"))
}

#[test]
fn a_session_round_trips_with_its_sheet_a_project_its_record_owns() {
    let dir = TestDir::new("practice");
    let mut library = library_at(&dir, None);
    let artwork = library
        .create(
            &Artwork::new(CanvasSize::new(64, 48).expect("size")),
            &untitled(),
        )
        .expect("an artwork");
    drop(library);
    let before = records(&dir);
    let mut library = library_at(&dir, None);
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
    let mut reopened = library_at(&dir, None);
    assert_eq!(sessions(&mut reopened).expect("practice"), vec![checked]);
}

/// A sheet's title (a colour copy's name) is its project's in the library's list, renamed as any
/// project's.
#[test]
fn a_sheet_s_title_is_its_project_s_and_renamed() {
    let dir = TestDir::new("practice-titles");
    let mut library = library_at(&dir, None);
    let session = SessionRecord::stage_one(1, 1, 5, 1_791_201_600, &[]);
    let title = Title::new("Colour copy 1").expect("title");
    let id = start(&mut library, (&sheet(), &title), &session).expect("started");
    let titled = |library: &mut openpaint_store::Library| {
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
    let mut library = library_at(&dir, None);
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
    let mut library = library_at(&dir, None);
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

/// A record as 1.6.2 wrote it (no pictures, no time: it ended after its warm-up's count) is given
/// its time from its compares once, when the library opens, and every other record keeps its
/// bytes; the sheet is read for the practice book without anything written.
#[test]
fn a_record_of_1_6_2_is_timed_by_its_compares_once_when_the_library_opens() {
    let dir = TestDir::new("practice-upgrade");
    let mut library = library_at(&dir, None);
    let mut old = SessionRecord::stage_one(1, 1, 0x5EED_0045, 1_791_201_600, &[]);
    let title = Title::new("Practice 1").expect("title");
    let id = start(&mut library, (&sheet(), &title), &old).expect("started");
    old.canvas = id.get();
    old.results
        .push(ExerciseResult::first(0, 1_791_201_700, 2, Vec::new()));
    old.results.push(ExerciseResult::first(
        1,
        1_791_201_700 + 7200,
        4,
        Vec::new(),
    ));
    save(&mut library, &old).expect("saved");
    let timed = SessionRecord::stage_one(2, 2, 3, 1_791_300_000, &[]);
    let second = Title::new("Practice 2").expect("title");
    start(&mut library, (&sheet(), &second), &timed).expect("started");
    drop(library);
    let key = |id: u64| [b"p".as_slice(), &id.to_be_bytes()].concat();
    let mut stored = records(&dir);
    let at = (stored.iter())
        .position(|(_, k, _)| *k == key(1))
        .expect("the record");
    // As 1.6.2 wrote it: without the pictures' count and the spans' (both 0 here), nor the
    // layout 1.17.0 ends a record with (S99).
    let length = stored[at].2.len();
    assert_eq!(stored[at].2[length - 9..], [0, 0, 0, 0, 0, 0, 0, 0, 1]);
    stored[at].2.truncate(length - 9);
    write_records(&dir, std::slice::from_ref(&stored[at]));
    let before = records(&dir);

    let mut library = library_at(&dir, None);
    let read = sessions(&mut library).expect("practice");
    assert_eq!(read[0].time, old.time_of_compares());
    assert_eq!(
        openpaint_practice::seconds(&read[0].time),
        100,
        "two hours between the compares count nothing"
    );
    assert_eq!(read[1].time, [], "written by 1.7.0: as it was");
    assert_eq!(library.artwork(id), Ok(sheet()));
    drop(library);
    let after = records(&dir);
    for record in &before {
        if record.1 != key(1) {
            assert!(after.contains(record), "{record:?} changed");
        }
    }
    let mut again = library_at(&dir, None);
    assert_eq!(sessions(&mut again).expect("practice"), read);
    drop(again);
    assert_eq!(records(&dir), after, "upgraded once");
}

#[test]
fn a_practice_record_that_cannot_be_read_is_left_out() {
    let dir = TestDir::new("practice-damaged");
    let mut library = library_at(&dir, None);
    let session = SessionRecord::stage_one(1, 1, 3, 0, &[]);
    let title = Title::new("Practice 1").expect("title");
    start(&mut library, (&sheet(), &title), &session).expect("started");
    drop(library);
    let mut key = b"p".to_vec();
    key.extend_from_slice(&2u64.to_be_bytes());
    write_records(&dir, &[("library".to_owned(), key, vec![1, 2, 3])]);
    let mut library = library_at(&dir, None);
    let read = sessions(&mut library).expect("practice");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].id, 1);
    assert_eq!(
        library.list().map(|listed| listed.len()),
        Ok(1),
        "its sheet"
    );
}
