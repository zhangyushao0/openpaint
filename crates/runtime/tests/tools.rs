//! GPU test of the eyedropper and the eraser through the whole frame loop: a tap with the
//! eyedropper on picks exactly the pixel the canvas readback holds there and draws nothing; the
//! Eraser button makes the pen take paint away, and the canvas always equals a cold evaluation.

mod hand;
mod support;

use hand::{Hand, assert_cold};
use openpaint_app::Tool;
use openpaint_core::Color;
use openpaint_render::{Core, RgbaImage};
use openpaint_runtime::{Runtime, ToolKind};
use support::Nowhere;

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The canvas starts under the top bar, one screen pixel per canvas pixel.
const BAR: f32 = 96.0;
/// The side bar's eyedropper, between its sliders (118 points long on this screen).
const EYEDROPPER: [f32; 2] = [80.0, 428.0];
/// Right to left from the top bar's right end: colour, Layers, Eraser, Brush.
const ERASER: [f32; 2] = [1296.0, 48.0];
const UNDO: [f32; 2] = [80.0, 808.0];

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
fn pixel(canvas: &RgbaImage, at: [f32; 2]) -> [u8; 4] {
    let [x, y] = [at[0] as u32, (at[1] - BAR) as u32];
    let index = ((y * canvas.size()[0] + x) * 4) as usize;
    let mut out = [0; 4];
    out.copy_from_slice(&canvas.pixels()[index..index + 4]);
    out
}

#[test]
fn the_eyedropper_picks_the_pixel_the_readback_holds() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(500.0));
    // On the line's antialiased edge: a grey, neither the ink nor the paper.
    let edge = [700.0, 504.5];
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    let [r, g, b, _] = pixel(&canvas, edge);
    assert!(0 < g && g < 255, "an edge pixel: {:?}", [r, g, b]);
    hand.tap(&mut runtime, EYEDROPPER);
    assert!(runtime.projection().brush().expect("toolbox").picking);
    hand.tap(&mut runtime, edge);
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
        pixel(&canvas, [700.0, 700.0]),
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
    assert_eq!(pixel(&canvas, crossing), [0, 0, 0, 255]);
    hand.tap(&mut runtime, ERASER);
    let toolbox = runtime.projection().brush().cloned().expect("toolbox");
    assert_eq!(toolbox.tool, Tool::Eraser);
    let down = [[780.0, 300.0], [780.0, 500.0], [780.0, 700.0]];
    hand.gesture(&mut runtime, ToolKind::Stylus, &down);
    assert_eq!(strokes(&runtime), 2);
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(pixel(&canvas, crossing), [255; 4], "erased to the paper");
    assert_cold(&core, &runtime, "after erasing");
    hand.tap(&mut runtime, UNDO);
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    assert_eq!(
        pixel(&canvas, crossing),
        [0, 0, 0, 255],
        "undo brings the paint back"
    );
    assert_cold(&core, &runtime, "after undo");
}
