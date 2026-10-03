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
    InputBatch, Intent, SelectionIntent, StrokeIntent, Touch, TouchAction, TransformFrame,
    TransformIntent, ViewIntent, ViewMapping, Viewport,
};
use openpaint_core::CanvasSize;

use crate::components::{CanvasArea, ScreenArea};
use crate::selection_bar::CLOSE_POINTS;
use crate::transform_bar::Handles;
use crate::transform_drag::{TransformDrag, gesture_placement};

/// What routing one frame's contacts needs to know.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CanvasFrame<'a> {
    pub(crate) area: CanvasArea,
    /// Controls over the canvas: a contact touching down on one is never the canvas's.
    pub(crate) covers: &'a [ScreenArea],
    /// The open artwork's size and how it is viewed.
    pub(crate) canvas: CanvasSize,
    pub(crate) view: CanvasView,
    pub(crate) pixels_per_point: f32,
    pub(crate) now_ms: i64,
    /// The next contact picks a colour.
    pub(crate) picking: bool,
    /// The Selection tool is on: the pen draws selections, from anywhere open (the desk too).
    pub(crate) selecting: bool,
    /// The Transform tool's frame: the pen and the fingers move what it holds, never draw.
    pub(crate) transform: Option<TransformFrame>,
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
        self.area.screen.contains(at) && !self.covers.iter().any(|cover| cover.contains(at))
    }
}

#[derive(Debug, Default)]
pub(crate) struct CanvasInput {
    owned: Vec<i32>,
    gestures: Gestures,
    /// When the view last moved, by the input clock.
    navigated_ms: Option<i64>,
    /// Fingers on the glass now.
    fingers: Vec<i32>,
    /// Where the pen hovers over open canvas (screen pixels), while no finger is down.
    hover: Option<[f32; 2]>,
    /// Where the pen drawing on the canvas is, while it is over open canvas and no finger is
    /// down (screen pixels).
    pressing: Option<[f32; 2]>,
    /// Where the contact picking a colour is (screen pixels).
    picking_at: Option<[f32; 2]>,
    /// Contacts on the Transform tool's frame.
    drag: TransformDrag,
}

impl CanvasInput {
    /// The intents of `batches`: stroke contacts first, in order, then what the fingers did.
    /// While picking, a contact over a control previews nothing: the last preview stays.
    pub(crate) fn route(&mut self, frame: &CanvasFrame<'_>, batches: &[InputBatch]) -> Vec<Intent> {
        let mapping = frame.mapping();
        let mut intents = Vec::new();
        let mut touches = Vec::new();
        let mut pen = false;
        if !frame.picking {
            self.picking_at = None;
        }
        for contact in batches.iter().flat_map(|batch| &batch.contacts) {
            self.follow(contact, frame);
            let first = contact.samples.first().map(|sample| sample.position);
            let open =
                contact.phase == ContactPhase::Down && first.is_some_and(|at| frame.open(at));
            if let Some(mut taken) = self.transform_contact(contact, frame, &mapping, open) {
                intents.append(&mut taken);
                continue;
            }
            if contact.device == Device::Finger && !frame.picking {
                touches.extend(touch(contact, open));
                continue;
            }
            pen |= matches!(contact.device, Device::Pen | Device::Eraser);
            let anywhere = frame.picking || frame.selecting;
            let on_canvas = first.is_some_and(|at| anywhere || inside(&mapping, frame.canvas, at));
            if open && on_canvas && !self.owned.contains(&contact.id) {
                self.owned.push(contact.id);
                // The view status gives way to the pen at once.
                self.navigated_ms = None;
            }
            if !self.owned.contains(&contact.id) {
                continue;
            }
            let ends = matches!(contact.phase, ContactPhase::Up | ContactPhase::Cancel);
            if ends {
                self.owned.retain(|id| *id != contact.id);
            }
            let at = contact.samples.last().map(|sample| sample.position);
            self.track(at.filter(|_| !ends), frame);
            let covered = !at.is_some_and(|at| frame.open(at));
            if frame.picking && contact.phase == ContactPhase::Move && covered {
                continue;
            }
            if matches!(contact.phase, ContactPhase::Hover | ContactPhase::Leave) {
                continue;
            }
            intents.push(owned_intent(mapped(contact, &mapping), frame, &mapping));
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

    /// While the Transform tool is on: the intents of a contact on its frame, or none for the
    /// pen elsewhere (it never draws then); `None` for a finger left to the gesture trackers,
    /// which notes whether the fingers' gesture began inside the frame.
    fn transform_contact(
        &mut self,
        contact: &Contact,
        frame: &CanvasFrame<'_>,
        mapping: &ViewMapping,
        open: bool,
    ) -> Option<Vec<Intent>> {
        let transform = frame.transform.as_ref()?;
        let held = (frame.pixels_per_point, open);
        if let Some(taken) = self.drag.route(contact, transform, mapping, held) {
            return Some(taken);
        }
        if contact.device != Device::Finger {
            return Some(Vec::new());
        }
        if contact.phase == ContactPhase::Down && self.fingers.len() == 1 {
            let handles = Handles::of(transform, mapping, frame.pixels_per_point);
            let first = contact.samples.first().map(|sample| sample.position);
            self.drag.fingers = open && first.is_some_and(|at| handles.inside(at));
        }
        None
    }

    fn apply(&mut self, gestured: Gestured, frame: &CanvasFrame<'_>, intents: &mut Vec<Intent>) {
        let on_frame = frame.transform.filter(|_| self.drag.fingers);
        match gestured.action {
            Some(TouchAction::Undo) => intents.push(Intent::History(HistoryIntent::Undo)),
            Some(TouchAction::Redo) => intents.push(Intent::History(HistoryIntent::Redo)),
            Some(TouchAction::Fit) if on_frame.is_some() => {}
            Some(TouchAction::Fit) => {
                self.navigated_ms = Some(frame.now_ms);
                intents.push(Intent::View(ViewIntent::Fit));
            }
            None => {}
        }
        if let (Some(gesture), Some(transform)) = (gestured.view, on_frame) {
            let placed = gesture_placement(transform.placement, &gesture, &frame.mapping());
            if let Some(placement) = placed {
                intents.push(Intent::Transform(TransformIntent::Place(placement)));
            }
        } else if let Some(gesture) = gestured.view {
            self.navigated_ms = Some(frame.now_ms);
            let viewport = frame.viewport();
            intents.push(Intent::View(ViewIntent::Gesture { gesture, viewport }));
        }
        if self.drag.fingers && self.fingers.is_empty() {
            // The fingers that moved the frame's content lifted: one step.
            self.drag.fingers = false;
            intents.push(Intent::Transform(TransformIntent::Settle));
        }
    }

    /// Where an owned contact is now (`None` once it ended): the picking contact's place, or the
    /// drawing pen's while over open canvas with no finger down.
    fn track(&mut self, at: Option<[f32; 2]>, frame: &CanvasFrame<'_>) {
        if frame.picking {
            self.picking_at = at;
            self.pressing = None;
        } else {
            let open = |at: &[f32; 2]| frame.open(*at) && self.fingers.is_empty();
            self.pressing = at.filter(open);
        }
    }

    /// Keeps the fingers down and where the pen hovers: over open canvas, with no finger down
    /// and the eyedropper off; any other contact of the pen hides it.
    fn follow(&mut self, contact: &Contact, frame: &CanvasFrame<'_>) {
        if contact.device == Device::Finger {
            match contact.phase {
                ContactPhase::Down if !self.fingers.contains(&contact.id) => {
                    self.fingers.push(contact.id);
                }
                ContactPhase::Up | ContactPhase::Cancel => {
                    self.fingers.retain(|id| *id != contact.id);
                }
                _ => {}
            }
            if !self.fingers.is_empty() {
                self.hover = None;
                self.pressing = None;
            }
            return;
        }
        let at = contact.samples.last().map(|sample| sample.position);
        self.hover = at.filter(|at| {
            contact.phase == ContactPhase::Hover
                && self.fingers.is_empty()
                && !frame.picking
                && frame.open(*at)
        });
    }

    /// Where the pen hovers over open canvas, in screen pixels.
    pub(crate) const fn hover(&self) -> Option<[f32; 2]> {
        self.hover
    }

    /// Where the pen drawing on open canvas is, with no finger down, in screen pixels.
    pub(crate) const fn pressing(&self) -> Option<[f32; 2]> {
        self.pressing
    }

    /// Where the contact picking from the canvas is, in screen pixels.
    pub(crate) const fn picking_at(&self) -> Option<[f32; 2]> {
        self.picking_at
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

/// What an owned contact in canvas pixels asks for: drawing a selection while the Selection
/// tool is on (closing within `CLOSE_POINTS` of the path's start at this view), else a stroke
/// (or, while the eyedropper is on, a pick).
fn owned_intent(contact: Contact, frame: &CanvasFrame<'_>, mapping: &ViewMapping) -> Intent {
    if frame.selecting && !frame.picking {
        let close = CLOSE_POINTS * frame.pixels_per_point / mapping.scale();
        Intent::Selection(SelectionIntent::Contact { contact, close })
    } else {
        Intent::Stroke(StrokeIntent::Contact(contact))
    }
}

/// True when a contact of `batches` touches down on the canvas `area` outside every one of
/// `covers`.
pub(crate) fn touched_outside(
    area: &ScreenArea,
    covers: &[ScreenArea],
    batches: &[InputBatch],
) -> bool {
    batches
        .iter()
        .flat_map(|batch| &batch.contacts)
        .filter(|contact| contact.phase == ContactPhase::Down)
        .filter_map(|contact| contact.samples.first())
        .any(|first| {
            area.contains(first.position)
                && !covers.iter().any(|cover| cover.contains(first.position))
        })
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
