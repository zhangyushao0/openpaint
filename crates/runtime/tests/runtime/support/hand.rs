//! A hand on the runtime: gestures through the frame loop as a hand makes them, on a clock the
//! hand holds (`Runtime::fix_clock`), and the cold draw the canvas must equal. An input comes at
//! its samples' time and the frames it asks for come at that time; a frame without input comes
//! one 60 Hz frame later (`frames.rs`); while a worker is busy (a store, a fill, a comparison)
//! frames come at the same time. So neither a slow frame nor a slow worker changes what a test
//! draws or sees (a hold, a drop, a tap's length, motion, what is taken ahead while idle): the
//! only wall-clock limit is `HANG`, a guard against a hang, never a measure.

use std::time::{Duration, Instant};

use openpaint_core::{Artwork, Color};
use openpaint_render::{CanvasTarget, Core, FrameRequest, RgbaImage};
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};

/// The longest a test waits for anything (a worker, the canvas filling in, motion settling): two
/// minutes of wall time, though the gate runs a dozen test binaries at once.
pub const HANG: Duration = Duration::from_secs(120);
/// Milliseconds between a gesture's samples.
const SAMPLE_MS: i64 = 8;
/// Frames an input is followed by at most, as the host paces them.
const FOLLOW_UPS: usize = 4;

/// A screen of `size` pixels and the clock the hand holds the runtime at, in milliseconds.
pub struct Hand {
    pub size: [u32; 2],
    pub clock: i64,
}

impl Hand {
    /// A hand whose clock starts at 0: make it before the runtime's first frame, and run every
    /// frame through it, so the runtime's clock never goes back.
    pub fn new(size: [u32; 2]) -> Self {
        Self { size, clock: 0 }
    }

    /// The clock `ms` on, the runtime held there; the time it reads, for the next input's
    /// samples.
    pub fn tick(&mut self, runtime: &mut Runtime, ms: i64) -> i64 {
        self.clock += ms;
        runtime.fix_clock(self.clock);
        self.clock
    }

    /// One frame at the time held, then more at that time while a worker is busy; whether the
    /// runtime wants another frame.
    pub fn frame_now(&mut self, runtime: &mut Runtime) -> bool {
        let start = Instant::now();
        loop {
            let again = runtime.frame_offscreen(self.size).expect("frame");
            if !runtime.storing() {
                return again;
            }
            assert!(start.elapsed() < HANG, "a worker still busy after {HANG:?}");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// The frames the runtime asks for after an input, at its time (`frame_now`), at most
    /// `FOLLOW_UPS`.
    pub fn follow(&mut self, runtime: &mut Runtime) {
        for _ in 0..FOLLOW_UPS {
            if !self.frame_now(runtime) {
                break;
            }
        }
    }

    /// One input of `phase`: a sample of each pointer (id, tool, where) at the next sample time,
    /// then the frames it asks for.
    pub fn send(
        &mut self,
        runtime: &mut Runtime,
        phase: MotionPhase,
        pointers: &[(i32, ToolKind, [f32; 2])],
    ) {
        let time_ms = self.tick(runtime, SAMPLE_MS);
        let pointers = pointers
            .iter()
            .map(|&(id, tool, position)| Pointer {
                id,
                tool,
                samples: vec![PointerSample {
                    position,
                    pressure: 0.8,
                    tilt: 0.0,
                    orientation: 0.0,
                    time_ms,
                }],
            })
            .collect();
        runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers,
        });
        self.follow(runtime);
    }

    /// One pointer moving through `points`: down at the first, a move per point, up at the
    /// last, a sample each `SAMPLE_MS`.
    pub fn gesture(&mut self, runtime: &mut Runtime, tool: ToolKind, points: &[[f32; 2]]) {
        let last = points.len() - 1;
        for (index, position) in points.iter().enumerate() {
            let phase = match index {
                0 => MotionPhase::Down,
                i if i == last => MotionPhase::Up,
                _ => MotionPhase::Move,
            };
            self.send(runtime, phase, &[(0, tool, *position)]);
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

/// `artwork` drawn whole on white into a fresh target and read back.
pub fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    let request = FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    };
    CanvasTarget::whole(core, &request, false).expect("drawn whole")
}

/// Panics unless the canvas on screen (every waiting tile evaluated) equals a cold evaluation of
/// the artwork shown.
pub fn assert_cold(core: &Core, runtime: &mut Runtime, when: &str) {
    let shown = runtime.read_canvas().expect("readback").expect("canvas");
    let artwork = runtime.artwork().expect("artwork");
    assert_eq!(
        shown.content_hash(),
        cold(core, artwork).content_hash(),
        "{when}: the canvas differs from a cold evaluation"
    );
}
