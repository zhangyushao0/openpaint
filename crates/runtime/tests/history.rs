//! GPU test of undo and redo through the whole frame loop: taps on the top bar's buttons move
//! through history, the canvas always equals a cold evaluation of the version shown, and the
//! bar never takes a stroke that started on the canvas or paints one that started on it.

mod support;

use openpaint_core::{Artwork, Color};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use support::Nowhere;

const SIZE: [u32; 2] = [640, 400];
const PIXELS_PER_POINT: f32 = 2.0;
/// Centres of the undo and redo buttons at density 2 (the scenario's at 1.75, rescaled).
const UNDO: [f32; 2] = [263.0, 48.0];
const REDO: [f32; 2] = [375.0, 48.0];

/// One pointer moving through `points`: down at the first, a move per point, up at the last;
/// one frame after each batch.
fn gesture(runtime: &mut Runtime, tool: ToolKind, points: &[[f32; 2]], clock: &mut i64) {
    let last = points.len() - 1;
    for (index, position) in points.iter().enumerate() {
        let phase = match index {
            0 => MotionPhase::Down,
            i if i == last => MotionPhase::Up,
            _ => MotionPhase::Move,
        };
        *clock += 8;
        let sample = PointerSample {
            position: *position,
            pressure: 0.8,
            tilt: 0.0,
            orientation: 0.0,
            time_ms: *clock,
        };
        let batch = MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples: vec![sample],
            }],
        };
        runtime.motion(&batch);
        // The frame, then the ones the runtime asks for, as the host paces them.
        for _ in 0..4 {
            if !runtime.frame_offscreen(SIZE).expect("frame") {
                break;
            }
        }
    }
}

fn tap(runtime: &mut Runtime, at: [f32; 2], clock: &mut i64) {
    gesture(runtime, ToolKind::Finger, &[at, at], clock);
}

fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [100.0 + 20.0 * f32::from(i), y])
        .collect()
}

fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    let canvas = artwork.canvas();
    let size = [canvas.width(), canvas.height()];
    let mut target = CanvasTarget::new(core, size).expect("target");
    let request = FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request, &[PixelRect::full(size)])
        .expect("evaluate");
    target.read(core).expect("readback")
}

fn assert_cold(core: &Core, runtime: &Runtime, when: &str) {
    let shown = runtime.read_canvas().expect("readback").expect("canvas");
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(
        shown.content_hash(),
        cold(core, artwork).content_hash(),
        "{when}: the canvas differs from a cold evaluation"
    );
}

#[test]
fn taps_move_through_history_and_the_canvas_stays_exact() {
    let core = Core::headless().expect("a GPU adapter is required for runtime tests");
    let mut runtime = support::start(PIXELS_PER_POINT, SIZE, Box::new(Nowhere));
    let mut clock = 0;
    for y in [150.0, 220.0, 290.0] {
        gesture(&mut runtime, ToolKind::Stylus, &line(y), &mut clock);
    }
    let drawn = runtime.artwork().expect("artwork").clone();
    tap(&mut runtime, UNDO, &mut clock);
    assert_eq!(strokes(&runtime), 2);
    assert_cold(&core, &runtime, "after one undo");
    for _ in 0..20 {
        for _ in 0..3 {
            tap(&mut runtime, UNDO, &mut clock);
        }
        tap(&mut runtime, UNDO, &mut clock);
        assert_eq!(strokes(&runtime), 0, "the grey undo does nothing");
        for _ in 0..3 {
            tap(&mut runtime, REDO, &mut clock);
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
    let mut clock = 0;
    gesture(&mut runtime, ToolKind::Stylus, &line(200.0), &mut clock);
    // A stroke that starts on the canvas and lifts over undo stays a stroke.
    let up_to_undo = [[263.0, 300.0], [263.0, 200.0], [263.0, 120.0], UNDO];
    gesture(&mut runtime, ToolKind::Stylus, &up_to_undo, &mut clock);
    assert_eq!(strokes(&runtime), 2, "the lift over undo did not undo");
    // A pen that starts on the bar and drags into the canvas paints nothing.
    let from_bar = [[560.0, 48.0], [560.0, 200.0], [500.0, 300.0]];
    gesture(&mut runtime, ToolKind::Stylus, &from_bar, &mut clock);
    assert_eq!(strokes(&runtime), 2, "a drag from the bar painted");
    // The pen taps undo like a finger does.
    gesture(&mut runtime, ToolKind::Stylus, &[UNDO, UNDO], &mut clock);
    assert_eq!(strokes(&runtime), 1);
}
