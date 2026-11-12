//! The test-lesson chore (`cargo device-input practice-test start|end`) through the whole frame
//! loop on a real library: a test lesson started from the home beside the author's open one,
//! drawn on and compared, then removed. The author's record, their sheet (as the bytes of its
//! OpenPaint file) and the path are as before; the store's own test holds every other record to
//! its bytes. Motion runs on the hand's held clock, 16 ms a frame, as in scenarios.

use openpaint_app::{ProjectId, ProjectStore};
use openpaint_practice::{Purpose, Skill, SkillLevel};
use openpaint_practice_app::PathProjection;
use openpaint_practice_store::sheets;
use openpaint_runtime::{Runtime, Start, ToolKind, skill_start};
use openpaint_store::Library;

use crate::support::{Hand, TestDir, gallery, mapping, start, until};

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// A runtime on the library in `dir` and the hand on it, on the home: no artwork is listed, so
/// the app starts on a blank canvas, left for the home.
fn on_the_home(dir: &std::path::Path) -> (Runtime, Hand) {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(dir)));
    assert!(runtime.back(), "to the home");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    (runtime, hand)
}

fn path(runtime: &Runtime) -> PathProjection {
    runtime.path().expect("the path")
}

/// A line across the shown exercise's empty frame, Compare, and back to the home.
fn copy_and_compare(runtime: &mut Runtime, hand: &mut Hand) {
    copy(runtime, hand);
    assert!(runtime.back(), "back to the home");
    until(runtime, hand, |r| gallery(r).is_some());
}

/// A line across the shown exercise's empty frame, and Compare.
fn copy(runtime: &mut Runtime, hand: &mut Hand) {
    let practice = runtime.practice().expect("a practice sheet");
    let frame = practice.sheet.pairs[practice.current.expect("an exercise")].copy;
    let view = mapping(runtime, SIZE, PIXELS_PER_POINT);
    let at = |p: [f64; 2]| view.to_screen(frame.to_canvas(p).map(|v| v as f32));
    hand.gesture(
        runtime,
        ToolKind::Stylus,
        &[at([0.3, 0.3]), at([0.5, 0.5]), at([0.7, 0.6])],
    );
    hand.tap(runtime, "Compare");
    assert!(runtime.practice().is_some_and(|p| p.comparing));
}

/// The skill of the lesson the open sheet shows, and its number.
fn in_hand(runtime: &Runtime) -> Option<(Option<Skill>, u32)> {
    let lesson = runtime.practice()?.lesson?;
    Some((lesson.row.skill, lesson.row.number))
}

#[test]
fn a_test_lesson_beside_the_authors_leaves_theirs_as_it_was() {
    let dir = TestDir::new("practice-test");
    // The author's lesson of lines: a line copied and compared, the lesson left open.
    let (mut runtime, mut hand) = on_the_home(&dir);
    let lines = Skill::of_key("line").expect("lines");
    let theirs = Start::At(lines, SkillLevel::First);
    runtime.start_lesson(theirs).expect("their lesson");
    until(&mut runtime, &mut hand, |r| r.practice().is_some());
    hand.tap(&mut runtime, "Got it");
    copy_and_compare(&mut runtime, &mut hand);
    let theirs = path(&runtime);
    let sheet = theirs.open.expect("their lesson open").sheet;
    drop(runtime);
    let mut library = Library::at(&dir);
    let records = sheets(&mut library).expect("their records");
    let file = library.artwork_file(sheet).expect("their sheet");
    drop(library);

    // The chore: a test lesson beside theirs, worked on, then removed. The home never lists
    // their sheet (the runtime's list, D112).
    let (mut runtime, mut hand) = on_the_home(&dir);
    assert_eq!(path(&runtime), theirs);
    let listed = gallery(&runtime).map(|g| g.items().iter().any(|item| item.id == sheet));
    assert_eq!(listed, Some(false), "their sheet is no artwork");
    let id = (runtime.start_test_lesson(Start::Next)).expect("started");
    until(&mut runtime, &mut hand, |r| r.test_practice(id).is_some());
    let test = runtime.test_practice(id).expect("its name");
    assert_ne!(test.sheet, sheet.get(), "a sheet of its own");
    let next = theirs.next.map(|offer| offer.skill);
    assert_eq!(
        in_hand(&runtime).map(|(skill, _)| skill),
        Some(next),
        "it starts as their next lesson would"
    );
    copy_and_compare(&mut runtime, &mut hand);
    assert_eq!(
        path(&runtime).open.map(|open| open.sheet),
        Some(sheet),
        "a test's sheet is never the day's"
    );
    let named = test.to_string().parse().expect("its name read back");
    runtime.end_test_practice(&named).expect("ended");
    until(&mut runtime, &mut hand, |r| !r.holds_practice(id));
    assert_eq!(path(&runtime), theirs, "the path as before");
    let mut other = test;
    other.started ^= 1;
    let refused = runtime.end_test_practice(&other);
    assert!(refused.is_err(), "nothing else is removed");
    drop(runtime);
    let mut library = Library::at(&dir);
    assert_eq!(
        sheets(&mut library).expect("records"),
        records,
        "their records"
    );
    assert_eq!(library.artwork_file(sheet).expect("their sheet"), file);
    assert!(library.load(ProjectId::new(test.sheet)).is_err());
}

#[test]
fn a_test_lesson_of_a_skill_at_a_level_is_kept_apart_and_one_more_follows_on_its_sheet() {
    let dir = TestDir::new("practice-test-lesson");
    let (mut runtime, mut hand) = on_the_home(&dir);
    let before = path(&runtime);
    assert!(skill_start("middle", None).is_err(), "no such skill");
    let points = skill_start("points-in-space", Some(1)).expect("a skill");
    let id = (runtime.start_test_lesson(points)).expect("started");
    until(&mut runtime, &mut hand, |r| r.test_practice(id).is_some());
    let skill = Skill::of_key("points-in-space");
    assert_eq!(in_hand(&runtime), Some((skill, 1)));
    let test = runtime.test_practice(id).expect("its name");
    let shown = runtime.practice().expect("a sheet");
    assert!(
        shown.how.is_some() && shown.how == shown.kind,
        "its How first"
    );
    // Its How closes by its word (on this small screen Got it lies below the card's room).
    hand.tap(&mut runtime, "How");
    copy(&mut runtime, &mut hand);
    // On to its end: One more lays the next lesson on the same sheet.
    while runtime.controls().find("Finish").is_err() {
        hand.tap(&mut runtime, "Next");
    }
    for word in ["Finish", "One more"] {
        hand.tap(&mut runtime, word);
    }
    until(&mut runtime, &mut hand, |r| {
        in_hand(r).is_some_and(|(_, number)| number == 2)
    });
    // What its end named: a skill of a unit ahead of the learner's again, not the first lesson.
    assert_eq!(
        in_hand(&runtime),
        Some((skill, 2)),
        "One more lays what its card names"
    );
    assert!(runtime.back(), "back to the home");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    assert_eq!(
        path(&runtime).open.map(|open| open.sheet),
        None,
        "a test's sheet is never the day's"
    );
    // Stored as a test's sheet (S121b); One more's lesson on it, in place (S121c).
    drop(runtime);
    let mut library = Library::at(&dir);
    let purposes: Vec<(u64, Purpose, usize)> = (sheets(&mut library).expect("records").iter())
        .map(|sheet| (sheet.id, sheet.purpose, sheet.lessons.len()))
        .collect();
    assert_eq!(purposes, [(id, Purpose::Test, 2)]);
    drop(library);
    let (mut runtime, mut hand) = on_the_home(&dir);
    runtime.end_test_practice(&test).expect("ended");
    until(&mut runtime, &mut hand, |r| !r.holds_practice(id));
    assert_eq!(path(&runtime).units, before.units, "no dot from it");
}

#[test]
fn the_next_lesson_while_one_is_under_way_opens_it() {
    let dir = TestDir::new("practice-under-way");
    let (mut runtime, mut hand) = on_the_home(&dir);
    runtime.start_lesson(Start::Next).expect("the first lesson");
    until(&mut runtime, &mut hand, |r| in_hand(r).is_some());
    assert!(runtime.back(), "to the home");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    let open = path(&runtime).open.expect("under way");
    assert_eq!((open.place, open.row.first), ((1, 3), true));
    // As Continue does: its sheet opens on it, nothing new is laid.
    runtime.start_lesson(Start::Next).expect("continued");
    until(&mut runtime, &mut hand, |r| in_hand(r).is_some());
    assert_eq!(in_hand(&runtime), Some((None, 1)));
    drop(runtime);
    let mut library = Library::at(&dir);
    let lessons: Vec<usize> = (sheets(&mut library).expect("records").iter())
        .map(|sheet| sheet.lessons.len())
        .collect();
    assert_eq!(lessons, [1]);
}
