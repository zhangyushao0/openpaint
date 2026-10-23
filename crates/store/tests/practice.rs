//! Practice records in the library: a session starts with its sheet in one write, its record
//! round-trips and is replaced whole, its sheet opens as the last artwork but the gallery never
//! lists it, and the records before practice keep their bytes.

mod common;

use common::{fresh_dir, records, untitled, write_records};
use openpaint_app::{ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{ExerciseResult, Metric, SessionRecord, Sheet};
use openpaint_store::Library;

fn sheet() -> Artwork {
    let [width, height] = Sheet::of(18).size;
    Artwork::new(CanvasSize::new(width, height).expect("size"))
}

#[test]
fn a_session_round_trips_with_its_sheet_hidden_from_the_gallery() {
    let dir = fresh_dir("practice");
    let mut library = Library::at(&dir);
    let artwork = library
        .create(
            &Artwork::new(CanvasSize::new(64, 48).expect("size")),
            &untitled(),
        )
        .expect("an artwork");
    let before = records(&{
        drop(library);
        dir.clone()
    });
    let mut library = Library::at(&dir);
    let session = SessionRecord::stage_one(1, 1, 0x5EED_0045, 1_791_201_600);
    let title = Title::new("Practice 1").expect("title");
    let id = library
        .start_practice(&sheet(), &title, &session)
        .expect("started");
    assert_ne!(id, artwork);
    let stored = library.practice().expect("practice");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].canvas, id.get());
    assert_eq!(
        SessionRecord {
            canvas: 0,
            ..stored[0].clone()
        },
        session
    );
    let listed = library.list().expect("list");
    assert_eq!(listed.len(), 1, "the sheet is not listed");
    assert_eq!(listed[0].id, artwork);
    let opened = library.open_last().expect("opens").expect("the sheet");
    assert_eq!(opened.id, id);
    assert_eq!(opened.title, title);

    let mut checked = stored[0].clone();
    checked.current = 3;
    checked.finished = Some(1_791_202_400);
    checked.results.push(ExerciseResult {
        index: 2,
        checked_at: 1_791_201_900,
        metrics: vec![(Metric::KeyPoints, 0.012_5), (Metric::Shape, 0.3)],
    });
    library.save_practice(&checked).expect("saved");
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
    assert_eq!(reopened.practice().expect("practice"), vec![checked]);
}

#[test]
fn a_practice_record_that_cannot_be_read_is_left_out() {
    let dir = fresh_dir("practice-damaged");
    let mut library = Library::at(&dir);
    let session = SessionRecord::stage_one(1, 1, 3, 0);
    let title = Title::new("Practice 1").expect("title");
    library
        .start_practice(&sheet(), &title, &session)
        .expect("started");
    drop(library);
    let mut key = b"p".to_vec();
    key.extend_from_slice(&2u64.to_be_bytes());
    write_records(&dir, &[("library".to_owned(), key, vec![1, 2, 3])]);
    let mut library = Library::at(&dir);
    let sessions = library.practice().expect("practice");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, 1);
    assert!(library.list().expect("list").is_empty());
}
