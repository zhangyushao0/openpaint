//! The scenario benchmarks (S58, `cargo device-input bench <scenario>` on the tablet) run
//! offscreen through the whole frame loop on a library with a display cache: each finishes, says
//! what it measured, and leaves the canvas equal to a cold evaluation. `session` appends the open
//! artwork's strokes to a new one, which then holds them all but the undone ones. `practice` runs
//! on a practice sheet started from the gallery's Today card and moves it three exercises on.

use openpaint_app::{BrushLibrary, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_runtime::{
    DisplayCache, Runtime, SCENARIOS, STROKES_ADDED, StartOptions, drawn, pile_path, style_of,
};
use openpaint_store::Library;

use crate::support::{Hand, assert_cold, core, gallery, library_dir, strokes, until};

/// Tall enough that `ui` finds the brush editor's StreamLine row in its column.
const SIZE: [u32; 2] = [1800, 1300];
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
    let root = library_dir("bench-app");
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
    runtime
}

#[test]
fn every_scenario_benchmark_runs_through_the_frame_loop() {
    let core = core();
    let mut runtime = runtime();
    let mut appended = 0;
    // `practice` runs on a practice sheet, below.
    for name in SCENARIOS.into_iter().filter(|name| *name != "practice") {
        if name == "session" {
            appended = strokes(&runtime);
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
    // The pile and the strokes the strokes bench added; the session's new artwork holds them all
    // but the one undone (the 20th).
    assert_eq!(appended, STROKES as usize + STROKES_ADDED);
    assert_eq!(strokes(&runtime), appended - 1);
    // S61: the zoom twice more, the second with its densest place already in the screen's middle
    // and its first zoom reached (the first ends there and so): neither its centring pan nor that
    // pinch is a two-finger tap, so nothing is undone.
    for _ in 0..2 {
        runtime.bench_scenario("zoom").expect("zoom again");
    }
    assert_eq!(strokes(&runtime), appended - 1, "the zoom undid nothing");
    assert!(runtime.bench_scenario("dance").is_err());
    assert!(
        runtime.bench_scenario("practice").is_err(),
        "no practice sheet"
    );
    // A practice sheet started from the Today card.
    let mut hand = Hand::new(SIZE);
    hand.tap(&mut runtime, "Gallery");
    until(&mut runtime, SIZE, |runtime| gallery(runtime).is_some());
    hand.tap(&mut runtime, "Start practice");
    until(&mut runtime, SIZE, |runtime| {
        runtime.projection().practice().is_some()
    });
    let lines = runtime.bench_scenario("practice").expect("practice");
    eprintln!("{}", lines.join("\n"));
    assert!(
        lines.iter().any(|line| line.starts_with("next 3")),
        "{lines:?}"
    );
    let practice = runtime.projection().practice().map(|p| p.current);
    assert_eq!(practice, Some(Some(3)), "three exercises on");
}
