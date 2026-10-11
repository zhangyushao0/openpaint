//! Device-loss recovery (S49) through the whole frame loop. The GPU device is destroyed under the
//! running app while the pen is drawing (wgpu's `Device::destroy`, which on this backend reaches
//! the device-lost callback, as a driver reset does): the next frame makes the renderer again on
//! a new device; the canvas shows the artwork and the stroke so far as a cold evaluation does,
//! the notice says the display was reset, the hand goes on and the stroke commits whole at the
//! lift, and undo still works. A test binary of its own: it destroys its device.

mod gpu;
mod hand;
mod support;

use std::sync::Arc;

use hand::{Hand, assert_cold};
use openpaint_core::Color;
use openpaint_render::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use support::Nowhere;

const SIZE: [u32; 2] = [1280, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// Right of the side bar.
fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [200.0 + 40.0 * f32::from(i), y])
        .collect()
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

/// One pen event at `at`, then the frames the runtime asks for (at most four).
fn pen(runtime: &mut Runtime, hand: &mut Hand, phase: MotionPhase, at: [f32; 2]) {
    hand.clock += 8;
    runtime.motion(&MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers: vec![Pointer {
            id: 0,
            tool: ToolKind::Stylus,
            samples: vec![PointerSample {
                position: at,
                pressure: 0.8,
                tilt: 0.0,
                orientation: 0.0,
                time_ms: hand.clock,
            }],
        }],
    });
    for _ in 0..4 {
        if !runtime.frame_offscreen(SIZE).expect("frame") {
            break;
        }
    }
}

/// Whether the canvas shows what the artwork and the stroke being drawn give evaluated cold on
/// the runtime's device.
fn shows_cold(runtime: &Runtime) -> bool {
    let core = runtime.core();
    let (artwork, active) = runtime.drawn().expect("a canvas");
    let canvas = artwork.canvas();
    let size = [canvas.width(), canvas.height()];
    let mut target = CanvasTarget::new(&core, size).expect("target");
    let request = FrameRequest {
        artwork: &artwork,
        active: active.as_ref(),
        background: Color::WHITE,
    };
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request, &[PixelRect::full(size)])
        .expect("evaluate");
    let cold = target.read(&core).expect("read");
    let shown = runtime.read_canvas().expect("read").expect("canvas");
    shown.content_hash() == cold.content_hash()
}

/// Destroys the device under `runtime` and runs frames until it rests: the device made again
/// shows the artwork and the stroke so far as cold, with the notice. Returns the new device.
fn lost_and_made_again(runtime: &mut Runtime) -> Arc<openpaint_render::Core> {
    assert!(
        runtime.lose_device(),
        "Device::destroy reached wgpu's device-lost callback"
    );
    for _ in 0..240 {
        runtime
            .frame_offscreen(SIZE)
            .expect("a frame after the loss");
        if !runtime.moving() {
            break;
        }
    }
    let core = runtime.core();
    assert!(core.lost().is_none());
    assert!(
        shows_cold(runtime),
        "the artwork and the stroke so far, as cold"
    );
    let names = runtime.controls().names();
    assert!(
        names.iter().any(|name| name == "OK"),
        "the notice: {names:?}"
    );
    core
}

#[test]
fn a_lost_device_comes_back_with_the_canvas_history_and_the_stroke_being_drawn() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    for y in [300.0, 440.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    let first = runtime.core();
    assert_cold(&first, &runtime, "before the loss");
    let before = runtime.read_canvas().expect("read").expect("canvas");
    // The pen is down and drawing when the device goes.
    let points = line(580.0);
    pen(&mut runtime, &mut hand, MotionPhase::Down, points[0]);
    for at in &points[1..10] {
        pen(&mut runtime, &mut hand, MotionPhase::Move, *at);
    }
    let second = lost_and_made_again(&mut runtime);
    assert!(!Arc::ptr_eq(&first, &second), "a new device");
    assert!(first.lost().is_some());
    // The hand goes on and lifts: the stroke commits whole.
    for at in &points[10..] {
        pen(&mut runtime, &mut hand, MotionPhase::Move, *at);
    }
    pen(
        &mut runtime,
        &mut hand,
        MotionPhase::Up,
        points[points.len() - 1],
    );
    assert_eq!(
        strokes(&runtime),
        3,
        "the stroke drawn across the loss committed"
    );
    let artwork = runtime.artwork().expect("artwork");
    let last = artwork.layers()[0].strokes().last().expect("a stroke");
    assert!(
        last.samples().len() >= points.len() - 1,
        "every sample before and after the loss: {}",
        last.samples().len()
    );
    assert_cold(&second, &runtime, "the stroke committed");
    // Undo takes it back: the canvas as before the loss.
    hand.tap(&mut runtime, "Undo");
    assert_eq!(strokes(&runtime), 2);
    let undone = runtime.read_canvas().expect("read").expect("canvas");
    assert_eq!(
        undone.content_hash(),
        before.content_hash(),
        "undo after the loss"
    );
    assert_cold(&second, &runtime, "after undo");
}
