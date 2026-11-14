//! The Pomodoro (S124) through the whole frame loop on a real library, its taps on the screens'
//! controls: a length chosen on the home kept across a restart and used by Start timer; a block
//! run to its time on the hand's held clock, the bar's End, the block's end and Another pomodoro
//! starting the next block on the next lesson.

use openpaint_practice::Length;
use openpaint_practice_app::BlockShown;
use openpaint_practice_store::{blocks, sheets};
use openpaint_runtime::Runtime;
use openpaint_store::Library;

use crate::support::{Hand, TestDir, gallery, start, until};

/// The tablet's screen: 1646 x 1029 points at density 1.75.
const SIZE: [u32; 2] = [2880, 1800];
const PIXELS_PER_POINT: f32 = 1.75;

/// A runtime on the library in `dir` and the hand on it, on the home.
fn on_the_home(dir: &std::path::Path) -> (Runtime, Hand) {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(dir)));
    assert!(runtime.back(), "to the home");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    (runtime, hand)
}

/// The block's minutes left and whether it is held, while one runs.
fn running(runtime: &Runtime) -> Option<(u32, bool)> {
    match runtime.timer().block {
        Some(BlockShown::Running { left, paused, .. }) => Some((left, paused)),
        _ => None,
    }
}

/// Start timer on the home, once its lesson shows; its How closed.
fn started(runtime: &mut Runtime, hand: &mut Hand) {
    hand.tap(runtime, "Start timer");
    until(runtime, hand, |r| {
        r.practice().is_some_and(|p| p.current.is_some())
    });
    if runtime.controls().find("Got it").is_ok() {
        hand.tap(runtime, "Got it");
    }
}

/// `length` tapped on the home's row, until the timer holds it.
fn chosen(runtime: &mut Runtime, hand: &mut Hand, length: Length) {
    let name = length.minutes().to_string();
    hand.tap(runtime, name.as_str());
    until(runtime, hand, |r| r.timer().length == length);
}

#[test]
fn a_length_chosen_on_the_home_is_kept_across_a_restart_and_starts_the_block() {
    let dir = TestDir::new("pomodoro-length");
    let (mut runtime, mut hand) = on_the_home(&dir);
    assert_eq!(runtime.timer().length, Length::TwentyFive, "25 at first");
    for length in [Length::Fifteen, Length::Forty, Length::Fifteen] {
        chosen(&mut runtime, &mut hand, length);
    }
    drop(runtime);
    let (mut runtime, mut hand) = on_the_home(&dir);
    assert_eq!(runtime.timer().length, Length::Fifteen, "kept");
    started(&mut runtime, &mut hand);
    assert_eq!(running(&runtime), Some((15, false)), "the block of 15");
}

#[test]
fn the_test_chores_short_block_ends_within_a_minute_and_its_end_leaves_nothing() {
    let dir = TestDir::new("pomodoro-test-chore");
    let (mut runtime, mut hand) = on_the_home(&dir);
    let id = runtime.start_test_pomodoro(60).expect("started");
    until(&mut runtime, &mut hand, |r| r.test_practice(id).is_some());
    assert_eq!(running(&runtime), Some((1, false)), "a minute of it");
    let test = runtime.test_practice(id).expect("its name");
    hand.tick(&mut runtime, 61 * 1000);
    until(&mut runtime, &mut hand, |r| {
        r.timer().block == Some(BlockShown::Up)
    });
    hand.tap(&mut runtime, "End");
    until(&mut runtime, &mut hand, |r| r.timer().sealed == 1);
    assert!(runtime.back(), "to the home");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    runtime.end_test_practice(&test).expect("ended");
    until(&mut runtime, &mut hand, |r| !r.holds_practice(id));
    assert_eq!(runtime.timer().sealed, 0, "its seal gone");
    drop(runtime);
    let mut library = Library::at(&dir);
    assert_eq!(blocks(&mut library).map(|b| b.done.len()), Ok(0));
    assert_eq!(
        sheets(&mut library).map(|s| s.len()),
        Ok(0),
        "no sheet left"
    );
}

#[test]
fn a_block_run_to_its_time_ends_by_the_bar_and_another_pomodoro_starts_the_next() {
    let dir = TestDir::new("pomodoro-block");
    let (mut runtime, mut hand) = on_the_home(&dir);
    started(&mut runtime, &mut hand);
    assert_eq!(running(&runtime), Some((25, false)));
    assert!(runtime.screen_on(), "kept on while it counts");
    assert!(runtime.wake_in().is_some(), "a frame at the minute's turn");
    let lesson = runtime
        .practice()
        .and_then(|p| p.lesson)
        .expect("its lesson");
    // The tablet put down 25 minutes with the sheet in front: the time is up.
    hand.tick(&mut runtime, 25 * 60 * 1000);
    until(&mut runtime, &mut hand, |r| {
        r.timer().block == Some(BlockShown::Up)
    });
    assert!(runtime.controls().find("Next").is_err(), "End in its place");
    hand.tap(&mut runtime, "End");
    until(&mut runtime, &mut hand, |r| {
        matches!(r.timer().block, Some(BlockShown::Ended { .. }))
    });
    assert_eq!(runtime.timer().sealed, 1, "today's seal");
    assert!(runtime.screen_on(), "the break counts");
    hand.tap(&mut runtime, "Another pomodoro");
    until(&mut runtime, &mut hand, |r| {
        let next = r.practice().and_then(|p| p.lesson);
        running(r).is_some() && next.is_some_and(|next| next.row.number > lesson.row.number)
    });
    assert_eq!(running(&runtime), Some((25, false)), "a new block");
    drop(runtime);
    let kept = blocks(&mut Library::at(&dir)).expect("the timer's records");
    assert_eq!(kept.done.len(), 1, "the block completed kept");
    assert!(kept.under_way.is_some(), "the next under way");
}
