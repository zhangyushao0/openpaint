//! The Pomodoro as the Android host drives it: nothing draws at rest, so once a frame wants no
//! other the host posts the next for when `Runtime::wake_in` says (`TimerHost.kt`). A block
//! counting, held and run to its time, and its break to its chime, frame by frame on the hand's
//! clock with no frame but the ones the host would draw.

use std::time::Duration;

use openpaint_practice_app::BlockShown;
use openpaint_runtime::{Runtime, ToolKind};
use openpaint_store::Library;

use crate::support::{Hand, TestDir, gallery, mapping, start, until};

/// The tablet's screen: 1646 x 1029 points at density 1.75.
const SIZE: [u32; 2] = [2880, 1800];
const PIXELS_PER_POINT: f32 = 1.75;
/// The Android host's margin past the moment the timer names.
const MARGIN_MS: i64 = 50;

/// The minutes the bar's ring or the break shows; none once neither counts down.
fn minutes(runtime: &Runtime) -> Option<u32> {
    match runtime.timer().block {
        Some(BlockShown::Running { left, .. }) => Some(left),
        Some(BlockShown::Ended { rest, .. }) => rest.map(|(left, _)| left),
        _ => None,
    }
}

/// Frames while the runtime wants the next at once, then at rest: the wake the host posts.
fn at_rest(runtime: &mut Runtime, hand: &mut Hand) -> Option<Duration> {
    until(runtime, hand, |_| true);
    runtime.wake_in()
}

/// At rest with `minutes` shown counting down: the host's wake is no later than the minute's
/// turn, and the frame it posts shows the next minute (or the end).
fn woken(runtime: &mut Runtime, hand: &mut Hand) {
    let shown = minutes(runtime).expect("counting");
    let wake = at_rest(runtime, hand).expect("a frame wanted while it counts");
    assert!(wake <= Duration::from_secs(60), "{wake:?}: past the minute");
    let ms = i64::try_from(wake.as_millis()).expect("ms") + MARGIN_MS;
    hand.tick(runtime, ms);
    hand.frame_now(runtime);
    assert_eq!(
        minutes(runtime).unwrap_or(0),
        shown - 1,
        "the minute turned at the wake"
    );
}

/// A line across the shown exercise's empty frame: the exercise begun.
pub fn drawn(runtime: &mut Runtime, hand: &mut Hand) {
    let practice = runtime.practice().expect("a practice sheet");
    let frame = practice.sheet.pairs[practice.current.expect("an exercise")].copy;
    let view = mapping(runtime, SIZE, PIXELS_PER_POINT);
    let at = |p: [f64; 2]| view.to_screen(frame.to_canvas(p).map(|v| v as f32));
    let line = [at([0.3, 0.3]), at([0.5, 0.5]), at([0.7, 0.6])];
    hand.gesture(runtime, ToolKind::Stylus, &line);
}

#[test]
fn the_host_wakes_a_counting_block_and_its_break_to_the_chime_and_never_a_held_one() {
    let dir = TestDir::new("pomodoro-wake");
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    assert!(runtime.back(), "to the home");
    until(&mut runtime, &mut hand, |r| gallery(r).is_some());
    let id = runtime.start_test_pomodoro(150).expect("started");
    until(&mut runtime, &mut hand, |r| r.test_practice(id).is_some());
    if runtime.controls().find("Got it").is_ok() {
        hand.tap(&mut runtime, "Got it");
    }
    assert_eq!(minutes(&runtime), Some(3), "two and a half minutes");
    woken(&mut runtime, &mut hand);
    // Held by the bar's Pause: no frame wanted until Continue.
    let ring = format!("{} min", minutes(&runtime).expect("counting"));
    hand.tap(&mut runtime, ring.as_str());
    hand.tap(&mut runtime, "Pause");
    assert_eq!(at_rest(&mut runtime, &mut hand), None, "held");
    hand.tap(&mut runtime, ring.as_str());
    hand.tap(&mut runtime, "Continue");
    drawn(&mut runtime, &mut hand);
    while minutes(&runtime).is_some() {
        woken(&mut runtime, &mut hand);
    }
    assert_eq!(runtime.timer().block, Some(BlockShown::Up));
    assert_eq!(at_rest(&mut runtime, &mut hand), None, "the time is up");
    hand.tap(&mut runtime, "End");
    assert_eq!(minutes(&runtime), Some(5), "the break");
    assert!(!runtime.take_chime());
    while minutes(&runtime).is_some() {
        woken(&mut runtime, &mut hand);
    }
    assert!(runtime.take_chime(), "the break's end chimes");
    assert!(!runtime.take_chime(), "once");
    assert_eq!(at_rest(&mut runtime, &mut hand), None, "the break is over");
}
