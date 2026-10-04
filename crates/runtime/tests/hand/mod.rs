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

    /// A finger tap on `spot`: a control's name, or window pixels off every control.
    pub fn tap(&mut self, runtime: &mut Runtime, spot: impl Spot) {
        let at = spot.point(runtime);
        self.gesture(runtime, ToolKind::Finger, &[at, at]);
    }
}

/// Where a tap lands, as the scenario runner reads its spots (D48): a control by its name, so a
/// layout change never moves what a test taps, or window pixels, which must lie off every
/// control of the last frame.
pub trait Spot {
    fn point(&self, runtime: &Runtime) -> [f32; 2];
}

/// The middle of the control called this.
impl Spot for &str {
    fn point(&self, runtime: &Runtime) -> [f32; 2] {
        on(runtime, self, [0.5, 0.5])
    }
}

/// Window pixels on the canvas or the desk; on a control they fail with its name to write.
impl Spot for [f32; 2] {
    fn point(&self, runtime: &Runtime) -> [f32; 2] {
        if let Some((name, control)) = runtime.controls().under(*self) {
            let [left, top, right, bottom] = control.rect;
            let fraction = [
                (self[0] - left) / (right - left),
                (self[1] - top) / (bottom - top),
            ];
            panic!("the tap at {self:?} lands on (\"{name}\", {fraction:?}): name it");
        }
        *self
    }
}

/// The point at a fraction of the named control's width and height from its top-left corner.
impl Spot for (&str, [f32; 2]) {
    fn point(&self, runtime: &Runtime) -> [f32; 2] {
        on(runtime, self.0, self.1)
    }
}

/// The point `fraction` across the control `name` of the last frame, in pixels; panics with
/// every name on screen when no single control has that name.
pub fn on(runtime: &Runtime, name: &str, fraction: [f32; 2]) -> [f32; 2] {
    match runtime.controls().find(name) {
        Ok(control) => control.at(fraction),
        Err(why) => panic!(
            "{name}: {why:?}; on screen: {:?}",
            runtime.controls().names()
        ),
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
