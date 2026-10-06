//! GPU test of the layer panel through the whole frame loop: the Layers button opens it, its
//! buttons, eyes, slider and Alpha Lock switch change the layers, undo takes each change back, the canvas always
//! equals a cold evaluation, a pen starting on the panel never paints and a tap outside closes
//! it.

mod gpu;
mod hand;
mod support;

use hand::{Hand, assert_cold, on};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, TextInput, ToolKind,
};
use support::Nowhere;

/// 800 x 700 points at density 2: the whole panel fits.
const SIZE: [u32; 2] = [1600, 1400];
const PIXELS_PER_POINT: f32 = 2.0;

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

/// A finger pressed on `from` and held still for half a second (a layer row lifts), then
/// moved straight to `to` and lifted, on the runtime's clock fixed to the gesture's (S38).
fn hold_and_drag(hand: &mut Hand, runtime: &mut Runtime, from: [f32; 2], to: [f32; 2]) {
    let held = std::iter::repeat_n(from, 32);
    let moved = (1..=10u8).map(|step| {
        let t = f32::from(step) / 10.0;
        [
            from[0] + (to[0] - from[0]) * t,
            from[1] + (to[1] - from[1]) * t,
        ]
    });
    let points: Vec<[f32; 2]> = held.chain(moved).chain([to]).collect();
    let last = points.len() - 1;
    for (index, position) in points.into_iter().enumerate() {
        let phase = match index {
            0 => MotionPhase::Down,
            i if i == last => MotionPhase::Up,
            _ => MotionPhase::Move,
        };
        hand.clock += 16;
        runtime.fix_clock(hand.clock);
        let sample = PointerSample {
            position,
            pressure: 1.0,
            tilt: 0.0,
            orientation: 0.0,
            time_ms: hand.clock,
        };
        runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool: ToolKind::Finger,
                samples: vec![sample],
            }],
        });
        runtime.frame_offscreen(hand.size).expect("frame");
    }
    runtime.frame_offscreen(hand.size).expect("frame");
}

/// Right of the side bar.
fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=12u8)
        .map(|i| [200.0 + 40.0 * f32::from(i), y])
        .collect()
}

#[test]
fn panel_actions_change_the_layers_and_undo_takes_each_back() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(500.0));
    let start = runtime.artwork().expect("artwork").clone();
    hand.tap(&mut runtime, "Layers");
    hand.tap(&mut runtime, "Add layer");
    assert_eq!(layers(&runtime), [(1, true, 1.0), (2, true, 1.0)]);
    hand.tap(&mut runtime, "Hide layer #1");
    assert_eq!(layers(&runtime)[1], (2, false, 1.0));
    hand.tap(&mut runtime, "Show layer");
    let slide = [0.97, 0.75, 0.5].map(|x| on(&runtime, "Opacity", [x, 0.5]));
    hand.gesture(&mut runtime, ToolKind::Finger, &slide);
    let (_, _, opacity) = layers(&runtime)[1];
    assert!((opacity - 0.5).abs() < 0.01, "opacity {opacity}");
    hand.tap(&mut runtime, "Alpha Lock");
    let artwork = runtime.artwork().expect("artwork");
    assert!(
        artwork.layers()[1].props().alpha_locked(),
        "the switch locks"
    );
    hand.tap(&mut runtime, "Layer 1");
    // Held still, Layer 1's row lifts; let go over Layer 2's row, it lands there: on top.
    let (from, to) = (
        on(&runtime, "Layer 1", [0.5, 0.5]),
        on(&runtime, "Layer 2", [0.5, 0.3]),
    );
    hold_and_drag(&mut hand, &mut runtime, from, to);
    assert_eq!(layers(&runtime)[1].0, 1, "layer 1 is on top");
    assert_cold(&core, &runtime, "after the move");
    hand.tap(&mut runtime, "Duplicate layer");
    assert_eq!(layers(&runtime).len(), 3);
    assert_cold(&core, &runtime, "after duplicating");
    hand.tap(&mut runtime, "Rename");
    runtime.text(TextInput::Commit("Sky".into()));
    runtime.frame_offscreen(SIZE).expect("frame");
    hand.tap(&mut runtime, "Done");
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(
        artwork.layers()[2].name().as_str(),
        "Sky",
        "the copy is renamed"
    );
    hand.tap(&mut runtime, "Delete layer");
    assert_eq!(layers(&runtime).len(), 2);
    assert_cold(&core, &runtime, "after the panel actions");
    // Delete, rename, duplicate, move, Alpha Lock, the slide, show, hide, add: nine back.
    for step in 0..9 {
        hand.tap(&mut runtime, "Undo");
        assert_cold(&core, &runtime, &format!("after undo {step}"));
    }
    assert!(runtime.artwork().expect("artwork").same_version(&start));
}

#[test]
fn merge_down_keeps_the_canvas_and_undo_takes_it_back() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(500.0));
    hand.tap(&mut runtime, "Layers");
    hand.tap(&mut runtime, "Add layer");
    hand.tap(&mut runtime, "Clipping Mask");
    hand.tap(&mut runtime, "Multiply");
    // A stroke on the canvas draws on the new layer and closes the panel.
    let across = [[300.0, 480.0], [500.0, 520.0], [700.0, 480.0]];
    hand.gesture(&mut runtime, ToolKind::Stylus, &across);
    let shown = |runtime: &Runtime| {
        let image = runtime.read_canvas().expect("readback").expect("canvas");
        image.content_hash()
    };
    let before = shown(&runtime);
    hand.tap(&mut runtime, "Layers");
    hand.tap(&mut runtime, "Merge Down");
    assert_eq!(layers(&runtime).len(), 1, "merged");
    assert_eq!(shown(&runtime), before, "the canvas as it was");
    assert_cold(&core, &runtime, "after the merge");
    hand.tap(&mut runtime, "Undo");
    assert_eq!(layers(&runtime).len(), 2, "undone");
    assert_eq!(shown(&runtime), before);
    assert_cold(&core, &runtime, "after undo");
}

#[test]
fn the_pen_never_paints_through_the_panel_and_a_tap_outside_closes_it() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.tap(&mut runtime, "Layers");
    // Where the panel's add button lies while it is open; shut, that spot is canvas.
    let add = on(&runtime, "Add layer", [0.5, 0.5]);
    let across_panel = [[1100.0, 300.0], [1300.0, 400.0], [700.0, 500.0]];
    hand.gesture(&mut runtime, ToolKind::Stylus, &across_panel);
    assert_eq!(strokes(&runtime), 0, "a stroke from the panel painted");
    hand.tap(&mut runtime, [400.0, 800.0]);
    assert_eq!(strokes(&runtime), 0, "the finger painted");
    // The panel is closed: the spot of its add button is canvas again.
    hand.tap(&mut runtime, add);
    assert_eq!(layers(&runtime).len(), 1);
    hand.tap(&mut runtime, "Layers");
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(800.0));
    assert_eq!(strokes(&runtime), 1, "a stroke outside the panel draws");
    hand.tap(&mut runtime, add);
    assert_eq!(layers(&runtime).len(), 1, "and closes the panel");
}
