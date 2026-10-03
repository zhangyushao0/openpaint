//! Shared by the runtime's GPU tests that tap and draw: gestures through the frame loop, and the
//! cold evaluation the canvas must equal.

use openpaint_core::{Artwork, Color};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};

/// A screen of `size` pixels and the input clock of the gestures on it.
pub struct Hand {
    pub size: [u32; 2],
    pub clock: i64,
}

impl Hand {
    pub fn new(size: [u32; 2]) -> Self {
        Self { size, clock: 0 }
    }

    /// One pointer moving through `points`: down at the first, a move per point, up at the
    /// last; one frame after each batch, then the ones the runtime asks for, as the host paces
    /// them.
    pub fn gesture(&mut self, runtime: &mut Runtime, tool: ToolKind, points: &[[f32; 2]]) {
        let last = points.len() - 1;
        for (index, position) in points.iter().enumerate() {
            let phase = match index {
                0 => MotionPhase::Down,
                i if i == last => MotionPhase::Up,
                _ => MotionPhase::Move,
            };
            self.clock += 8;
            let sample = PointerSample {
                position: *position,
                pressure: 0.8,
                tilt: 0.0,
                orientation: 0.0,
                time_ms: self.clock,
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
            for _ in 0..4 {
                if !runtime.frame_offscreen(self.size).expect("frame") {
                    break;
                }
            }
        }
    }

    /// A finger tap at `at`.
    pub fn tap(&mut self, runtime: &mut Runtime, at: [f32; 2]) {
        self.gesture(runtime, ToolKind::Finger, &[at, at]);
    }
}

/// `artwork` evaluated whole into a fresh target.
pub fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
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

/// Panics unless the canvas on screen equals a cold evaluation of the artwork shown.
pub fn assert_cold(core: &Core, runtime: &Runtime, when: &str) {
    let shown = runtime.read_canvas().expect("readback").expect("canvas");
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(
        shown.content_hash(),
        cold(core, artwork).content_hash(),
        "{when}: the canvas differs from a cold evaluation"
    );
}
