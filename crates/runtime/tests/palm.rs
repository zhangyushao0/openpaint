//! GPU test of palm rejection through the whole frame loop: a hand on the glass while the pen
//! draws neither paints, nor taps the side bar's buttons, nor disturbs the stroke; a touch the
//! platform cancels never taps.

mod gpu;
mod support;

use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use support::Nowhere;

/// 640 x 400 points at density 2.
const SIZE: [u32; 2] = [1280, 800];
const PIXELS_PER_POINT: f32 = 2.0;
const PEN: i32 = 0;
const PALM: i32 = 1;

/// One event of the given pointers, then the frames the runtime asks for.
fn send(runtime: &mut Runtime, phase: MotionPhase, pointers: &[(i32, ToolKind, [f32; 2])]) {
    let pointers = pointers
        .iter()
        .map(|&(id, tool, position)| Pointer {
            id,
            tool,
            samples: vec![PointerSample {
                position,
                pressure: 0.8,
                tilt: 0.0,
                orientation: 0.0,
                time_ms: 0,
            }],
        })
        .collect();
    runtime.motion(&MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers,
    });
    for _ in 0..4 {
        if !runtime.frame_offscreen(SIZE).expect("frame") {
            break;
        }
    }
}

fn pen(runtime: &mut Runtime, phase: MotionPhase, at: [f32; 2]) {
    send(runtime, phase, &[(PEN, ToolKind::Stylus, at)]);
}

fn palm(runtime: &mut Runtime, phase: MotionPhase, at: [f32; 2]) {
    send(runtime, phase, &[(PALM, ToolKind::Finger, at)]);
}

/// A pen stroke along y = `y`, right of the side bar.
fn stroke(runtime: &mut Runtime, y: f32) {
    pen(runtime, MotionPhase::Down, [200.0, y]);
    pen(runtime, MotionPhase::Move, [600.0, y]);
    pen(runtime, MotionPhase::Up, [1000.0, y]);
}

/// The middle of the side bar's undo button, found by its name (D48).
fn undo(runtime: &Runtime) -> [f32; 2] {
    let control = runtime.controls().find("Undo").expect("Undo on screen");
    control.at([0.5, 0.5])
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

#[test]
fn a_hand_on_undo_while_the_pen_draws_taps_nothing() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    stroke(&mut runtime, 300.0);
    let undo = undo(&runtime);
    // The hand lands on undo during the stroke and lifts after the pen.
    pen(&mut runtime, MotionPhase::Down, [200.0, 500.0]);
    palm(&mut runtime, MotionPhase::Down, undo);
    pen(&mut runtime, MotionPhase::Move, [600.0, 500.0]);
    pen(&mut runtime, MotionPhase::Up, [1000.0, 500.0]);
    palm(&mut runtime, MotionPhase::Up, undo);
    assert_eq!(
        strokes(&runtime),
        2,
        "the stroke is whole and nothing was undone"
    );
}

#[test]
fn a_hand_resting_on_undo_before_the_pen_lands_is_dropped() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    stroke(&mut runtime, 300.0);
    let undo = undo(&runtime);
    palm(&mut runtime, MotionPhase::Down, undo);
    stroke(&mut runtime, 500.0);
    palm(&mut runtime, MotionPhase::Up, undo);
    assert_eq!(strokes(&runtime), 2, "lifting the hand did not undo");
    // A finger tap with the pen off the glass still undoes.
    palm(&mut runtime, MotionPhase::Down, undo);
    palm(&mut runtime, MotionPhase::Up, undo);
    assert_eq!(strokes(&runtime), 1);
}

#[test]
fn a_touch_the_platform_cancels_never_taps() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    stroke(&mut runtime, 300.0);
    // A finger and the hand on undo; the system judges the hand a palm and cancels it alone.
    let finger = (2, ToolKind::Finger, [1200.0, 760.0]);
    let hand = (PALM, ToolKind::Finger, undo(&runtime));
    send(&mut runtime, MotionPhase::Down, &[hand]);
    send(
        &mut runtime,
        MotionPhase::PointerDown { index: 1 },
        &[hand, finger],
    );
    send(
        &mut runtime,
        MotionPhase::PointerCancel { index: 0 },
        &[hand, finger],
    );
    send(&mut runtime, MotionPhase::Up, &[finger]);
    assert_eq!(strokes(&runtime), 1, "the cancelled hand did not undo");
    // The whole gesture cancelled while the hand rests on undo.
    send(&mut runtime, MotionPhase::Down, &[hand]);
    send(&mut runtime, MotionPhase::Cancel, &[hand]);
    assert_eq!(strokes(&runtime), 1);
}
