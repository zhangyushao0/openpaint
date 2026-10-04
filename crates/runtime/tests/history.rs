//! GPU test of undo and redo through the whole frame loop: taps on the side bar's buttons move
//! through history, the canvas always equals a cold evaluation of the version shown, and the
//! bars never take a stroke that started on the canvas or paint one that started on them.

mod gpu;
mod hand;
mod support;

use hand::{Hand, assert_cold, on};
use openpaint_runtime::{Runtime, ToolKind};
use support::Nowhere;

/// 640 x 500 points at density 2: the side bar's sliders shortened to fit (the bar starts
/// under the top islands; the screen is too short to centre it).
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

#[test]
fn taps_move_through_history_and_the_canvas_stays_exact() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    for y in [300.0, 440.0, 580.0] {
        hand.gesture(&mut runtime, ToolKind::Stylus, &line(y));
    }
    let drawn = runtime.artwork().expect("artwork").clone();
    hand.tap(&mut runtime, "Undo");
    assert_eq!(strokes(&runtime), 2);
    assert_cold(&core, &runtime, "after one undo");
    for _ in 0..20 {
        for _ in 0..3 {
            hand.tap(&mut runtime, "Undo");
        }
        hand.tap(&mut runtime, "Undo");
        assert_eq!(strokes(&runtime), 0, "the grey undo does nothing");
        for _ in 0..3 {
            hand.tap(&mut runtime, "Redo");
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
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(400.0));
    let undo = on(&runtime, "Undo", [0.5, 0.5]);
    // A stroke that starts on the canvas and lifts over undo stays a stroke.
    let up_to_undo = [[400.0, 700.0], [300.0, 660.0], [200.0, 630.0], undo];
    hand.gesture(&mut runtime, ToolKind::Stylus, &up_to_undo);
    assert_eq!(strokes(&runtime), 2, "the lift over undo did not undo");
    // A pen that starts on a bar and drags into the canvas paints nothing.
    for from in [[700.0, 48.0], [80.0, 300.0]] {
        let from_bar = [from, [560.0, 400.0], [700.0, 500.0]];
        hand.gesture(&mut runtime, ToolKind::Stylus, &from_bar);
        assert_eq!(strokes(&runtime), 2, "a drag from {from:?} painted");
    }
    // The pen taps undo like a finger does.
    hand.gesture(&mut runtime, ToolKind::Stylus, &[undo, undo]);
    assert_eq!(strokes(&runtime), 1);
}
