//! The test-session chore (`cargo device-input practice-test start|end`) through the whole frame
//! loop on a real library: a test session started from the gallery beside the author's open one,
//! drawn on and compared, then removed. The author's session record, their sheet (as the bytes of
//! its OpenPaint file) and the Today card are as before; the store's own test holds every other
//! record to its bytes. Motion runs on a held clock, 16 ms a frame, as in scenarios.

use std::time::Instant;

use openpaint_app::{ProjectId, ProjectStore, TodayProjection};
use openpaint_practice::Family;
use openpaint_runtime::{Runtime, StartOptions, ToolKind};
use openpaint_store::Library;

use crate::support::{Hand, TestDir, core, gallery, mapping};

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// The runtime on a held clock.
struct Held {
    runtime: Runtime,
    clock: i64,
}

impl Held {
    fn start(dir: &std::path::Path) -> Self {
        let options = StartOptions {
            core: Some(core()),
            ..StartOptions::default()
        };
        let store = Box::new(Library::at(dir));
        let runtime = Runtime::start(PIXELS_PER_POINT, store, options).expect("starts");
        Self { runtime, clock: 0 }
    }

    /// Frames 16 ms apart until `ready` holds, nothing moves and no worker is busy; panics
    /// after 30 seconds.
    fn until(&mut self, ready: impl Fn(&Runtime) -> bool) {
        let start = Instant::now();
        loop {
            self.clock += 16;
            self.runtime.fix_clock(self.clock);
            let again = self.runtime.frame_offscreen(SIZE).expect("frame");
            if !again && !self.runtime.storing() && ready(&self.runtime) {
                return;
            }
            assert!(start.elapsed().as_secs() < 30, "never got there");
        }
    }
}

fn today(runtime: &Runtime) -> TodayProjection {
    gallery(runtime)
        .and_then(|g| g.today().cloned())
        .expect("the Today card")
}

/// A line across the shown exercise's empty frame, Compare, and back to the gallery.
fn copy_and_compare(held: &mut Held, hand: &mut Hand) {
    copy(&mut held.runtime, hand);
    assert!(held.runtime.back(), "back to the gallery");
    held.until(|r| gallery(r).is_some());
}

/// A line across the shown exercise's empty frame, and Compare.
fn copy(runtime: &mut Runtime, hand: &mut Hand) {
    let practice = runtime
        .projection()
        .practice()
        .cloned()
        .expect("a practice sheet");
    let frame = practice.sheet.pairs[practice.current.expect("an exercise")].copy;
    let view = mapping(runtime, SIZE, PIXELS_PER_POINT);
    let at = |p: [f64; 2]| view.to_screen(frame.to_canvas(p).map(|v| v as f32));
    hand.gesture(
        runtime,
        ToolKind::Stylus,
        &[at([0.3, 0.3]), at([0.5, 0.5]), at([0.7, 0.6])],
    );
    hand.tap(runtime, "Compare");
    assert!(runtime.projection().practice().is_some_and(|p| p.comparing));
}

#[test]
fn a_test_session_beside_the_authors_leaves_theirs_as_it_was() {
    let dir = TestDir::new("practice-test");
    let mut held = Held::start(&dir);
    let mut hand = Hand::new(SIZE);
    // The author's session: a line copied and compared, the session left open.
    held.until(|r| r.artwork().is_some());
    assert!(held.runtime.back());
    held.until(|r| gallery(r).is_some());
    hand.tap(&mut held.runtime, "Start practice");
    held.until(|r| r.projection().practice().is_some());
    hand.tap(&mut held.runtime, "Got it");
    copy_and_compare(&mut held, &mut hand);
    let theirs = today(&held.runtime);
    let sheet = theirs.open.expect("their session open");
    drop(held);
    let mut library = Library::at(&dir);
    let records = library.practice().expect("their records");
    let file = library.artwork_file(sheet).expect("their sheet");
    drop(library);

    // The chore: a test session beside theirs, worked on, then removed. No artwork is listed,
    // so the app starts on a blank canvas, left for the gallery.
    let mut held = Held::start(&dir);
    held.until(|r| r.artwork().is_some());
    assert!(held.runtime.back());
    held.until(|r| gallery(r).is_some());
    assert_eq!(today(&held.runtime), theirs);
    let id = (held.runtime.start_test_practice(None, None)).expect("started");
    held.until(|r| r.test_practice(id).is_some());
    let test = held.runtime.test_practice(id).expect("its name");
    assert_ne!(test.sheet, sheet.get(), "a sheet of its own");
    let practice = held
        .runtime
        .projection()
        .practice()
        .cloned()
        .expect("the test sheet");
    assert!(
        practice.warming_up,
        "it starts as their next session would, warming up"
    );
    copy_and_compare(&mut held, &mut hand);
    assert_eq!(
        today(&held.runtime).open.map(ProjectId::get),
        Some(test.sheet)
    );
    let named = test.to_string().parse().expect("its name read back");
    held.runtime.end_test_practice(&named).expect("ended");
    held.until(|r| !r.holds_practice(id));
    assert_eq!(today(&held.runtime), theirs, "the Today card as before");
    let mut other = test;
    other.seed ^= 1;
    let refused = held.runtime.end_test_practice(&other);
    assert!(refused.is_err(), "nothing else is removed");
    drop(held);
    let mut library = Library::at(&dir);
    assert_eq!(
        library.practice().expect("records"),
        records,
        "their records"
    );
    assert_eq!(library.artwork_file(sheet).expect("their sheet"), file);
    assert!(library.load(ProjectId::new(test.sheet)).is_err());
}

/// The kind a drill's sheet drills and its number, while one shows.
fn drilled(runtime: &Runtime) -> Option<(Family, u32)> {
    let practice = runtime.projection().practice().cloned()?;
    Some((practice.drill?, practice.number))
}

#[test]
fn a_test_drill_is_named_by_its_kind_counts_in_no_session_and_one_more_drills_again() {
    let dir = TestDir::new("practice-test-drill");
    let mut held = Held::start(&dir);
    let mut hand = Hand::new(SIZE);
    held.until(|r| r.artwork().is_some());
    assert!(held.runtime.back());
    held.until(|r| gallery(r).is_some());
    let before = today(&held.runtime);
    assert!(
        held.runtime.start_test_drill("middle").is_err(),
        "no such kind"
    );
    let id = (held.runtime.start_test_drill("centre")).expect("started");
    held.until(|r| r.test_practice(id).is_some());
    assert_eq!(drilled(&held.runtime), Some((Family::Centre, 1)));
    let test = held.runtime.test_practice(id).expect("its name");
    let shown = held
        .runtime
        .projection()
        .practice()
        .cloned()
        .expect("a sheet");
    assert_eq!(shown.instructions, Some(Family::Centre), "its How first");
    // Its How closes by its word (on this small screen Got it lies below the card's room).
    hand.tap(&mut held.runtime, "How");
    copy(&mut held.runtime, &mut hand);
    // Finished: One more starts the kind's next drill, its How first again.
    for word in ["Next", "Next", "Finish", "One more"] {
        hand.tap(&mut held.runtime, word);
    }
    held.until(|r| drilled(r).is_some_and(|(_, number)| number == 2));
    let again = held
        .runtime
        .projection()
        .practice()
        .cloned()
        .expect("a sheet");
    assert_eq!(again.instructions, Some(Family::Centre));
    assert!(held.runtime.back(), "back to the gallery");
    held.until(|r| gallery(r).is_some());
    // No session: the Today card as before; the tile inked and ringed (the next drill open).
    let after = today(&held.runtime);
    assert_eq!(
        (after.stage, after.done, after.open),
        (before.stage, 0, None)
    );
    let tile = (after.drills.iter())
        .find(|tile| tile.family == Family::Centre)
        .expect("its tile");
    assert!(tile.compared && tile.open.is_some());
    held.runtime.end_test_practice(&test).expect("ended");
    held.until(|r| !r.holds_practice(id));
}
