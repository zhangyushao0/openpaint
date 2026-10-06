//! ColorDrop (Procreate's fill), as draweave tuned it: a contact (pen or finger) that presses
//! the colour button and moves more than 8 points carries a drop of the colour; lifting it over
//! the canvas fills the region there. Held still (under 3 points of jitter) for 0.4 s over the
//! canvas, the drop anchors: the fill shows as a preview and sliding sideways changes the
//! threshold, `sign * (|dx| - 4) / 600` per point, in 0.5 % steps, from the threshold it had;
//! lifting keeps the newest. Lifting over a control or off the canvas cancels. After a drop,
//! "Continue filling" lets each pen tap on the canvas fill there, until Done.
//!
//! Routed from the input batches, before the canvas sees them: the contacts a drop owns never
//! draw. The drop's notices and bar are drawn in the next pass from this state.

use openpaint_app::{ContactPhase, Device, FillIntent, InputBatch, Intent};
use openpaint_core::Color;

use crate::canvas_input::CanvasFrame;
use crate::components::ScreenArea;

/// Movement that turns a press on the colour button into a drop, in points.
const ACTIVATE: f32 = 8.0;
/// Jitter a held drop may have and still count as still, in points.
const STILL: f32 = 3.0;
/// How long a still drop waits before it anchors and the threshold can be adjusted.
const HOLD_MS: i64 = 400;
/// Sideways movement around the anchor that changes nothing, in points.
const DEAD_ZONE: f32 = 4.0;
/// Points of sideways movement per whole threshold range.
const RANGE: f32 = 600.0;

/// What the drop needs besides the canvas frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DropFrame {
    /// Where the colour button is, from the last pass.
    pub(crate) swatch: Option<ScreenArea>,
    pub(crate) color: Option<Color>,
    /// The threshold a new drop starts with.
    pub(crate) threshold: f32,
    /// The fill bar in the top band, which the drop's contacts never start on.
    pub(crate) bar: Option<ScreenArea>,
}

/// Where a held drop anchored: the screen point, the canvas point it fills from and the
/// threshold it started with.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    at: [f32; 2],
    point: [f32; 2],
    base: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Drag {
    id: i32,
    start: [f32; 2],
    at: [f32; 2],
    /// Where and since when the contact has been still.
    still: ([f32; 2], i64),
    active: bool,
    anchor: Option<Anchor>,
    /// The last request sent.
    sent: Option<([f32; 2], f32)>,
}

#[derive(Debug, Default)]
pub(crate) struct ColorDrop {
    drag: Option<Drag>,
    /// Pen taps on the canvas fill.
    continuing: bool,
    /// "Continue filling" is offered after a drop.
    offered: bool,
}

/// What the fill bar shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DropBar {
    /// A drop is carried; with the threshold once it is anchored.
    Dropping(Option<f32>),
    /// "Continue filling" is offered.
    Offered,
    /// Pen taps fill, until Done.
    Continuing,
}

impl ColorDrop {
    /// Takes the contacts a drop owns out of `batches` and returns the fill intents and the
    /// batches left for the canvas.
    pub(crate) fn route(
        &mut self,
        frame: &CanvasFrame<'_>,
        drop: &DropFrame,
        batches: &[InputBatch],
    ) -> (Vec<Intent>, Vec<InputBatch>) {
        let mut intents = Vec::new();
        let mut left = Vec::with_capacity(batches.len());
        for batch in batches {
            let mut rest = InputBatch {
                contacts: Vec::new(),
                pen: batch.pen,
            };
            for contact in &batch.contacts {
                let (Some(first), Some(last)) = (contact.samples.first(), contact.samples.last())
                else {
                    continue;
                };
                if contact.phase == ContactPhase::Down && self.drag.is_none() {
                    let start = (first.position, first.time_ms);
                    self.begin(frame, drop, contact.id, contact.device, start);
                }
                let owned = self.drag.as_ref().is_some_and(|drag| drag.id == contact.id);
                if !owned {
                    rest.contacts.push(contact.clone());
                    continue;
                }
                self.moved(frame, last.position, last.time_ms);
                match contact.phase {
                    ContactPhase::Up => self.lift(frame, drop, &mut intents),
                    ContactPhase::Cancel => {
                        let active = self.drag.take().is_some_and(|drag| drag.active);
                        intents.extend(active.then_some(Intent::Fill(FillIntent::Cancel)));
                    }
                    _ => {}
                }
            }
            left.push(rest);
        }
        self.hold(frame, drop, &mut intents);
        (intents, left)
    }

    /// A contact touching down at `at`: on the colour button it may become a drop; while
    /// continuing, a pen on the canvas is one at once (a fill tap). Any other contact lets the
    /// offer of continuing go.
    fn begin(
        &mut self,
        frame: &CanvasFrame<'_>,
        drop: &DropFrame,
        id: i32,
        device: Device,
        (at, time_ms): ([f32; 2], i64),
    ) {
        let on_bar = drop.bar.is_some_and(|bar| bar.contains(at));
        let on_swatch = drop.swatch.is_some_and(|swatch| swatch.contains(at));
        let tap = self.continuing && device == Device::Pen && canvas_point(frame, at).is_some();
        if !on_bar && !tap {
            self.offered = false;
        }
        if (on_swatch || tap) && drop.color.is_some() && !frame.drawing {
            self.drag = Some(Drag {
                id,
                start: at,
                at,
                still: (at, time_ms),
                active: tap,
                anchor: None,
                sent: None,
            });
        }
    }

    fn moved(&mut self, frame: &CanvasFrame<'_>, at: [f32; 2], time_ms: i64) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let points =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) / frame.pixels_per_point;
        if drag.anchor.is_none() && points(drag.still.0, at) > STILL {
            drag.still = (at, time_ms);
        }
        drag.at = at;
        drag.active |= points(drag.start, at) > ACTIVATE;
    }

    /// Lifting a drop: over the canvas it fills (the anchored point and threshold, or where it
    /// lifted), anywhere else it cancels.
    fn lift(&mut self, frame: &CanvasFrame<'_>, drop: &DropFrame, intents: &mut Vec<Intent>) {
        let Some(drag) = self.drag.take().filter(|drag| drag.active) else {
            return;
        };
        match request(frame, drop, &drag) {
            Some((point, threshold)) => {
                if drag.sent != Some((point, threshold)) {
                    push_request(intents, drop, point, threshold);
                }
                intents.push(Intent::Fill(FillIntent::Commit));
                self.offered = !self.continuing;
            }
            None => intents.push(Intent::Fill(FillIntent::Cancel)),
        }
    }

    /// A drop held still long enough anchors; an anchored one asks for the threshold the
    /// hand's sideways movement sets, once per change.
    fn hold(&mut self, frame: &CanvasFrame<'_>, drop: &DropFrame, intents: &mut Vec<Intent>) {
        let Some(drag) = self.drag.as_mut().filter(|drag| drag.active) else {
            return;
        };
        if drag.anchor.is_none()
            && frame.now_ms - drag.still.1 >= HOLD_MS
            && let Some(point) = canvas_point(frame, drag.at)
        {
            drag.anchor = Some(Anchor {
                at: drag.at,
                point,
                base: drop.threshold,
            });
        }
        if drag.anchor.is_some()
            && let Some(wanted) = request(frame, drop, drag)
            && drag.sent != Some(wanted)
        {
            push_request(intents, drop, wanted.0, wanted.1);
            drag.sent = Some(wanted);
        }
    }

    /// What the fill bar shows, if anything.
    pub(crate) fn bar(&self) -> Option<DropBar> {
        if let Some(drag) = self.drag.as_ref().filter(|drag| drag.active) {
            return Some(DropBar::Dropping(drag.sent.map(|(_, threshold)| threshold)));
        }
        if self.continuing {
            Some(DropBar::Continuing)
        } else if self.offered {
            Some(DropBar::Offered)
        } else {
            None
        }
    }

    /// Where a carried drop is on screen, in pixels.
    pub(crate) fn carried(&self) -> Option<[f32; 2]> {
        self.drag
            .as_ref()
            .filter(|drag| drag.active)
            .map(|drag| drag.at)
    }

    /// True while a drop is held: frames must keep coming for the hold to anchor.
    pub(crate) fn holding(&self) -> bool {
        self.drag.is_some()
    }

    /// "Continue filling" was tapped (`true`), or Done (`false`).
    pub(crate) fn set_continuing(&mut self, continuing: bool) {
        self.continuing = continuing;
        self.offered = false;
    }

    /// The screen changed or another tool was chosen: no drop, no continuing.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The canvas pixel under the screen point `at`, when it lies on the canvas itself, in the
/// canvas area and outside every control.
fn canvas_point(frame: &CanvasFrame<'_>, at: [f32; 2]) -> Option<[f32; 2]> {
    if !frame.open(at) {
        return None;
    }
    let [x, y] = frame.mapping().to_canvas(at);
    let size = [frame.canvas.width() as f32, frame.canvas.height() as f32];
    ((0.0..size[0]).contains(&x) && (0.0..size[1]).contains(&y)).then_some([x, y])
}

/// The point and threshold `drag` asks for now; `None` when it is over a control or off the
/// canvas, which cancels on lifting even after it anchored.
fn request(frame: &CanvasFrame<'_>, drop: &DropFrame, drag: &Drag) -> Option<([f32; 2], f32)> {
    let here = canvas_point(frame, drag.at)?;
    Some(match drag.anchor {
        Some(anchor) => {
            let dx = (drag.at[0] - anchor.at[0]) / frame.pixels_per_point;
            (anchor.point, adjusted(anchor.base, dx))
        }
        None => (here, drop.threshold),
    })
}

/// `base` moved by `dx` points of sideways movement: nothing within the dead zone, then the
/// whole range per `RANGE` points, in steps of 0.5 %.
pub(crate) fn adjusted(base: f32, dx: f32) -> f32 {
    let delta = dx.signum() * (dx.abs() - DEAD_ZONE).max(0.0) / RANGE;
    ((base + delta).clamp(0.0, 1.0) * 200.0).round() / 200.0
}

fn push_request(intents: &mut Vec<Intent>, drop: &DropFrame, point: [f32; 2], threshold: f32) {
    if let Some(color) = drop.color {
        intents.push(Intent::Fill(FillIntent::Request {
            point,
            threshold,
            color,
        }));
    }
}

#[cfg(test)]
#[path = "color_drop_tests.rs"]
mod tests;
