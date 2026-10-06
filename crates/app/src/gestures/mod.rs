//! Finger gestures on the canvas (Procreate's defaults as draweave tuned them, D34): two fingers
//! pan, pinch-zoom and rotate, each past its own dead zone; one finger pans; a quick pinch fits
//! the canvas; a two-finger tap undoes and a three-finger tap redoes, held they repeat; three
//! fingers swiped down open the copy menu (`swipe.rs`). The pen
//! never navigates and touch never paints (D18).
//!
//! The UI's canvas input feeds every finger contact here with whether it touched down on open
//! canvas (inside the canvas area, outside every control over it); this module decides what
//! the fingers mean. Positions are screen pixels; thresholds are points.

mod filter;
mod pair;
mod pan;
mod picture;
mod swipe;
mod taps;

pub use picture::{PictureGestured, PictureGestures};

use crate::input::ContactPhase;
use crate::view::ViewGesture;

use filter::NavigationFilter;
use pair::PairTracker;
use pan::OneFingerPan;
use swipe::Swipe;
use taps::Taps;

/// One finger contact's news in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Touch {
    pub id: i32,
    pub phase: ContactPhase,
    /// The contact's current position (its last sample), in screen pixels.
    pub position: [f32; 2],
    /// Platform time of that sample.
    pub time_ms: i64,
    /// True when the contact touched down on open canvas; only read for `Down`.
    pub open: bool,
}

/// What else the frame holds that gestures must respect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureFrame {
    pub pixels_per_point: f32,
    /// The input clock now, for holds that repeat without new input.
    pub now_ms: i64,
    /// A stroke is being drawn.
    pub drawing: bool,
    /// A pen contact is in this frame's input.
    pub pen: bool,
}

/// A finger shortcut. Each duplicates a visible control (D32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchAction {
    Undo,
    Redo,
    /// The copy menu (S40): three fingers swiped down.
    CopyPaste,
    /// The whole canvas in view.
    Fit,
}

/// What the fingers asked for this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Gestured {
    pub action: Option<TouchAction>,
    pub view: Option<ViewGesture>,
}

/// The gesture trackers of the canvas.
#[derive(Debug, Default)]
pub struct Gestures {
    pair: PairTracker,
    filter: NavigationFilter,
    pan: OneFingerPan,
    taps: Taps,
    swipe: Swipe,
    /// The fingers on the glass moved the view.
    moved: bool,
}

impl Gestures {
    /// The meaning of this frame's finger contacts, in order.
    pub fn update(&mut self, touches: &[Touch], frame: GestureFrame) -> Gestured {
        let ppp = frame.pixels_per_point;
        let swiped = self.swipe.update(touches, ppp);
        let action = self.taps.update(touches, ppp, frame.now_ms).or(swiped);
        let one = if frame.drawing || frame.pen {
            self.pan.reset();
            None
        } else {
            self.pan.update(touches, ppp)
        };
        let delta = self.pair.update(touches);
        let two = self.filter.update(delta, ppp);
        if !self.pair.active() {
            self.filter.update(None, ppp);
        }
        let mut out = Gestured::default();
        if let Some(action) = action {
            out.action = Some(action);
        } else {
            out.view = two.or(one);
        }
        // Navigating while fingers that moved the view are still down.
        let down = self.filter.active() || self.pan.active();
        self.moved = down && out.action.is_none() && (self.moved || out.view.is_some());
        out
    }

    /// A control took a contact, the eyedropper took the canvas or the screen changed: every
    /// finger down so far is forgotten, so nothing lingers as a canvas gesture.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// True while the fingers move the view.
    #[must_use]
    pub fn navigating(&self) -> bool {
        self.moved
    }

    /// True while fingers rest still for a history shortcut that repeats: frames must keep
    /// coming without input.
    #[must_use]
    pub fn holding(&self) -> bool {
        self.taps.holding()
    }
}

/// `value` shortened by `threshold`, or zero inside it.
fn vector_after_dead_zone(value: [f32; 2], threshold: f32) -> [f32; 2] {
    let length = value[0].hypot(value[1]);
    if length <= threshold || length <= f32::EPSILON {
        [0.0; 2]
    } else {
        value.map(|v| v * ((length - threshold) / length))
    }
}

fn scalar_after_dead_zone(value: f32, threshold: f32) -> f32 {
    value.signum() * (value.abs() - threshold).max(0.0)
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
