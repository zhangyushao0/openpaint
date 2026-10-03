//! GPU test of the layer panel through the whole frame loop: the Layers button opens it, its
//! buttons, eyes and slider change the layers, undo takes each change back, the canvas always
//! equals a cold evaluation, a pen starting on the panel never paints and a tap outside closes
//! it.

mod hand;
mod support;

use hand::{Hand, assert_cold};
use openpaint_render::Core;
use openpaint_runtime::{Runtime, ToolKind};
use support::Nowhere;

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// Button centres in pixels: points from the panel's layout (popover 320 points wide, one
/// space from the right edge and under the top bar; rows, slider and buttons one touch target
/// high), times 2.
const UNDO: [f32; 2] = [263.0, 48.0];
const LAYERS: [f32; 2] = [1520.0, 48.0];
const ADD: [f32; 2] = [1520.0, 176.0];
/// The eye of the first row (the top layer).
const TOP_EYE: [f32; 2] = [1520.0, 272.0];
/// The name of the second row, with two layers.
const SECOND_ROW: [f32; 2] = [1100.0, 368.0];
/// The slider's track runs from x 984 to 1544, with two layers.
const SLIDER_Y: f32 = 572.0;
const RAISE: [f32; 2] = [1008.0, 668.0];
const DELETE: [f32; 2] = [1520.0, 668.0];

fn layers(runtime: &Runtime) -> Vec<(u64, bool, f32)> {
    let artwork = runtime.artwork().expect("artwork");
    let layers = artwork.layers().iter();
    layers
        .map(|l| (l.id().get(), l.props().visible(), l.props().opacity()))
        .collect()
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=12u8)
        .map(|i| [100.0 + 40.0 * f32::from(i), y])
        .collect()
}

#[test]
fn panel_actions_change_the_layers_and_undo_takes_each_back() {
    let core = Core::headless().expect("a GPU adapter is required for runtime tests");
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(500.0));
    let start = runtime.artwork().expect("artwork").clone();
    hand.tap(&mut runtime, LAYERS);
    hand.tap(&mut runtime, ADD);
    assert_eq!(layers(&runtime), [(1, true, 1.0), (2, true, 1.0)]);
    hand.tap(&mut runtime, TOP_EYE);
    assert_eq!(layers(&runtime)[1], (2, false, 1.0));
    hand.tap(&mut runtime, TOP_EYE);
    let slide = [[1540.0, SLIDER_Y], [1400.0, SLIDER_Y], [1264.0, SLIDER_Y]];
    hand.gesture(&mut runtime, ToolKind::Finger, &slide);
    let (_, _, opacity) = layers(&runtime)[1];
    assert!((opacity - 0.5).abs() < 0.01, "opacity {opacity}");
    hand.tap(&mut runtime, SECOND_ROW);
    hand.tap(&mut runtime, RAISE);
    assert_eq!(layers(&runtime)[1].0, 1, "layer 1 is on top");
    hand.tap(&mut runtime, DELETE);
    assert_eq!(layers(&runtime).len(), 1);
    assert_cold(&core, &runtime, "after the panel actions");
    // Delete, raise, the whole slide, show, hide, add: six steps back.
    for step in 0..6 {
        hand.tap(&mut runtime, UNDO);
        assert_cold(&core, &runtime, &format!("after undo {step}"));
    }
    assert!(runtime.artwork().expect("artwork").same_version(&start));
}

#[test]
fn the_pen_never_paints_through_the_panel_and_a_tap_outside_closes_it() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.tap(&mut runtime, LAYERS);
    let across_panel = [[1100.0, 300.0], [1300.0, 400.0], [700.0, 500.0]];
    hand.gesture(&mut runtime, ToolKind::Stylus, &across_panel);
    assert_eq!(strokes(&runtime), 0, "a stroke from the panel painted");
    hand.tap(&mut runtime, [400.0, 800.0]);
    assert_eq!(strokes(&runtime), 0, "the finger painted");
    // The panel is closed: the spot of its add button is canvas again.
    hand.tap(&mut runtime, ADD);
    assert_eq!(layers(&runtime).len(), 1);
    hand.tap(&mut runtime, LAYERS);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(800.0));
    assert_eq!(strokes(&runtime), 1, "a stroke outside the panel draws");
    hand.tap(&mut runtime, ADD);
    assert_eq!(layers(&runtime).len(), 1, "and closes the panel");
}
