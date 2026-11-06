//! The practice benchmarks (S58, S90; `cargo device-input bench book|practice` on the tablet)
//! offscreen through the whole frame loop, from the benchmarks' pile (`runtime/support/bench.rs`):
//! in the gallery the practice book's; then on a practice sheet started from the gallery's Today
//! card, three exercises on and to its first group of combined lines, each compared aligned with
//! its fit shown; then on stage 3's first session, started from the practice options once stage
//! 1's was practised, its point, angle and point in space compared. A test binary of its own
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
use openpaint_runtime::{Runtime, ToolKind};

use bench::{in_real_time, on_the_pile};
use frames::{settle, until};
use hand::{Hand, assert_cold, on};

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
fn the_book_and_practice_benchmarks_run_in_the_gallery_and_on_sheets() {
    let (_root, mut runtime, mut hand) = on_the_pile();
    let (runtime, hand) = (&mut runtime, &mut hand);
    // The pile shown as drawn cold.
    assert_cold(&gpu::core(), runtime, "the pile opened");
    hand.tap(runtime, "Gallery");
    until(runtime, hand, |runtime| gallery(runtime).is_some());
    let book = bench(hand, runtime, "book");
    assert_eq!(book.len(), 6, "{book:?}");
    hand.tap(runtime, "Start practice");
    until(runtime, hand, |runtime| {
        runtime.projection().practice().is_some()
    });
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
    let projection = runtime.projection();
    let practice = projection.practice().expect("the sheet");
    assert_eq!(practice.current, Some(10), "on to the first colour");
    assert!(
        practice.comparing && practice.result.is_some(),
        "filled and compared"
    );
    stage_three(runtime, hand);
}

/// Stage 3 started from the practice options and its bench: a point, an angle and a point in
/// space marked and compared (S90).
fn stage_three(runtime: &mut Runtime, hand: &mut Hand) {
    hand.tap(runtime, "Gallery");
    until(runtime, hand, |runtime| gallery(runtime).is_some());
    hand.tap(runtime, "Practice options");
    // On this short window stage 3's row lies below the Practice column's room: a finger drags
    // the column up to it (S91's tiles take the room).
    let [x, y] = on(runtime, "Stage 2 · Single shapes", [0.3, 0.5]);
    let drag: Vec<[f32; 2]> = (0..=12).map(|step| [x, y - 50.0 * step as f32]).collect();
    hand.gesture(runtime, ToolKind::Finger, &drag);
    settle(runtime, hand);
    hand.tap(runtime, "Stage 3 · Points");
    until(runtime, hand, |runtime| {
        (runtime.projection().practice()).is_some_and(|practice| practice.stage == 3)
    });
    let lines = bench(hand, runtime, "practice");
    eprintln!("{}", lines.join("\n"));
    for motion in [
        "point compare opens",
        "angle compare closes",
        "point in space compare opens",
    ] {
        let found = lines.iter().any(|line| line.starts_with(motion));
        assert!(found, "{motion}: {lines:?}");
    }
}
