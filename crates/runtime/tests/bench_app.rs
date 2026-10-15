//! The scenario benchmarks (S58, `cargo device-input bench <scenario>` on the tablet) run
//! offscreen through the whole frame loop on a library with a display cache: each finishes, says
//! what it measured, and leaves the canvas equal to a cold evaluation. `session` appends the open
//! artwork's strokes to a new one, which then holds them all but the undone ones.

mod gpu;
#[expect(
    dead_code,
    reason = "only the shared module's cold comparison is used here"
)]
mod hand;

use std::path::PathBuf;

use hand::assert_cold;
use openpaint_app::{BrushLibrary, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_runtime::{DisplayCache, Runtime, SCENARIOS, drawn, pile_path, style_of};
use openpaint_store::Library;

const SIZE: [u32; 2] = [1200, 900];
const PIXELS_PER_POINT: f32 = 2.0;
const CANVAS: [u32; 2] = [900, 640];
const STROKES: u32 = 24;

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

fn runtime() -> Runtime {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("bench-app-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clear an old test directory");
    }
    let mut library = Library::at(root.join("library"));
    let title = Title::new("Bench").expect("title");
    library.create(&pile(), &title).expect("created");
    let stores = (
        Box::new(library) as Box<dyn ProjectStore>,
        DisplayCache::at(root.join("cache")),
    );
    let mut runtime = Runtime::start_on_cached(gpu::core(), PIXELS_PER_POINT, stores)
        .expect("the runtime starts");
    let mut frames = 0;
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(SIZE).expect("frame");
        frames += 1;
        assert!(frames < 10_000, "the artwork never opened");
    }
    runtime
}

#[test]
fn every_scenario_benchmark_runs_through_the_frame_loop() {
    let core = gpu::core();
    let mut runtime = runtime();
    let mut appended = 0;
    for name in SCENARIOS {
        if name == "session" {
            appended = strokes(runtime.artwork().expect("artwork"));
        }
        let lines = runtime.bench_scenario(name).expect(name);
        eprintln!("{}", lines.join("\n"));
        assert!(lines.len() > 1, "{name}: {lines:?}");
        assert!(
            lines.iter().all(|line| !line.contains("NOT")),
            "{name} did not finish: {lines:?}"
        );
        assert_cold(&core, &mut runtime, name);
    }
    // The pile and a stroke of each texture brush; the session's new artwork holds them all but
    // the one undone (the 20th).
    assert_eq!(appended, STROKES as usize + 4);
    assert_eq!(strokes(runtime.artwork().expect("artwork")), appended - 1);
    assert!(runtime.bench_scenario("dance").is_err());
}

fn strokes(artwork: &Artwork) -> usize {
    (artwork.layers().iter())
        .map(|layer| layer.strokes().len())
        .sum()
}
