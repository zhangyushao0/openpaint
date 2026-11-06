//! The scenario benchmarks (S58, `cargo device-input bench <scenario>` on the tablet) run
//! offscreen through the whole frame loop on a library with a display cache: each finishes, says
//! what it measured, and leaves the canvas equal to a cold evaluation. `session` appends the open
//! artwork's strokes to a new one, which then holds them all but the undone ones. `book` and
//! `practice` run in a binary of their own (`bench_practice.rs`), beside this one (S103): the
//! benchmarks time their device's frames, which two at once on one device cannot, and one after
//! another they took over three minutes, at a person's pace. Each runs in real time
//! (`in_real_time`), the steps between on the hand's held clock.

use openpaint_runtime::{SCENARIOS, STROKES_ADDED};

use crate::support::bench::{STROKES, in_real_time, on_the_pile};
use crate::support::{assert_cold, core, strokes};

#[test]
fn every_canvas_benchmark_runs_through_the_frame_loop() {
    let core = core();
    let (_root, mut runtime, mut hand) = on_the_pile();
    let mut appended = 0;
    // `practice` runs on a practice sheet and `book` in the gallery (`bench_practice.rs`).
    for name in SCENARIOS
        .into_iter()
        .filter(|name| !["practice", "book"].contains(name))
    {
        if name == "session" {
            appended = strokes(&runtime);
        }
        let lines = in_real_time(&mut hand, &mut runtime, |runtime| {
            runtime.bench_scenario(name).expect(name)
        });
        eprintln!("{}", lines.join("\n"));
        assert!(lines.len() > 1, "{name}: {lines:?}");
        assert!(
            lines.iter().all(|line| !line.contains("NOT")),
            "{name} did not finish: {lines:?}"
        );
        assert_cold(&core, &mut runtime, name);
    }
    // The pile and the strokes the strokes bench added; the session's new artwork holds them all
    // but the one undone (the 20th).
    assert_eq!(appended, STROKES as usize + STROKES_ADDED);
    assert_eq!(strokes(&runtime), appended - 1);
    // S61: the zoom twice more, the second with its densest place already in the screen's middle
    // and its first zoom reached (the first ends there and so): neither its centring pan nor that
    // pinch is a two-finger tap, so nothing is undone.
    for _ in 0..2 {
        in_real_time(&mut hand, &mut runtime, |runtime| {
            runtime.bench_scenario("zoom").expect("zoom again")
        });
    }
    assert_eq!(strokes(&runtime), appended - 1, "the zoom undid nothing");
    // Both refused before a frame.
    assert!(runtime.bench_scenario("dance").is_err());
    assert!(
        runtime.bench_scenario("practice").is_err(),
        "no practice sheet"
    );
}
