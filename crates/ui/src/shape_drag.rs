//! Contacts on QuickShape's handles and on the Drawing Guide's centre (S42). While a snapped
//! shape is edited, a pen or finger touching down within reach of one of its handles takes the
//! nearest and drags it until it lifts (each move one `MoveHandle`). While the guide is on, a
//! finger touching down on its centre moves it; the pen never does, so a stroke begun at the
//! centre (a radial pattern's middle) draws.

use openpaint_app::{
    Contact, ContactPhase, Device, GuideIntent, Intent, StrokeIntent, ViewMapping,
};

use crate::canvas_input::CanvasFrame;
use crate::transform_bar::HANDLE_REACH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grip {
    Handle(usize),
    Centre,
}

/// The contact holding a grip, until it lifts.
#[derive(Debug, Default)]
pub(crate) struct Grips {
    held: Option<(i32, Grip)>,
}

impl Grips {
    /// The intents of `contact` when it holds a grip, or touches down `open` (outside every
    /// control) on one; `None` leaves it to the canvas.
    pub(crate) fn route(
        &mut self,
        contact: &Contact,
        frame: &CanvasFrame<'_>,
        mapping: &ViewMapping,
        open: bool,
    ) -> Option<Vec<Intent>> {
        if let Some((id, grip)) = self.held {
            if contact.id != id {
                return None;
            }
            if matches!(contact.phase, ContactPhase::Up | ContactPhase::Cancel) {
                self.held = None;
            }
            if contact.phase == ContactPhase::Cancel {
                return Some(Vec::new());
            }
            let at = mapping.to_canvas(contact.samples.last()?.position);
            let intent = match grip {
                Grip::Handle(handle) => StrokeIntent::MoveHandle { handle, to: at },
                Grip::Centre => StrokeIntent::Guide(GuideIntent::Centre(at)),
            };
            return Some(vec![Intent::Stroke(intent)]);
        }
        if contact.phase != ContactPhase::Down || !open {
            return None;
        }
        let at = contact.samples.first()?.position;
        let reach = HANDLE_REACH * frame.pixels_per_point;
        let distance = |point: [f32; 2]| {
            let [x, y] = mapping.to_screen(point);
            (x - at[0]).hypot(y - at[1])
        };
        let handle = (0..frame.handles.len())
            .map(|index| (index, distance(frame.handles[index])))
            .filter(|(_, distance)| *distance <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let centre = frame
            .guide_centre
            .filter(|centre| contact.device == Device::Finger && distance(*centre) <= reach);
        let grip = match (handle, centre) {
            (Some((index, _)), _) => Grip::Handle(index),
            (None, Some(_)) => Grip::Centre,
            (None, None) => return None,
        };
        self.held = Some((contact.id, grip));
        Some(Vec::new())
    }
}
