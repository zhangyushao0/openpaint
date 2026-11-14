//! The practice benchmarks (S58, S90; `cargo device-input bench book|practice` on the tablet)
//! offscreen through the whole frame loop, from the benchmarks' pile (`runtime/support/bench.rs`):
//! on the home the practice book's; then on unit 1's full lesson started from the home,
//! three exercises on and to its first group of combined lines, each compared aligned with
//! its fit shown; then on unit 3's full lesson, started from the home once unit
//! 1's was practised, its point, point in space and solid compared. A test binary of its own
//! (S103): the benchmarks time their device's frames, which two at once on one device cannot, so
//! these run on their own device beside the canvas benchmarks (`runtime/bench_app.rs`) instead of
//! after them; both go at a person's pace, in real time (`in_real_time`), the taps between on the
//! hand's held clock.

#[path = "runtime/support/bench.rs"]
mod bench;
#[path = "runtime/support/frames.rs"]
mod frames;
#[path = "runtime/support/gpu.rs"]
mod gpu;
#[path = "runtime/support/hand.rs"]
mod hand;
#[path = "runtime/support/test_dir.rs"]
mod test_dir;

use openpaint_app::{GalleryProjection, ScreenProjection};
use openpaint_runtime::{Runtime, Start};

use bench::{in_real_time, on_the_pile};
use frames::{settle, until};
use hand::{Hand, assert_cold};

/// The gallery, once it lists the library.
fn gallery(runtime: &Runtime) -> Option<GalleryProjection> {
    match runtime.screen() {
        ScreenProjection::Gallery(gallery) if gallery.listed() => Some(gallery),
        _ => None,
    }
}

/// The benchmark `name` in real time; its lines.
fn bench(hand: &mut Hand, runtime: &mut Runtime, name: &str) -> Vec<String> {
    in_real_time(hand, runtime, |runtime| {
        runtime.bench_scenario(name).expect(name)
    })
}

#[test]
fn the_book_and_practice_benchmarks_run_on_the_home_and_on_sheets() {
    let (_root, mut runtime, mut hand) = on_the_pile();
    let (runtime, hand) = (&mut runtime, &mut hand);
    // The pile shown as drawn cold.
    assert_cold(&gpu::core(), runtime, "the pile opened");
    hand.tap(runtime, "Home");
    until(runtime, hand, |runtime| gallery(runtime).is_some());
    let book = bench(hand, runtime, "book");
    assert_eq!(book.len(), 6, "{book:?}");
    started(runtime, hand, 1);
    let lines = bench(hand, runtime, "practice");
    eprintln!("{}", lines.join("\n"));
    for motion in [
        "aligned",
        "line compared aligned, the fit shown",
        "next 3",
        "group compared aligned, the fit shown",
        "inline wheel dragged",
        "colour compared",
    ] {
        let found = lines.iter().any(|line| line.starts_with(motion));
        assert!(found, "{motion}: {lines:?}");
    }
    assert!(
        lines.iter().all(|line| !line.contains("NOT shown")),
        "every fit shown: {lines:?}"
    );
    let practice = runtime.practice().expect("the sheet");
    assert!(
        practice.comparing && practice.result.is_some(),
        "filled and compared"
    );
    unit_three(runtime, hand);
}

/// A lesson of unit `unit`'s first skill started from the home (`Runtime::start_lesson`, as a
/// scenario's `lesson` step starts it), once its sheet shows and its motion settled.
///
/// S124 integration: the full lesson this walked is gone; the walk goes through a Pomodoro's
/// lessons one after another (unit 1's line, group and colour; unit 3's point, point in space and
/// solid), the bench (`runtime/src/bench/practice.rs`) looking ahead past the lesson in hand.
fn started(runtime: &mut Runtime, hand: &mut Hand, unit: u8) {
    let skill = openpaint_practice::UNITS[usize::from(unit) - 1].skills[0];
    runtime.start_lesson(Start::Skill(skill)).expect("started");
    until(runtime, hand, |runtime| {
        let lesson = runtime.practice().and_then(|practice| practice.lesson);
        lesson.is_some_and(|lesson| lesson.row.skill == Some(skill))
    });
    settle(runtime, hand);
}

/// Unit 3's full lesson started from the home and its bench: a point, a point in space and a
/// solid marked and compared (S90).
fn unit_three(runtime: &mut Runtime, hand: &mut Hand) {
    hand.tap(runtime, "Home");
    until(runtime, hand, |runtime| gallery(runtime).is_some());
    started(runtime, hand, 3);
    let lines = bench(hand, runtime, "practice");
    eprintln!("{}", lines.join("\n"));
    for motion in [
        "point compare opens",
        "solid compare closes",
        "point in space compare opens",
    ] {
        let found = lines.iter().any(|line| line.starts_with(motion));
        assert!(found, "{motion}: {lines:?}");
    }
}
