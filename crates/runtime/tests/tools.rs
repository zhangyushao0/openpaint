//! GPU test of the eyedropper and the eraser through the whole frame loop, at fit: a tap with
//! the eyedropper on picks exactly the pixel the canvas readback holds there and draws nothing; the
//! Eraser button makes the pen take paint away, and the canvas always equals a cold evaluation.

mod hand;
mod screen_map;
mod support;

use hand::{Hand, assert_cold};
use openpaint_app::Tool;
use openpaint_core::Color;
use openpaint_render::{Core, RgbaImage};
use openpaint_runtime::{Runtime, ToolKind};
use screen_map::{mapping, pixel_at};
use support::Nowhere;

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The side bar's eyedropper, between its sliders (118 points long on this screen).
const EYEDROPPER: [f32; 2] = [80.0, 438.0];
/// In the right island, left to right: Brush, Eraser, Layers, colour.
const ERASER: [f32; 2] = [1316.0, 66.0];
const UNDO: [f32; 2] = [80.0, 810.0];

fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=12u8)
        .map(|i| [300.0 + 80.0 * f32::from(i), y])
        .collect()
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

/// The canvas pixel under the screen point `at`.
fn pixel(runtime: &Runtime, canvas: &RgbaImage, at: [f32; 2]) -> [u8; 4] {
    let map = mapping(runtime, SIZE, PIXELS_PER_POINT);
    pixel_at(canvas, map.to_canvas(at))
}

#[test]
fn the_eyedropper_picks_the_pixel_the_readback_holds() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(500.0));
    // On the line's antialiased edge: a grey, neither the ink nor the paper, at a texel centre.
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    let map = mapping(&runtime, SIZE, PIXELS_PER_POINT);
    let [x, y] = map.to_canvas([700.0, 500.0]).map(|v| v.floor() + 0.5);
    let edge = (0..20u8)
        .map(|below| [x, y + f32::from(below)])
        .find(|at| (1..255).contains(&pixel_at(&canvas, *at)[1]))
        .expect("an antialiased edge below the line's middle");
    let [r, g, b, _] = pixel_at(&canvas, edge);
    hand.tap(&mut runtime, EYEDROPPER);
    assert!(runtime.projection().brush().expect("toolbox").picking);
    hand.tap(&mut runtime, map.to_screen(edge));
    let toolbox = runtime.projection().brush().cloned().expect("toolbox");
    let channel = |value: u8| f32::from(value) / 255.0;
    let picked = Color::rgb(channel(r), channel(g), channel(b)).expect("colour");
    assert_eq!(
        toolbox.color, picked,
        "the picked colour is the exported pixel"
    );
    assert_eq!(toolbox.recent.first(), Some(&picked));
    assert!(!toolbox.picking, "the eyedropper ends with the pick");
    assert_eq!(strokes(&runtime), 1, "picking draws nothing");
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(700.0));
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(
        pixel(&runtime, &canvas, [700.0, 700.0]),
        [r, g, b, 255],
        "it paints in that colour"
    );
}

#[test]
fn the_eraser_button_makes_the_pen_take_paint_away() {
    let core = Core::headless().expect("a GPU adapter is required for runtime tests");
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(500.0));
    let crossing = [780.0, 500.0];
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(pixel(&runtime, &canvas, crossing), [0, 0, 0, 255]);
    hand.tap(&mut runtime, ERASER);
    let toolbox = runtime.projection().brush().cloned().expect("toolbox");
    assert_eq!(toolbox.tool, Tool::Eraser);
    let down = [[780.0, 300.0], [780.0, 500.0], [780.0, 700.0]];
    hand.gesture(&mut runtime, ToolKind::Stylus, &down);
    assert_eq!(strokes(&runtime), 2);
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(
        pixel(&runtime, &canvas, crossing),
        [255; 4],
        "erased to the paper"
    );
    assert_cold(&core, &runtime, "after erasing");
    hand.tap(&mut runtime, UNDO);
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(
        pixel(&runtime, &canvas, crossing),
        [0, 0, 0, 255],
        "undo brings the paint back"
    );
    assert_cold(&core, &runtime, "after undo");
}
