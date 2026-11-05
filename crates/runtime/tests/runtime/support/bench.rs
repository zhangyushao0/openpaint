//! What the scenario benchmarks' tests start on (S58): a small pile of the built-ins' strokes in
//! a library and display cache of their own, on a window tall enough for `ui`. Shared by the
//! canvas benchmarks here and the practice benchmarks' binary of its own (`bench_practice.rs`).

use openpaint_app::{BrushLibrary, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_runtime::{DisplayCache, Runtime, StartOptions, drawn, pile_path, style_of};
use openpaint_store::Library;

use super::gpu::core;
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

/// A runtime on the pile in a library and display cache of their own, and their directory (bound
/// first by the caller, so it outlives the runtime).
pub fn on_the_pile() -> (TestDir, Runtime) {
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
    let mut frames = 0;
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(SIZE).expect("frame");
        frames += 1;
        assert!(frames < 10_000, "the artwork never opened");
    }
    (root, runtime)
}
