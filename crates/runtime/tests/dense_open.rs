//! Opening a dense artwork (S49) through the whole frame loop: 300 strokes crossing one place, so
//! the whole canvas's work passes what one frame takes (here with a small frame budget, as the
//! tablet's slower GPU gives it at the real one). The canvas fills in over frames, the screen
//! keeps answering (a tap opens the layer panel while it fills), and the canvas ends equal to it
//! drawn cold. A test binary of its own: it sets its device's frame budget.

#[path = "runtime/support/gpu.rs"]
mod gpu;
#[path = "runtime/support/hand.rs"]
mod hand;
#[path = "runtime/support/store.rs"]
mod store;

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

use hand::{HANG, Hand, assert_cold};
use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
};
use openpaint_runtime::{Runtime, StartOptions};
use store::Nowhere;

const SIZE: [u32; 2] = [1600, 1400];
const PIXELS_PER_POINT: f32 = 2.0;
const CANVAS: [u32; 2] = [1000, 700];

/// 300 strokes crossing the canvas's middle at every angle.
fn dense() -> Artwork {
    let mut artwork = Artwork::new(CanvasSize::new(CANVAS[0], CANVAS[1]).expect("size"));
    let middle = CANVAS.map(|side| side as f32 / 2.0);
    for k in 0..300u32 {
        let angle = k as f32 * 2.399_963_2;
        let shift = [((k * 37) % 61) as f32 - 30.0, ((k * 59) % 41) as f32 - 20.0];
        let samples: Vec<StrokeSample> = (0..20u16)
            .map(|i| {
                let t = f32::from(i) / 19.0;
                let s = 360.0 * (t - 0.5);
                let at = CanvasPoint::new(
                    middle[0] + shift[0] + s * angle.cos(),
                    middle[1] + shift[1] + s * angle.sin(),
                )
                .expect("point");
                let pressure = 0.6 + 0.3 * (PI * t).sin();
                StrokeSample::new(at, pressure, 0.0, 0.0, u32::from(i) * 8).expect("sample")
            })
            .collect();
        let colour = Color::rgb(0.2 + 0.08 * (k % 7) as f32, 0.3, 0.6).expect("colour");
        let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 40.0, 0.7, colour);
        let mut stroke = artwork
            .begin_stroke(style.expect("style"), samples[0])
            .expect("begin");
        stroke.extend(&samples[1..]).expect("extend");
        artwork = stroke.finish().artwork;
    }
    artwork
}

#[test]
fn a_dense_artwork_fills_in_over_frames_while_taps_work_and_ends_exact() {
    let core = gpu::core();
    core.set_frame_budget(10_000.0);
    let store = Nowhere {
        last: Some(dense()),
        ..Nowhere::default()
    };
    let options = StartOptions {
        core: Some(Arc::clone(&core)),
        blank: true,
        ..StartOptions::default()
    };
    let mut runtime =
        Runtime::start(PIXELS_PER_POINT, Box::new(store), options).expect("the runtime starts");
    let mut hand = Hand::new(SIZE);
    // Single frames a 60 Hz frame apart, each counted.
    let frame = |runtime: &mut Runtime, hand: &mut Hand| {
        hand.tick(runtime, 16);
        runtime.frame_offscreen(SIZE).expect("frame");
    };
    let start = Instant::now();
    while runtime.artwork().is_none() {
        frame(&mut runtime, &mut hand);
        assert!(start.elapsed() < HANG, "the artwork never opened");
    }
    let mut filling = 0;
    while runtime.moving() {
        if filling == 2 {
            // The screen answers while the canvas fills in.
            hand.tap(&mut runtime, "Layers");
            let names = runtime.controls().names();
            assert!(
                names.iter().any(|name| name == "Add layer"),
                "the layer panel opened while filling: {names:?}"
            );
            assert!(runtime.moving(), "still filling in after the tap");
        }
        frame(&mut runtime, &mut hand);
        filling += 1;
        assert!(start.elapsed() < HANG, "the canvas never filled in");
    }
    eprintln!("the dense artwork filled in over {filling} frames");
    assert!(filling > 4, "{filling} frames");
    assert_cold(&core, &mut runtime, "the canvas filled in");
}
