//! GPU test of QuickShape and the Drawing Guide through the whole frame loop (S42): with the
//! radial guide on, every frame of a stroke shows exactly what a cold evaluation of the artwork
//! with its mirrored copies and the stroke so far gives; a circle held still snaps on the
//! screen as cold; Edit shape's handle dragged and undo stay cold too.

mod gpu;
mod hand;
mod screen_map;
mod support;

use hand::{Hand, assert_cold, cold};
use openpaint_core::Color;
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use screen_map::{mapping, pixel_at};
use support::Nowhere;

/// 700 x 800 points at density 2: the Actions panel fits.
const SIZE: [u32; 2] = [1400, 1600];
const PIXELS_PER_POINT: f32 = 2.0;

/// Panics unless the canvas equals a cold evaluation of what it shows now, the stroke being
/// drawn included.
fn assert_drawn_cold(core: &Core, runtime: &mut Runtime, when: &str) {
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    let (artwork, active) = runtime.drawn().expect("a canvas");
    let size = canvas.size();
    let mut target = CanvasTarget::new(core, size).expect("target");
    let request = FrameRequest {
        artwork: &artwork,
        active: active.as_ref(),
        background: Color::WHITE,
    };
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request, &[PixelRect::full(size)])
        .expect("evaluate");
    let expected = target.read(core).expect("readback");
    assert_eq!(canvas.content_hash(), expected.content_hash(), "{when}");
}

/// The pen through `points` (screen pixels), a batch each 16 ms apart from
/// `hand.clock` with the frames it asks for, lifting at the last; `each` looks after each.
fn pen(
    hand: &mut Hand,
    runtime: &mut Runtime,
    points: &[[f32; 2]],
    mut each: impl FnMut(&mut Runtime, usize),
) {
    for (index, at) in points.iter().enumerate() {
        let phase = match index {
            0 => MotionPhase::Down,
            i if i == points.len() - 1 => MotionPhase::Up,
            _ => MotionPhase::Move,
        };
        hand.clock += 16;
        let sample = PointerSample {
            position: *at,
            pressure: 0.6,
            tilt: 0.0,
            orientation: 0.0,
            time_ms: hand.clock,
        };
        let batch = MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool: ToolKind::Stylus,
                samples: vec![sample],
            }],
        };
        runtime.motion(&batch);
        for _ in 0..4 {
            if !runtime.frame_offscreen(SIZE).expect("frame") {
                break;
            }
        }
        each(runtime, index);
    }
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

#[test]
fn radial_copies_show_as_cold_while_drawn_and_commit_as_one_step() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    hand.tap(&mut runtime, "Actions");
    hand.tap(&mut runtime, "Radial");
    assert!(runtime.projection().guide().on);
    hand.tap(&mut runtime, "Actions");
    let wave: Vec<[f32; 2]> = (0..24)
        .map(|k| {
            [
                760.0 + 14.0 * k as f32,
                500.0 + 40.0 * (k as f32 * 0.5).sin(),
            ]
        })
        .collect();
    pen(&mut hand, &mut runtime, &wave, |runtime, index| {
        if index % 5 == 2 {
            assert_drawn_cold(&core, runtime, &format!("frame {index}"));
        }
    });
    assert_eq!(strokes(&runtime), 8, "the stroke and seven copies");
    assert_cold(&core, &mut runtime, "committed");
    hand.tap(&mut runtime, "Undo");
    assert_eq!(strokes(&runtime), 0, "one undo takes all eight");
    assert_cold(&core, &mut runtime, "undone");
}

#[test]
fn a_held_circle_snaps_and_its_edited_shape_stays_cold() {
    let core = gpu::core();
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut hand = Hand::new(SIZE);
    let mut circle: Vec<[f32; 2]> = (0..40)
        .map(|k| {
            let t = (k as f32 * 9.0).to_radians();
            [700.0 + 260.0 * t.cos(), 800.0 + 250.0 * t.sin()]
        })
        .collect();
    let end = *circle.last().expect("end");
    // Held still 40 frames of 16 ms at the end, then lifted.
    circle.extend(std::iter::repeat_n(end, 40));
    let held = circle.len() - 2;
    pen(&mut hand, &mut runtime, &circle, |runtime, index| {
        if index == held {
            assert_drawn_cold(&core, runtime, "snapped while held");
            let (_, active) = runtime.drawn().expect("canvas");
            let samples = active.expect("drawing").samples().len();
            assert!(samples > 60, "samples along the circle: {samples}");
        }
    });
    assert_eq!(strokes(&runtime), 1);
    assert_cold(&core, &mut runtime, "committed");
    hand.tap(&mut runtime, "Edit shape");
    let shape = runtime.projection().shape().cloned().expect("offered");
    assert!(shape.editing);
    let handle = mapping(&runtime, SIZE, PIXELS_PER_POINT).to_screen(shape.handles[0]);
    let to = [handle[0] + 120.0, handle[1]];
    hand.gesture(
        &mut runtime,
        ToolKind::Stylus,
        &[handle, [handle[0] + 60.0, handle[1]], to],
    );
    assert_eq!(
        strokes(&runtime),
        1,
        "the handle moved the shape, it drew nothing"
    );
    assert_cold(&core, &mut runtime, "edited");
    // The ellipse now passes where the handle was let go.
    let canvas = runtime.read_canvas().expect("readback").expect("canvas");
    let end = mapping(&runtime, SIZE, PIXELS_PER_POINT).to_canvas(to);
    assert!(pixel_at(&canvas, end)[0] < 96, "ink at the moved handle");
    hand.tap(&mut runtime, "Done");
    assert!(runtime.projection().shape().is_none());
    hand.tap(&mut runtime, "Undo");
    assert_eq!(
        strokes(&runtime),
        0,
        "drawing, snapping and editing: one step"
    );
    let blank = runtime.artwork().expect("artwork").clone();
    assert_eq!(
        runtime
            .read_canvas()
            .expect("readback")
            .expect("canvas")
            .content_hash(),
        cold(&core, &blank).content_hash()
    );
}
