//! Who owns the canvas's contacts, once the controls over it had their turn. The pen (and,
//! while the eyedropper is on, any contact) that touches down on the canvas inside the canvas
//! area and outside every control over it is owned until it lifts, wherever it moves, and goes
//! to the session as stroke contacts in canvas pixels, mapped through the view. Fingers go to
//! the gesture trackers (`app::Gestures`): navigation, quick pinch to fit, two- and three-finger
//! taps for undo and redo. A control that takes a contact, or the eyedropper taking the canvas,
//! resets the trackers, so nothing lingers as a canvas gesture.
//!
//! Contacts are routed outside the egui pass, from the areas the last pass laid out: egui may
//! run a pass twice, and an intent must never be emitted twice.

use openpaint_app::{
    CanvasView, Contact, ContactPhase, Device, GestureFrame, Gestured, Gestures, HistoryIntent,
    InputBatch, Intent, StrokeIntent, Touch, TouchAction, ViewIntent, ViewMapping, Viewport,
};
use openpaint_core::CanvasSize;

use crate::components::ScreenArea;

/// What routing one frame's contacts needs to know.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CanvasFrame<'a> {
    pub(crate) area: ScreenArea,
    /// Controls over the canvas: a contact touching down on one is never the canvas's.
    pub(crate) covers: &'a [ScreenArea],
    /// The open artwork's size and how it is viewed.
    pub(crate) canvas: CanvasSize,
    pub(crate) view: CanvasView,
    pub(crate) pixels_per_point: f32,
    pub(crate) now_ms: i64,
    /// The next contact picks a colour.
    pub(crate) picking: bool,
    pub(crate) drawing: bool,
    /// A control took a contact this frame.
    pub(crate) control: bool,
}

impl CanvasFrame<'_> {
    fn viewport(&self) -> Viewport {
        self.area.viewport(self.pixels_per_point)
    }

    pub(crate) fn mapping(&self) -> ViewMapping {
        self.view.mapping(self.canvas, &self.viewport())
    }

    /// True when the screen point `at` lies in the canvas area outside every control.
    pub(crate) fn open(&self, at: [f32; 2]) -> bool {
        self.area.contains(at) && !self.covers.iter().any(|cover| cover.contains(at))
    }
}

#[derive(Debug, Default)]
pub(crate) struct CanvasInput {
    owned: Vec<i32>,
    gestures: Gestures,
    /// When the view last moved, by the input clock.
    navigated_ms: Option<i64>,
}

impl CanvasInput {
    /// The intents of `batches`: stroke contacts first, in order, then what the fingers did.
    pub(crate) fn route(&mut self, frame: &CanvasFrame<'_>, batches: &[InputBatch]) -> Vec<Intent> {
        let mapping = frame.mapping();
        let mut intents = Vec::new();
        let mut touches = Vec::new();
        let mut pen = false;
        for contact in batches.iter().flat_map(|batch| &batch.contacts) {
            let first = contact.samples.first().map(|sample| sample.position);
            let open =
                contact.phase == ContactPhase::Down && first.is_some_and(|at| frame.open(at));
            if contact.device == Device::Finger && !frame.picking {
                touches.extend(touch(contact, open));
                continue;
            }
            pen |= matches!(contact.device, Device::Pen | Device::Eraser);
            let on_canvas =
                first.is_some_and(|at| frame.picking || inside(&mapping, frame.canvas, at));
            if open && on_canvas && !self.owned.contains(&contact.id) {
                self.owned.push(contact.id);
                // The view status gives way to the pen at once.
                self.navigated_ms = None;
            }
            if !self.owned.contains(&contact.id) {
                continue;
            }
            if matches!(contact.phase, ContactPhase::Up | ContactPhase::Cancel) {
                self.owned.retain(|id| *id != contact.id);
            }
            if !matches!(contact.phase, ContactPhase::Hover | ContactPhase::Leave) {
                intents.push(Intent::Stroke(StrokeIntent::Contact(mapped(
                    contact, &mapping,
                ))));
            }
        }
        let gestured = if frame.picking || frame.control {
            self.gestures.reset();
            Gestured::default()
        } else {
            let gesture_frame = GestureFrame {
                pixels_per_point: frame.pixels_per_point,
                now_ms: frame.now_ms,
                drawing: frame.drawing,
                pen,
            };
            self.gestures.update(&touches, gesture_frame)
        };
        self.apply(gestured, frame, &mut intents);
        intents
    }

    fn apply(&mut self, gestured: Gestured, frame: &CanvasFrame<'_>, intents: &mut Vec<Intent>) {
        match gestured.action {
            Some(TouchAction::Undo) => intents.push(Intent::History(HistoryIntent::Undo)),
            Some(TouchAction::Redo) => intents.push(Intent::History(HistoryIntent::Redo)),
            Some(TouchAction::Fit) => {
                self.navigated_ms = Some(frame.now_ms);
                intents.push(Intent::View(ViewIntent::Fit));
            }
            None => {}
        }
        if let Some(gesture) = gestured.view {
            self.navigated_ms = Some(frame.now_ms);
            let viewport = frame.viewport();
            intents.push(Intent::View(ViewIntent::Gesture { gesture, viewport }));
        }
    }

    /// The input clock when the view last moved, while it may still be on the move.
    pub(crate) fn navigated(&self) -> Option<i64> {
        if self.gestures.navigating() {
            Some(i64::MAX)
        } else {
            self.navigated_ms
        }
    }

    /// True while fingers rest for a shortcut that repeats: frames must keep coming.
    pub(crate) fn holding(&self) -> bool {
        self.gestures.holding()
    }
}

/// `contact` as the gesture trackers see it: its current position.
fn touch(contact: &Contact, open: bool) -> Option<Touch> {
    let last = contact.samples.last()?;
    Some(Touch {
        id: contact.id,
        phase: contact.phase,
        position: last.position,
        time_ms: last.time_ms,
        open,
    })
}

/// True when the screen point `at` shows the canvas itself, not the surround.
fn inside(mapping: &ViewMapping, canvas: CanvasSize, at: [f32; 2]) -> bool {
    let [x, y] = mapping.to_canvas(at);
    (0.0..canvas.width() as f32).contains(&x) && (0.0..canvas.height() as f32).contains(&y)
}

/// `contact` with its positions in canvas pixels.
fn mapped(contact: &Contact, mapping: &ViewMapping) -> Contact {
    let mut moved = contact.clone();
    for sample in &mut moved.samples {
        sample.position = mapping.to_canvas(sample.position);
    }
    moved
}

#[cfg(test)]
#[path = "canvas_input_tests.rs"]
mod tests;
