//! GPU test of the walk-through through the whole frame loop: the pen draws under its tip, a
//! finger does not draw, and the screen shows exactly what a cold evaluation of the committed
//! artwork gives.

mod support;

use openpaint_core::{Artwork, Color};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use support::Nowhere;

const SIZE: [u32; 2] = [640, 400];
const PIXELS_PER_POINT: f32 = 2.0;
/// The 48-point top bar at density 2.
const BAR_ROWS: u32 = 96;
/// The side bar over the canvas's left edge (8 + 64 points) at density 2, and some room.
const SIDE_BAR_COLUMNS: u32 = 160;
const PAPER: [u8; 4] = [255; 4];

fn sample(x: f32, y: f32, time_ms: i64) -> PointerSample {
    PointerSample {
        position: [x, y],
        pressure: 0.8,
        tilt: 0.4,
        orientation: 0.3,
        time_ms,
    }
}

/// A straight swipe from x = 180 to x = 580 at screen row `y`, right of the side bar: down,
/// moves of three samples, up; one frame after each batch.
fn swipe(runtime: &mut Runtime, tool: ToolKind, y: f32) {
    let points: Vec<PointerSample> = (0..=40u8)
        .map(|i| sample(180.0 + 10.0 * f32::from(i), y, i64::from(i) * 4))
        .collect();
    let mut batches = vec![(MotionPhase::Down, points[..1].to_vec())];
    batches.extend(
        points[1..40]
            .chunks(3)
            .map(|c| (MotionPhase::Move, c.to_vec())),
    );
    batches.push((MotionPhase::Up, points[40..].to_vec()));
    for (phase, samples) in batches {
        let batch = MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples,
            }],
        };
        assert!(runtime.motion(&batch));
        runtime.frame_offscreen(SIZE).expect("frame");
    }
}

fn pixel(image: &RgbaImage, [x, y]: [u32; 2]) -> [u8; 4] {
    let at = ((y * image.size()[0] + x) * 4) as usize;
    let mut out = [0; 4];
    out.copy_from_slice(&image.pixels()[at..at + 4]);
    out
}

fn cold(artwork: &Artwork) -> RgbaImage {
    let core = Core::headless().expect("GPU");
    let canvas = artwork.canvas();
    let size = [canvas.width(), canvas.height()];
    let mut target = CanvasTarget::new(&core, size).expect("target");
    let request = FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request, &[PixelRect::full(size)])
        .expect("evaluate");
    target.read(&core).expect("readback")
}

#[test]
fn the_pen_draws_under_its_tip_and_a_finger_does_not() {
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    swipe(&mut runtime, ToolKind::Stylus, 200.0);
    swipe(&mut runtime, ToolKind::Finger, 320.0);
    let screen = runtime.screenshot(SIZE).expect("screenshot");
    assert_eq!(
        pixel(&screen, [300, 200]),
        [0, 0, 0, 255],
        "ink under the pen"
    );
    assert_eq!(pixel(&screen, [300, 214]), PAPER, "the brush is 12 px wide");
    assert_eq!(pixel(&screen, [300, 320]), PAPER, "the finger left no ink");
    let artwork = runtime.artwork().expect("artwork").clone();
    let strokes: usize = artwork.layers().iter().map(|l| l.strokes().len()).sum();
    assert_eq!(strokes, 1);
    let cold = cold(&artwork);
    let [width, height] = cold.size();
    let rows = (0..height).all(|y| {
        (SIDE_BAR_COLUMNS..width).all(|x| pixel(&screen, [x, y + BAR_ROWS]) == pixel(&cold, [x, y]))
    });
    assert!(
        rows,
        "the screen differs from a cold evaluation of the artwork"
    );
}
