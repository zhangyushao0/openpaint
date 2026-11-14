//! A lesson appended to a stored sheet (S121b) and cut by the Pomodoro's End (S124): the sheet's
//! record and the lesson's givens in one write; the sheet reads back whole, its givens resolving
//! every exercise; a write refused leaves the sheet and its givens as they were.

mod support;
#[path = "../../store/tests/common/test_dir.rs"]
mod test_dir;

use openpaint_app::{PortError, ProjectStore, Record, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{
    Exercise, Given, Givens, Kind, Layout, LineTier, Picked, Planned, Purpose, Sheet, SkillLevel,
    schedule,
};
use openpaint_practice_store::{GIVENS, append, decode_givens, printed, sheets, start, trim};
use openpaint_store::Library;
use support::Batches;
use test_dir::TestDir;

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A lesson of `n` single lines begun at `started`, the `id`th sheet's.
fn lines(id: u64, n: u64, started: u64) -> Sheet {
    let exercises = (0..n)
        .map(|seed| {
            let line = Exercise::generated(Kind::Line(LineTier::Arc), seed);
            (line, SkillLevel::First)
        })
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    Sheet::begun(id, (Purpose::Day, started), planned)
}

/// A lesson of four single lines started in `library` (finished), as stored.
fn started(library: &mut dyn ProjectStore) -> Sheet {
    let mut lesson = lines(1, 4, NOON);
    let [width, height] = Layout::of_sheet(&lesson).size;
    let artwork = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let givens = Givens::of_lesson(&lesson.lessons[0], &[]);
    let title = Title::new("Lesson 1").expect("title");
    let id = start(library, (&artwork, &title), &lesson, &givens).expect("started");
    lesson.canvas = id.get();
    lesson.lessons[0].finished = Some(NOON + 300);
    lesson
}

/// `sheet` with `lesson`, the first of a sheet, appended, and what it prints.
fn appended(mut sheet: Sheet, lesson: Sheet) -> (Sheet, Givens) {
    let lesson = lesson.lessons[0].clone();
    let givens = Givens::of_lesson(&lesson, &[]);
    let givens = sheet.append(lesson, givens);
    (sheet, givens)
}

/// The first lesson, to append.
fn first() -> Sheet {
    Sheet::begun(0, (Purpose::Day, NOON + 600), schedule::first(5))
}

#[test]
fn a_sheet_of_two_lessons_round_trips_its_givens_resolving_every_exercise() {
    let dir = TestDir::new("practice-append");
    let mut store = Batches::of(Library::at(&dir));
    let (sheet, givens) = appended(started(&mut store), first());
    append(&mut store, &sheet, &givens).expect("appended");
    assert_eq!(store.writes, [2], "the record and its givens in one write");
    drop(store);
    let mut library = Library::at(&dir);
    let read = sheets(&mut library).expect("practice");
    assert_eq!(read, std::slice::from_ref(&sheet));
    assert_eq!(library.records(GIVENS, &[]).map(|g| g.len()), Ok(2));
    let printed = printed(&mut library, &read[0]).expect("every exercise's");
    assert_eq!(printed, Given::of_sheet(&sheet, &[]));
    let layout = Layout::of_sheet(&read[0]);
    assert_eq!(
        layout.pairs[4],
        Layout::pair_at(6, None),
        "the second lesson on a new row"
    );
}

#[test]
fn an_append_refused_leaves_the_sheet_and_its_givens_as_they_were() {
    let dir = TestDir::new("practice-append-refused");
    let mut store = Batches::of(Library::at(&dir));
    store.refused = |(space, _)| *space == GIVENS;
    let sheet = started(&mut store);
    let before = sheets(&mut store).expect("practice");
    let (longer, givens) = appended(sheet, first());
    assert!(append(&mut store, &longer, &givens).is_err());
    assert!(store.writes.is_empty());
    assert_eq!(
        sheets(&mut store).expect("practice"),
        before,
        "its record as it was"
    );
    assert_eq!(store.records(GIVENS, &[]).map(|g| g.len()), Ok(1));
    assert!(
        printed(&mut store, &before[0]).is_ok(),
        "its one lesson's givens"
    );
}

/// A stored sheet of four lines, then six more appended from its third row: the sheet and every
/// givens record as stored.
fn two_lessons(store: &mut Batches) -> (Sheet, Vec<Record>) {
    let (sheet, givens) = appended(started(store), lines(0, 6, NOON + 600));
    append(store, &sheet, &givens).expect("appended");
    store.writes.clear();
    let kept = store.records(GIVENS, &[]).expect("givens");
    (sheet, kept)
}

/// `sheet` as End leaves it at the exercise `at` of its last lesson: those after it gone, the
/// lesson finished.
fn cut(sheet: &Sheet, at: usize) -> Sheet {
    let mut cut = sheet.clone();
    let lesson = cut.lessons.last_mut().expect("a lesson");
    lesson.at = u16::try_from(at).expect("at");
    lesson.exercises.truncate(at + 1);
    lesson.finished = Some(NOON + 1500);
    cut
}

#[test]
fn end_mid_lesson_shortens_the_record_and_its_givens_in_one_write_freeing_the_slots() {
    let dir = TestDir::new("practice-trim");
    let mut store = Batches::of(Library::at(&dir));
    let (sheet, kept) = two_lessons(&mut store);
    assert_eq!(Layout::next_slot(&sheet), 12, "rows 1 to 4 used");
    let ended = cut(&sheet, 1);
    trim(&mut store, &ended).expect("trimmed");
    assert_eq!(store.writes, [2], "the record and its givens in one write");
    let read = sheets(&mut store).expect("practice");
    assert_eq!(read, std::slice::from_ref(&ended));
    assert_eq!(Layout::next_slot(&read[0]), 9, "the fourth row free again");
    let givens = store.records(GIVENS, &[]).expect("givens");
    assert_eq!(givens[0], kept[0], "the first lesson's givens as they were");
    let last = decode_givens(&givens[1].1).expect("the last lesson's");
    assert_eq!(last, Givens::of_lesson(&ended.lessons[1], &[]));
    assert_eq!(
        last.slots.iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
        [6, 7]
    );
    let printed = printed(&mut store, &read[0]).expect("every exercise's");
    assert_eq!(printed, Given::of_sheet(&ended, &[]));
}

#[test]
fn end_on_a_lesson_s_last_exercise_removes_nothing() {
    let dir = TestDir::new("practice-trim-last");
    let mut store = Batches::of(Library::at(&dir));
    let (sheet, kept) = two_lessons(&mut store);
    let ended = cut(&sheet, 5);
    assert_eq!(ended.lessons[1].exercises.len(), 6);
    trim(&mut store, &ended).expect("trimmed");
    assert_eq!(sheets(&mut store).expect("practice"), [ended]);
    assert_eq!(store.records(GIVENS, &[]).expect("givens"), kept);
}

#[test]
fn end_refused_leaves_the_record_and_its_givens_as_they_were() {
    let dir = TestDir::new("practice-trim-refused");
    let mut store = Batches::of(Library::at(&dir));
    let (sheet, kept) = two_lessons(&mut store);
    store.refused = |(space, _)| *space == GIVENS;
    assert!(trim(&mut store, &cut(&sheet, 1)).is_err());
    let stored = sheets(&mut store).expect("practice");
    assert_eq!(stored, std::slice::from_ref(&sheet));
    assert_eq!(store.records(GIVENS, &[]).expect("givens"), kept);
    // A sheet whose last lesson's givens are not kept is refused before any write.
    store.refused = |_| false;
    let mut other = cut(&sheet, 1);
    other.canvas += 1;
    let corrupt = PortError::Corrupt {
        what: "practice givens",
    };
    assert_eq!(trim(&mut store, &other), Err(corrupt));
    assert!(store.writes.is_empty());
    assert_eq!(sheets(&mut store).expect("practice"), stored);
}
