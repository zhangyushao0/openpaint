//! GPU test of the layer panel through the whole frame loop: the Layers button opens it, its
//! buttons, eyes, slider and Alpha Lock switch change the layers, undo takes each change back, the canvas always
//! equals a cold evaluation, a pen starting on the panel never paints and a tap outside closes
//! it.

mod hand;
mod support;

use hand::{Hand, assert_cold};
use openpaint_render::Core;
use openpaint_runtime::{Runtime, ToolKind};
use support::Nowhere;

/// 800 x 700 points at density 2: the whole panel fits.
const SIZE: [u32; 2] = [1600, 1400];
const PIXELS_PER_POINT: f32 = 2.0;
/// Left of the colour button in the right island, in pixels.
const LAYERS: [f32; 2] = [1420.0, 66.0];
/// The panel's add button, in pixels (the second test needs the spot while the panel is shut).
const ADD: [f32; 2] = [1520.0, 176.0];

/// The point `fraction` across the control `name` of the last frame, in pixels.
fn on(runtime: &Runtime, name: &str, fraction: [f32; 2]) -> [f32; 2] {
    let control = runtime.controls().find(name);
    control
        .unwrap_or_else(|_| panic!("{name} on screen"))
        .at(fraction)
}

fn tap(hand: &mut Hand, runtime: &mut Runtime, name: &str) {
    let at = on(runtime, name, [0.5, 0.5]);
    hand.tap(runtime, at);
}

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

/// Right of the side bar.
fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=12u8)
        .map(|i| [200.0 + 40.0 * f32::from(i), y])
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
    tap(&mut hand, &mut runtime, "Add layer");
    assert_eq!(layers(&runtime), [(1, true, 1.0), (2, true, 1.0)]);
    tap(&mut hand, &mut runtime, "Hide layer #1");
    assert_eq!(layers(&runtime)[1], (2, false, 1.0));
    tap(&mut hand, &mut runtime, "Show layer");
    let slide = [0.97, 0.75, 0.5].map(|x| on(&runtime, "Opacity", [x, 0.5]));
    hand.gesture(&mut runtime, ToolKind::Finger, &slide);
    let (_, _, opacity) = layers(&runtime)[1];
    assert!((opacity - 0.5).abs() < 0.01, "opacity {opacity}");
    tap(&mut hand, &mut runtime, "Alpha Lock");
    let artwork = runtime.artwork().expect("artwork");
    assert!(
        artwork.layers()[1].props().alpha_locked(),
        "the switch locks"
    );
    tap(&mut hand, &mut runtime, "Layer 1");
    tap(&mut hand, &mut runtime, "Move layer up");
    assert_eq!(layers(&runtime)[1].0, 1, "layer 1 is on top");
    tap(&mut hand, &mut runtime, "Delete layer");
    assert_eq!(layers(&runtime).len(), 1);
    assert_cold(&core, &runtime, "after the panel actions");
    // Delete, raise, Alpha Lock, the whole slide, show, hide, add: seven steps back.
    for step in 0..7 {
        tap(&mut hand, &mut runtime, "Undo");
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
