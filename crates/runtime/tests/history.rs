//! GPU test of undo and redo through the whole frame loop: taps on the top bar's buttons move
//! through history, the canvas always equals a cold evaluation of the version shown, and the
//! bar never takes a stroke that started on the canvas or paints one that started on it.

mod hand;
mod support;

use hand::{Hand, assert_cold};
use openpaint_render::Core;
use openpaint_runtime::{Runtime, ToolKind};
use support::Nowhere;

const SIZE: [u32; 2] = [640, 400];
const PIXELS_PER_POINT: f32 = 2.0;
/// Centres of the undo and redo buttons at density 2 (the scenario's at 1.75, rescaled).
const UNDO: [f32; 2] = [263.0, 48.0];
const REDO: [f32; 2] = [375.0, 48.0];

fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [100.0 + 20.0 * f32::from(i), y])
        .collect()
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

#[test]
fn taps_move_through_history_and_the_canvas_stays_exact() {
    let core = Core::headless().expect("a GPU adapter is required for runtime tests");
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    for y in [150.0, 220.0, 290.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    let drawn = runtime.artwork().expect("artwork").clone();
    hand.tap(&mut runtime, UNDO);
    assert_eq!(strokes(&runtime), 2);
    assert_cold(&core, &runtime, "after one undo");
    for _ in 0..20 {
        for _ in 0..3 {
            hand.tap(&mut runtime, UNDO);
        }
        hand.tap(&mut runtime, UNDO);
        assert_eq!(strokes(&runtime), 0, "the grey undo does nothing");
        for _ in 0..3 {
            hand.tap(&mut runtime, REDO);
        }
    }
    let artwork = runtime.artwork().expect("artwork");
    assert!(
        artwork.same_version(&drawn),
        "20 round trips end where they began"
    );
    assert_cold(&core, &runtime, "after 20 round trips");
}

#[test]
fn the_bar_and_the_canvas_keep_their_own_contacts() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(200.0));
    // A stroke that starts on the canvas and lifts over undo stays a stroke.
    let up_to_undo = [[263.0, 300.0], [263.0, 200.0], [263.0, 120.0], UNDO];
    hand.gesture(&mut runtime, ToolKind::Stylus, &up_to_undo);
    assert_eq!(strokes(&runtime), 2, "the lift over undo did not undo");
    // A pen that starts on the bar and drags into the canvas paints nothing.
    let from_bar = [[560.0, 48.0], [560.0, 200.0], [500.0, 300.0]];
    hand.gesture(&mut runtime, ToolKind::Stylus, &from_bar);
    assert_eq!(strokes(&runtime), 2, "a drag from the bar painted");
    // The pen taps undo like a finger does.
    hand.gesture(&mut runtime, ToolKind::Stylus, &[UNDO, UNDO]);
    assert_eq!(strokes(&runtime), 1);
}
