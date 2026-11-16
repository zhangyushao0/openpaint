//! Practice records in the library's record spaces (D112, S120): a sheet starts with its project
//! and its lesson's givens in one write, its record round-trips and is replaced whole (its givens
//! as they were written), its project (one its record owns, the runtime's gallery list leaves out)
//! opens as the last artwork, the records before practice keep their bytes, and a sheet removed
//! (the test-lesson chore's) takes its record, its givens and its project with it and leaves
//! every other byte; a record that cannot be read, or does not begin with a sheet's tag, is left
//! out; its givens read back whole or refuse it.

#[path = "../../store/tests/common/mod.rs"]
mod common;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{PortError, ProjectId, ProjectStore, Record, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{
    First, Given, Givens, Layout, Learner, Metric, Picked, Pictures, Planned, Purpose, Sheet,
    Skill, SkillLevel, Standing, schedule,
};
use openpaint_practice_store::{
    GIVENS, decode_givens, encode_givens, printed, remove, save, sheets, start,
};
use openpaint_store::Library;

fn artwork() -> Artwork {
    let [width, height] = Layout::of(&[None; 18]).size;
    Artwork::new(CanvasSize::new(width, height).expect("size"))
}

/// Three of unit 1's skills in one lesson, the `id`th sheet, seeded `seed`, after the lessons of
/// `before`: a line, a group and a colour, then a lesson of each.
fn mixed_lesson(id: u64, seed: u64, started: u64, before: &[Sheet]) -> Sheet {
    let standing = Standing {
        pictures: &Pictures::default(),
        learner: &Learner::of(before),
    };
    let lessons: Vec<Planned> = (["line", "combined", "colour"].iter())
        .map(|key| {
            let skill = Skill::of_key(key).expect("a skill of unit 1");
            schedule::lesson(skill, SkillLevel::First, seed, &standing).expect("its lesson")
        })
        .collect();
    let firsts = lessons.iter().filter_map(|lesson| lesson.exercises.first());
    let parts = lessons.iter().flat_map(|lesson| lesson.exercises.iter());
    let planned = Planned {
        picked: Picked::Chosen,
        exercises: firsts.chain(parts).cloned().collect(),
    };
    Sheet::begun(id, (Purpose::Day, started), planned)
}

/// What `sheet`'s lesson printed, as it starts with.
fn givens(sheet: &Sheet) -> Givens {
    Givens::of_lesson(sheet.lesson().expect("a lesson"), &[])
}

/// The givens stored for project `id`: one record, of its first lesson, what `sheet` printed.
fn printed_once(library: &mut Library, id: ProjectId, sheet: &Sheet) -> Vec<Record> {
    let printed = (library.records(GIVENS, &id.get().to_be_bytes())).expect("givens");
    let mut key = vec![b'g'];
    key.extend_from_slice(&id.get().to_be_bytes());
    key.extend_from_slice(&0u16.to_be_bytes());
    assert_eq!(printed.len(), 1, "one lesson's");
    assert_eq!(printed[0].0, key);
    assert_eq!(decode_givens(&printed[0].1), Ok(givens(sheet)));
    printed
}

/// `all` without the change counter and the next project id.
fn without_counters(all: &[(String, Vec<u8>, Vec<u8>)]) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let counter = |(keyspace, key, _): &&(String, Vec<u8>, Vec<u8>)| {
        keyspace == "library" && (key == b"clock" || key == b"next")
    };
    all.iter().filter(|r| !counter(r)).cloned().collect()
}

#[test]
fn a_sheet_round_trips_with_its_project_and_its_givens() {
    let dir = TestDir::new("practice");
    let mut library = Library::at(&dir);
    let artwork_id = library
        .create(
            &Artwork::new(CanvasSize::new(64, 48).expect("size")),
            &untitled(),
        )
        .expect("an artwork");
    drop(library);
    let before = records(&dir);
    let mut library = Library::at(&dir);
    let sheet = mixed_lesson(1, 0x5EED_0045, 1_791_201_600, &[]);
    let title = Title::new("Practice 1").expect("title");
    let id = start(&mut library, (&artwork(), &title), &sheet, &givens(&sheet)).expect("started");
    assert_ne!(id, artwork_id);
    let stored = sheets(&mut library).expect("practice");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].canvas, id.get());
    assert_eq!(
        Sheet {
            canvas: 0,
            ..stored[0].clone()
        },
        sheet
    );
    let printed = printed_once(&mut library, id, &sheet);
    // The library lists every project; the gallery leaves the sheet out (the runtime's list).
    let listed = library.list().expect("list");
    let ids: Vec<_> = listed.iter().map(|item| item.id).collect();
    assert_eq!(
        ids,
        [id, artwork_id],
        "the sheet, touched last, and the artwork"
    );
    let opened = library.open_last().expect("opens").expect("the sheet");
    assert_eq!(opened.id, id);
    assert_eq!(opened.title, title);

    let mut checked = stored[0].clone();
    let lesson = checked.lesson_mut().expect("its lesson");
    lesson.at = 3;
    lesson.finished = Some(1_791_202_400);
    let metrics = vec![(Metric::FirstDelta, 0.012_5), (Metric::FinalHue, -0.3)];
    let laid = checked.laid_mut(12).expect("a colour");
    laid.compared(First::of(1_791_201_900, 5, metrics));
    laid.seconds = 41;
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
    assert_eq!(sheets(&mut reopened).expect("practice"), vec![checked]);
    let kept = reopened.records(GIVENS, &[]).expect("givens");
    assert_eq!(kept, printed, "a save writes only the sheet's record");
}

/// A sheet's title (a colour copy's name) is its project's in the library's list, renamed as any
/// project's.
#[test]
fn a_sheet_s_title_is_its_project_s_and_renamed() {
    let dir = TestDir::new("practice-titles");
    let mut library = Library::at(&dir);
    let sheet = mixed_lesson(1, 5, 1_791_201_600, &[]);
    let title = Title::new("Colour copy 1").expect("title");
    let id = start(&mut library, (&artwork(), &title), &sheet, &givens(&sheet)).expect("started");
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
fn a_sheet_removed_takes_its_record_givens_and_project_and_leaves_every_other_byte() {
    let dir = TestDir::new("practice-removed");
    let mut library = Library::at(&dir);
    library
        .create(
            &Artwork::new(CanvasSize::new(64, 48).expect("size")),
            &untitled(),
        )
        .expect("an artwork");
    // The author's lesson, part done, then the home.
    let mut theirs = mixed_lesson(1, 0x5EED_0045, 1_791_201_600, &[]);
    let title = Title::new("Practice 1").expect("title");
    let printed = givens(&theirs);
    let sheet_id = start(&mut library, (&artwork(), &title), &theirs, &printed).expect("started");
    theirs.canvas = sheet_id.get();
    library
        .save(sheet_id, &artwork())
        .expect("their sheet saved");
    theirs.lesson_mut().expect("its lesson").at = 3;
    let first = First::of(1_791_201_700, 2, Vec::new());
    theirs.laid_mut(0).expect("a line").compared(first);
    save(&mut library, &theirs).expect("their record");
    library.close().expect("the gallery");
    drop(library);
    let before = records(&dir);

    // A test lesson beside it, worked on, then removed.
    let mut library = Library::at(&dir);
    let read = sheets(&mut library).expect("practice");
    let mut test = mixed_lesson(2, 77, 1_791_202_000, &read);
    let test_title = Title::new("Practice 2").expect("title");
    let test_givens = givens(&test);
    let test_id =
        start(&mut library, (&artwork(), &test_title), &test, &test_givens).expect("started");
    test.canvas = test_id.get();
    library
        .save(test_id, &artwork())
        .expect("the test sheet saved");
    test.lesson_mut().expect("its lesson").at = 5;
    save(&mut library, &test).expect("the test record");
    library
        .keep_version(test_id, 1_791_202_100)
        .expect("a version, were one kept");
    assert_eq!(library.records(GIVENS, &[]).map(|g| g.len()), Ok(2));
    remove(&mut library, &test).expect("removed");
    assert_eq!(
        sheets(&mut library).expect("practice"),
        vec![theirs.clone()]
    );
    assert!(library.load(test_id).is_err(), "its sheet is gone");
    drop(library);
    let after = records(&dir);
    // Only the change counter and the next project id moved on.
    assert_eq!(
        without_counters(&after),
        without_counters(&before),
        "every other record byte for byte"
    );
}

/// A sheet's givens read back as it printed them (S120d); missing, damaged or short of an
/// exercise, the sheet is refused, never remade.
#[test]
fn a_sheet_s_givens_read_back_as_printed_and_refuse_it_when_not_whole() {
    let dir = TestDir::new("practice-printed");
    let mut library = Library::at(&dir);
    let sheet = mixed_lesson(1, 0x5EED_0120, 1_791_201_600, &[]);
    let title = Title::new("Practice 1").expect("title");
    let id = start(&mut library, (&artwork(), &title), &sheet, &givens(&sheet)).expect("started");
    let stored = sheets(&mut library).expect("practice").remove(0);
    let read = printed(&mut library, &stored).expect("its givens");
    assert_eq!(
        read,
        Given::of_sheet(&sheet, &[]),
        "as the generators print now"
    );
    let mut key = vec![b'g'];
    key.extend_from_slice(&id.get().to_be_bytes());
    key.extend_from_slice(&0u16.to_be_bytes());
    let corrupt = Err(PortError::Corrupt {
        what: "practice givens",
    });
    let mut short = givens(&sheet);
    short.slots.pop();
    library
        .put_records(&[(GIVENS, (key.clone(), encode_givens(&short)))])
        .expect("an exercise's left out");
    assert_eq!(printed(&mut library, &stored), corrupt, "short of one");
    library
        .put_records(&[(GIVENS, (key.clone(), vec![1, 0, 0]))])
        .expect("damaged");
    assert!(printed(&mut library, &stored).is_err(), "damaged");
    library
        .remove_owned(&[(GIVENS, key)], &[])
        .expect("removed");
    assert_eq!(printed(&mut library, &stored), corrupt, "missing");
}

#[test]
fn a_practice_record_that_cannot_be_read_is_left_out() {
    let dir = TestDir::new("practice-damaged");
    let mut library = Library::at(&dir);
    let sheet = mixed_lesson(1, 3, 0, &[]);
    let title = Title::new("Practice 1").expect("title");
    start(&mut library, (&artwork(), &title), &sheet, &givens(&sheet)).expect("started");
    drop(library);
    let key = |id: u64| [b"p".as_slice(), &id.to_be_bytes()].concat();
    // A sheet's tag with too few bytes after it, and a record beginning otherwise: neither is read.
    write_records(
        &dir,
        &[
            ("library".to_owned(), key(2), vec![0x53, 2, 3]),
            ("library".to_owned(), key(3), vec![2, 1, 0, 0, 0]),
        ],
    );
    let mut library = Library::at(&dir);
    let read = sheets(&mut library).expect("practice");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].id, 1);
    assert_eq!(
        library.list().map(|listed| listed.len()),
        Ok(1),
        "its sheet"
    );
}
