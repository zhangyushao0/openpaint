//! GPU test of palm rejection through the whole frame loop: a hand on the glass while the pen
//! draws neither paints, nor taps the side bar's buttons, nor disturbs the stroke; a touch the
//! platform cancels never taps.

use crate::support::{Hand, Nowhere, start, strokes};
use openpaint_runtime::{MotionPhase, Runtime, ToolKind};

/// 640 x 400 points at density 2.
const SIZE: [u32; 2] = [1280, 800];
const PIXELS_PER_POINT: f32 = 2.0;
const PEN: i32 = 0;
const PALM: i32 = 1;

fn pen((runtime, hand): (&mut Runtime, &mut Hand), phase: MotionPhase, at: [f32; 2]) {
    hand.send(runtime, phase, &[(PEN, ToolKind::Stylus, at)]);
}

fn palm((runtime, hand): (&mut Runtime, &mut Hand), phase: MotionPhase, at: [f32; 2]) {
    hand.send(runtime, phase, &[(PALM, ToolKind::Finger, at)]);
}

/// A pen stroke along y = `y`, right of the side bar.
fn stroke((runtime, hand): (&mut Runtime, &mut Hand), y: f32) {
    pen((runtime, hand), MotionPhase::Down, [200.0, y]);
    pen((runtime, hand), MotionPhase::Move, [600.0, y]);
    pen((runtime, hand), MotionPhase::Up, [1000.0, y]);
}

/// The middle of the side bar's undo button, found by its name (D48).
fn undo(runtime: &Runtime) -> [f32; 2] {
    let control = runtime.controls().find("Undo").expect("Undo on screen");
    control.at([0.5, 0.5])
}

#[test]
fn a_hand_on_undo_while_the_pen_draws_taps_nothing() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    stroke((&mut runtime, &mut hand), 300.0);
    let undo = undo(&runtime);
    // The hand lands on undo during the stroke and lifts after the pen.
    pen((&mut runtime, &mut hand), MotionPhase::Down, [200.0, 500.0]);
    palm((&mut runtime, &mut hand), MotionPhase::Down, undo);
    pen((&mut runtime, &mut hand), MotionPhase::Move, [600.0, 500.0]);
    pen((&mut runtime, &mut hand), MotionPhase::Up, [1000.0, 500.0]);
    palm((&mut runtime, &mut hand), MotionPhase::Up, undo);
    assert_eq!(
        strokes(&runtime),
        2,
        "the stroke is whole and nothing was undone"
    );
}

#[test]
fn a_hand_resting_on_undo_before_the_pen_lands_is_dropped() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    stroke((&mut runtime, &mut hand), 300.0);
    let undo = undo(&runtime);
    palm((&mut runtime, &mut hand), MotionPhase::Down, undo);
    stroke((&mut runtime, &mut hand), 500.0);
    palm((&mut runtime, &mut hand), MotionPhase::Up, undo);
    assert_eq!(strokes(&runtime), 2, "lifting the hand did not undo");
    // A finger tap with the pen off the glass still undoes.
    palm((&mut runtime, &mut hand), MotionPhase::Down, undo);
    palm((&mut runtime, &mut hand), MotionPhase::Up, undo);
    assert_eq!(strokes(&runtime), 1);
}

#[test]
fn a_touch_the_platform_cancels_never_taps() {
    let (mut runtime, mut hand) = start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere::default()));
    stroke((&mut runtime, &mut hand), 300.0);
    // A finger and the hand on undo; the system judges the hand a palm and cancels it alone.
    let finger = (2, ToolKind::Finger, [1200.0, 760.0]);
    let resting = (PALM, ToolKind::Finger, undo(&runtime));
    let both = [resting, finger];
    let runtime = &mut runtime;
    hand.send(runtime, MotionPhase::Down, &[resting]);
    hand.send(runtime, MotionPhase::PointerDown { index: 1 }, &both);
    hand.send(runtime, MotionPhase::PointerCancel { index: 0 }, &both);
    hand.send(runtime, MotionPhase::Up, &[finger]);
    assert_eq!(strokes(runtime), 1, "the cancelled hand did not undo");
    // The whole gesture cancelled while the hand rests on undo.
    hand.send(runtime, MotionPhase::Down, &[resting]);
    hand.send(runtime, MotionPhase::Cancel, &[resting]);
    assert_eq!(strokes(runtime), 1);
}
