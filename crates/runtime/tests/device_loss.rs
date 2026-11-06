//! Device-loss recovery (S49) through the whole frame loop. The GPU device is destroyed under the
//! running app while the pen is drawing (wgpu's `Device::destroy`, which on this backend reaches
//! the device-lost callback, as a driver reset does): the next frame makes the renderer again on
//! a new device; the canvas shows the artwork and the stroke so far as a cold evaluation does,
//! the notice says the display was reset, the hand goes on and the stroke commits whole at the
//! lift, and undo still works. A test binary of its own: it destroys its device.

#[path = "runtime/support/frames.rs"]
mod frames;
#[path = "runtime/support/gpu.rs"]
mod gpu;
#[path = "runtime/support/hand.rs"]
mod hand;
#[path = "runtime/support/launch.rs"]
mod launch;
#[path = "runtime/support/store.rs"]
mod store;

use std::sync::Arc;
use std::time::Instant;

use frames::settle;
use hand::{HANG, Hand, assert_cold};
use openpaint_core::Color;
use openpaint_render::{CanvasTarget, FrameRequest};
use openpaint_runtime::{MotionPhase, Runtime, ToolKind};
use store::Nowhere;

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

/// One pen event at `at`, then the frames the runtime asks for.
fn pen(runtime: &mut Runtime, hand: &mut Hand, phase: MotionPhase, at: [f32; 2]) {
    hand.send(runtime, phase, &[(0, ToolKind::Stylus, at)]);
}

/// Whether the canvas shows what the artwork and the stroke being drawn give evaluated cold on
/// the runtime's device.
fn shows_cold(runtime: &mut Runtime) -> bool {
    let core = runtime.core();
    let (artwork, active) = runtime.drawn().expect("a canvas");
    let request = FrameRequest {
        artwork: &artwork,
        active: active.as_ref(),
        background: Color::WHITE,
    };
    let cold = CanvasTarget::whole(&core, &request, false).expect("drawn whole");
    let shown = runtime.read_canvas().expect("read").expect("canvas");
    shown.content_hash() == cold.content_hash()
}

/// Destroys the device under `runtime` and runs frames until the device is made again, the
/// canvas filled in and the notice shown: the device made again shows the artwork and the stroke
/// so far as cold. The frames take no time on the hand's clock: the pen, still down, held still
/// half a second would snap the stroke to a QuickShape. Returns the new device.
fn lost_and_made_again(runtime: &mut Runtime, hand: &mut Hand) -> Arc<openpaint_render::Core> {
    assert!(
        runtime.lose_device(),
        "Device::destroy reached wgpu's device-lost callback"
    );
    let notice = |runtime: &Runtime| runtime.controls().names().iter().any(|name| name == "OK");
    let start = Instant::now();
    loop {
        hand.frame_now(runtime);
        if runtime.core().lost().is_none() && !runtime.canvas_filling() && notice(runtime) {
            break;
        }
        assert!(start.elapsed() < HANG, "never made again");
    }
    assert!(
        shows_cold(runtime),
        "the artwork and the stroke so far, as cold"
    );
    runtime.core()
}

#[test]
fn a_lost_device_comes_back_with_the_canvas_history_and_the_stroke_being_drawn() {
    let (mut runtime, mut hand) =
        launch::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    for y in [300.0, 440.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    let first = runtime.core();
    assert_cold(&first, &mut runtime, "before the loss");
    let before = runtime.read_canvas().expect("read").expect("canvas");
    // The pen is down and drawing when the device goes.
    let points = line(580.0);
    pen(&mut runtime, &mut hand, MotionPhase::Down, points[0]);
    for at in &points[1..10] {
        pen(&mut runtime, &mut hand, MotionPhase::Move, *at);
    }
    let second = lost_and_made_again(&mut runtime, &mut hand);
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
    assert_cold(&second, &mut runtime, "the stroke committed");
    // Undo takes it back: the canvas as before the loss.
    hand.tap(&mut runtime, "Undo");
    settle(&mut runtime, &mut hand);
    assert_eq!(strokes(&runtime), 2);
    let undone = runtime.read_canvas().expect("read").expect("canvas");
    assert_eq!(
        undone.content_hash(),
        before.content_hash(),
        "undo after the loss"
    );
    assert_cold(&second, &mut runtime, "after undo");
}
