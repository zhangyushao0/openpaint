//! The test-session chore (`cargo device-input practice-test start|end`) through the whole frame
//! loop on a real library: a test session started from the gallery beside the author's open one,
//! drawn on and compared, then removed. The author's session record, their sheet (as the bytes of
//! its OpenPaint file) and the Today card are as before; the store's own test holds every other
//! record to its bytes. Motion runs on the hand's held clock, 16 ms a frame, as in scenarios.

use openpaint_app::{ProjectId, ProjectStore};
use openpaint_practice::{Family, Purpose};
use openpaint_practice_app::TodayProjection;
use openpaint_practice_store::sheets;
use openpaint_runtime::{Runtime, ToolKind};
use openpaint_store::Library;

use crate::support::{Hand, TestDir, gallery, mapping, start, until};

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// A runtime on the library in `dir` and the hand on it, in the gallery: no artwork is listed,
/// so the app starts on a blank canvas, left for the gallery.
fn in_the_gallery(dir: &std::path::Path) -> (Runtime, Hand) {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(dir)));
    assert!(runtime.back(), "to the gallery");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    (runtime, hand)
}

fn today(runtime: &Runtime) -> TodayProjection {
    runtime.today().expect("the Today card")
}

/// A line across the shown exercise's empty frame, Compare, and back to the gallery.
fn copy_and_compare(runtime: &mut Runtime, hand: &mut Hand) {
    copy(runtime, hand);
    assert!(runtime.back(), "back to the gallery");
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

#[test]
fn a_test_session_beside_the_authors_leaves_theirs_as_it_was() {
    let dir = TestDir::new("practice-test");
    // The author's session: a line copied and compared, the session left open.
    let (mut runtime, mut hand) = in_the_gallery(&dir);
    hand.tap(&mut runtime, "Start practice");
    until(&mut runtime, &mut hand, |r| r.practice().is_some());
    hand.tap(&mut runtime, "Got it");
    copy_and_compare(&mut runtime, &mut hand);
    let theirs = today(&runtime);
    let sheet = theirs.open.expect("their session open");
    drop(runtime);
    let mut library = Library::at(&dir);
    let records = sheets(&mut library).expect("their records");
    let file = library.artwork_file(sheet).expect("their sheet");
    drop(library);

    // The chore: a test session beside theirs, worked on, then removed. The gallery never lists
    // their sheet (the runtime's gallery list, D112).
    let (mut runtime, mut hand) = in_the_gallery(&dir);
    assert_eq!(today(&runtime), theirs);
    let listed = gallery(&runtime).map(|g| g.items().iter().any(|item| item.id == sheet));
    assert_eq!(listed, Some(false), "their sheet is no artwork");
    let id = (runtime.start_test_practice(None, None)).expect("started");
    until(&mut runtime, &mut hand, |r| r.test_practice(id).is_some());
    let test = runtime.test_practice(id).expect("its name");
    assert_ne!(test.sheet, sheet.get(), "a sheet of its own");
    let practice = runtime.practice().expect("the test sheet");
    assert!(
        practice.warming_up,
        "it starts as their next session would, warming up"
    );
    copy_and_compare(&mut runtime, &mut hand);
    assert_eq!(today(&runtime).open.map(ProjectId::get), Some(test.sheet));
    let named = test.to_string().parse().expect("its name read back");
    runtime.end_test_practice(&named).expect("ended");
    until(&mut runtime, &mut hand, |r| !r.holds_practice(id));
    assert_eq!(today(&runtime), theirs, "the Today card as before");
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

/// The kind a drill's sheet drills and its number, while one shows.
fn drilled(runtime: &Runtime) -> Option<(Family, u32)> {
    let practice = runtime.practice()?;
    Some((practice.drill?, practice.number))
}

#[test]
fn a_test_drill_is_named_by_its_kind_counts_in_no_session_and_one_more_drills_again() {
    let dir = TestDir::new("practice-test-drill");
    let (mut runtime, mut hand) = in_the_gallery(&dir);
    let before = today(&runtime);
    assert!(runtime.start_test_drill("middle").is_err(), "no such kind");
    let id = (runtime.start_test_drill("centre")).expect("started");
    until(&mut runtime, &mut hand, |r| r.test_practice(id).is_some());
    assert_eq!(drilled(&runtime), Some((Family::Centre, 1)));
    let test = runtime.test_practice(id).expect("its name");
    let shown = runtime.practice().expect("a sheet");
    assert_eq!(shown.instructions, Some(Family::Centre), "its How first");
    // Its How closes by its word (on this small screen Got it lies below the card's room).
    hand.tap(&mut runtime, "How");
    copy(&mut runtime, &mut hand);
    // Finished: One more starts the kind's next drill, its How first again.
    for word in ["Next", "Next", "Finish", "One more"] {
        hand.tap(&mut runtime, word);
    }
    until(&mut runtime, &mut hand, |r| {
        drilled(r).is_some_and(|(_, number)| number == 2)
    });
    let again = runtime.practice().expect("a sheet");
    assert_eq!(again.instructions, Some(Family::Centre));
    assert!(runtime.back(), "back to the gallery");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    // No session: the Today card as before; the tile inked and ringed (the next drill open).
    let after = today(&runtime);
    assert_eq!(
        (after.stage, after.done, after.open),
        (before.stage, 0, None)
    );
    let tile = (after.drills.iter())
        .find(|tile| tile.family == Family::Centre)
        .expect("its tile");
    assert!(tile.compared && tile.open.is_some());
    // Stored as a test's sheet (S121b); One more's drill a test lesson on it, in place (S121c).
    drop(runtime);
    let mut library = Library::at(&dir);
    let purposes: Vec<(u64, Purpose, usize)> = (sheets(&mut library).expect("records").iter())
        .map(|sheet| (sheet.id, sheet.purpose, sheet.lessons.len()))
        .collect();
    assert_eq!(purposes, [(id, Purpose::Test, 2)]);
    drop(library);
    let (mut runtime, mut hand) = in_the_gallery(&dir);
    runtime.end_test_practice(&test).expect("ended");
    until(&mut runtime, &mut hand, |r| !r.holds_practice(id));
}
