//! What the scenario benchmarks' tests start on (S58): a small pile of the built-ins' strokes in
//! a library and display cache of their own, on a window tall enough for `ui`; and the real time
//! the benchmarks run in between the hand's held steps. Shared by the canvas benchmarks here and
//! the practice benchmarks' binary of its own (`bench_practice.rs`).

use std::time::Instant;

use openpaint_app::{BrushLibrary, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_runtime::{DisplayCache, Runtime, StartOptions, drawn, pile_path, style_of};
use openpaint_store::Library;

use super::frames::until;
use super::gpu::core;
use super::hand::Hand;
use super::test_dir::TestDir;

/// Tall enough that `ui` finds the brush editor's StreamLine row in its column.
pub const SIZE: [u32; 2] = [1800, 1300];
const PIXELS_PER_POINT: f32 = 2.0;
const CANVAS: [u32; 2] = [900, 640];
pub const STROKES: u32 = 24;

/// A small pile: `STROKES` strokes of the built-ins in turn about the canvas's middle.
fn pile() -> Artwork {
    let library = BrushLibrary::built_in().expect("brushes");
    let keys = ["ink-pen", "textured-round", "soft-brush", "speckle"];
    let mut artwork = Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1]).expect("size"));
    let middle = CANVAS.map(|side| side as f32 / 2.0);
    for k in 0..STROKES {
        let style = style_of(&library, keys[k as usize % keys.len()], Some(24.0)).expect("style");
        let path: Vec<[f32; 2]> = (pile_path(k, middle).iter())
            .map(|[x, y]| {
                [
                    middle[0] + (x - middle[0]) * 0.6,
                    middle[1] + (y - middle[1]) * 0.6,
                ]
            })
            .collect();
        artwork = drawn(&artwork, style.with_seed(k), &path, |_| 0.8).expect("drawn");
    }
    artwork
}

/// A runtime on the pile in a library and display cache of their own, opened on the clock of a
/// hand on it, and their directory (bound first by the caller, so it outlives the runtime).
pub fn on_the_pile() -> (TestDir, Runtime, Hand) {
    let root = TestDir::new("bench-app");
    let mut library = Library::at(root.join("library"));
    let title = Title::new("Bench").expect("title");
    library.create(&pile(), &title).expect("created");
    let options = StartOptions {
        core: Some(core()),
        cache: Some(DisplayCache::at(root.join("cache"))),
        ..StartOptions::default()
    };
    let mut runtime =
        Runtime::start(PIXELS_PER_POINT, Box::new(library), options).expect("the runtime starts");
    let mut hand = Hand::new(SIZE);
    // The canvas's bar laid out, so a tap lands on it.
    until(&mut runtime, &mut hand, |runtime| {
        runtime.artwork().is_some()
    });
    (root, runtime, hand)
}

/// `run` (a benchmark: it paces itself by the wall clock, as a person does) with the runtime's
/// clocks in real time from where `hand` held them; then held again by the hand a second past
/// where real time took them, past any input the benchmark stamped ahead of its clock.
pub fn in_real_time<T>(
    hand: &mut Hand,
    runtime: &mut Runtime,
    run: impl FnOnce(&mut Runtime) -> T,
) -> T {
    let started = Instant::now();
    runtime.release_clock();
    let result = run(runtime);
    let took = i64::try_from(started.elapsed().as_millis()).expect("milliseconds");
    hand.tick(runtime, took + 1000);
    result
}
