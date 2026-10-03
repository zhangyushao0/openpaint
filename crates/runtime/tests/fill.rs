//! GPU test of ColorDrop through the whole frame loop, at fit: the colour dragged from the
//! colour button into a closed antialiased curve fills it up to the line with no light seam
//! under the line's soft inner edge (draweave 674c59b), leaves the outside alone, equals a cold
//! evaluation, and undo takes it back.

mod hand;
mod screen_map;
mod support;

use hand::{Hand, assert_cold};
use openpaint_render::{Core, RgbaImage};
use openpaint_runtime::{Runtime, ToolKind};
use screen_map::{mapping, pixel_at};
use support::Nowhere;

/// 800 x 500 points at density 2.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The colour button at the top bar's right end.
const COLOUR: [f32; 2] = [1520.0, 48.0];
const UNDO: [f32; 2] = [80.0, 808.0];
const CENTRE: [f32; 2] = [800.0, 520.0];

/// A closed circle of `radius` screen pixels about `CENTRE`.
fn circle(radius: f32) -> Vec<[f32; 2]> {
    (0..=72u16)
        .map(|i| {
            let angle = std::f32::consts::TAU * f32::from(i) / 72.0;
            [
                CENTRE[0] + radius * angle.cos(),
                CENTRE[1] + radius * angle.sin(),
            ]
        })
        .collect()
}

/// Frames until the fill worker (and the store) are done.
fn settle(runtime: &mut Runtime) {
    let start = std::time::Instant::now();
    while runtime.frame_offscreen(SIZE).expect("frame") || runtime.storing() {
        assert!(start.elapsed().as_secs() < 20, "the fill did not finish");
    }
}

fn canvas(runtime: &Runtime) -> RgbaImage {
    runtime.read_canvas().expect("readback").expect("canvas")
}

fn fills(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork
        .layers()
        .iter()
        .map(|layer| layer.edits().len())
        .sum()
}

#[test]
fn a_colour_dropped_into_a_closed_curve_fills_it_without_a_seam() {
    let core = Core::headless().expect("a GPU adapter is required for runtime tests");
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &circle(260.0));
    let drop = [COLOUR, [1300.0, 300.0], [1000.0, 450.0], CENTRE, CENTRE];
    hand.gesture(&mut runtime, ToolKind::Finger, &drop);
    settle(&mut runtime);
    assert_eq!(fills(&runtime), 1, "one fill, committed");
    let strokes: usize = runtime
        .artwork()
        .map(|a| a.layers().iter().map(|l| l.strokes().len()).sum())
        .unwrap_or(0);
    assert_eq!(strokes, 1, "the drop drew nothing");
    let image = canvas(&runtime);
    let map = mapping(&runtime, SIZE, PIXELS_PER_POINT);
    let centre = map.to_canvas(CENTRE);
    let black = [0, 0, 0, 255];
    // From the centre outwards (canvas pixels): black (the fill, then under and on the line),
    // then the line's outer soft edge, then the paper; never paper inside.
    let at = |r: f32| pixel_at(&image, [centre[0] + r, centre[1]]);
    let line = map.to_canvas([CENTRE[0] + 260.0, CENTRE[1]])[0] - centre[0];
    for r in (0..line as u32).map(|r| r as f32) {
        assert_eq!(
            at(r),
            black,
            "inside the curve at {r} px: filled to the line"
        );
    }
    assert_eq!(at(line + 30.0), [255; 4], "outside stays paper");
    assert_cold(&core, &runtime, "after the fill");
    hand.tap(&mut runtime, UNDO);
    settle(&mut runtime);
    assert_eq!(fills(&runtime), 0);
    assert_eq!(
        pixel_at(&canvas(&runtime), centre),
        [255; 4],
        "undo takes it back"
    );
    assert_cold(&core, &runtime, "after undo");
}
