//! Who owns the reference window's contacts, from the raw input batches (release positions
//! included), before the canvas sees them. A contact that touches down on the window belongs to
//! it until it lifts: on the bar's grip it moves the window, on the corner mark it resizes it,
//! on the bar's buttons it is egui's (a tap on ×); on the picture, while the eyedropper is on,
//! it previews colours and picks; otherwise fingers and pen go to the picture's gestures (a tap
//! toggles the bar, fingers pan and pinch). The canvas never owns these contacts and its
//! gestures never navigate with them: they did not touch down on open canvas.

use egui::{Pos2, Rect};
use openpaint_app::{Contact, ContactPhase, Device, InputBatch, Intent, ToolIntent, Touch};

use crate::reference_window::{BAR_IDLE_MS, ReferenceWindow, kept_on, resized};

/// What a contact on the window does.
#[derive(Debug, Clone, Copy)]
enum Role {
    /// Moves or resizes the window from where it was (`from`) as the contact leaves `start`.
    Move {
        start: Pos2,
        from: Rect,
    },
    Resize {
        start: Pos2,
        from: Rect,
    },
    /// A button on the bar: egui's.
    Button,
    Picture,
    Pick,
}

#[derive(Debug, Default)]
pub(crate) struct WindowTouch {
    owned: Vec<(i32, Role)>,
    /// Where the contact picking from the picture is, in screen pixels.
    picking_at: Option<[f32; 2]>,
}

impl ReferenceWindow {
    /// The intents of the contacts on the window in `batches` (colour previews and picks);
    /// moves, resizes, the bar and the picture's view change here.
    pub(crate) fn route(
        &mut self,
        batches: &[InputBatch],
        now_ms: i64,
        picking: bool,
    ) -> Vec<Intent> {
        let mut intents = Vec::new();
        if self.layout.is_none() {
            return intents;
        }
        let (mut fingers, mut pens) = (Vec::new(), Vec::new());
        for contact in batches.iter().flat_map(|batch| &batch.contacts) {
            let Some(role) = self.role(contact, now_ms, picking) else {
                continue;
            };
            let ends = matches!(contact.phase, ContactPhase::Up | ContactPhase::Cancel);
            if ends {
                self.touch.owned.retain(|(id, _)| *id != contact.id);
            } else if self.bar_until.is_some() {
                self.bar_until = Some(now_ms + BAR_IDLE_MS);
            }
            let at = self.point(contact);
            match role {
                Role::Move { start, from } => self.place = Some(from.translate(at - start)),
                Role::Resize { start, from } => {
                    let screen = self.layout.map(|(_, screen)| screen).unwrap_or(from);
                    self.place = Some(resized(screen, from, at - start));
                }
                Role::Button => {}
                Role::Pick => intents.extend(self.pick(contact, at)),
                Role::Picture => {
                    let touch = touch(contact);
                    if contact.device == Device::Finger {
                        fingers.extend(touch);
                    } else {
                        pens.extend(touch);
                    }
                }
            }
        }
        if let Some((layout, screen)) = self.layout {
            self.place = self.place.map(|place| kept_on(screen, place));
            self.gesture(&fingers, &pens, layout.picture, now_ms);
        }
        intents
    }

    /// The role of `contact`: its own once it touched down on the window. A touch down
    /// elsewhere hides the bar.
    fn role(&mut self, contact: &Contact, now_ms: i64, picking: bool) -> Option<Role> {
        let owned = self.touch.owned.iter().find(|(id, _)| *id == contact.id);
        if contact.phase != ContactPhase::Down {
            return owned.map(|(_, role)| *role);
        }
        let (layout, _) = self.layout?;
        let at = self.point(contact);
        if !layout.window.contains(at) {
            self.bar_until = None;
            return None;
        }
        let from = self.place.unwrap_or(layout.window);
        let on = |rect: Option<Rect>| rect.is_some_and(|rect| rect.contains(at));
        let role = if on(layout.grip) {
            Role::Move { start: at, from }
        } else if on(layout.resize) {
            Role::Resize { start: at, from }
        } else if on(layout.bar) {
            Role::Button
        } else if picking {
            Role::Pick
        } else {
            Role::Picture
        };
        if self.bar_until.is_some() {
            self.bar_until = Some(now_ms + BAR_IDLE_MS);
        }
        self.touch.owned.push((contact.id, role));
        Some(role)
    }

    /// The picking contact at `at` (points): a preview of the image pixel under it while it is
    /// on the image, the last preview picked as it lifts.
    fn pick(&mut self, contact: &Contact, at: Pos2) -> Option<Intent> {
        let ppp = self.pixels_per_point;
        match contact.phase {
            ContactPhase::Up => {
                self.touch.picking_at = None;
                return Some(Intent::Tool(ToolIntent::Pick));
            }
            ContactPhase::Cancel => {
                self.touch.picking_at = None;
                return Some(Intent::Tool(ToolIntent::Eyedropper(false)));
            }
            ContactPhase::Hover | ContactPhase::Leave => return None,
            ContactPhase::Down | ContactPhase::Move => {}
        }
        self.touch.picking_at = Some([at.x * ppp, at.y * ppp]);
        let (layout, _) = self.layout?;
        let image = self.image()?;
        if !layout.picture.contains(at) {
            return None;
        }
        let (shown, scale) = self.shown_image(layout.picture, image);
        let pixel = (at - shown.min) / scale;
        if pixel.x < 0.0 || pixel.y < 0.0 {
            return None;
        }
        let color = image.color_at([pixel.x as u32, pixel.y as u32])?;
        Some(Intent::Tool(ToolIntent::Preview(color)))
    }

    /// The picture's gestures this frame: a tap shows or hides the bar, fingers pan and pinch.
    fn gesture(&mut self, fingers: &[Touch], pens: &[Touch], picture: Rect, now_ms: i64) {
        let gestured = self.gestures.update(fingers, pens, self.pixels_per_point);
        if gestured.tap {
            self.bar_until = match self.bar_until {
                Some(_) => None,
                None => Some(now_ms + BAR_IDLE_MS),
            };
        }
        if let Some(view) = gestured.view
            && let Some(image) = self.image().cloned()
        {
            self.navigate(view, picture, &image);
        }
    }

    /// Where the contact picking from the picture is, in screen pixels.
    pub(crate) const fn picking_at(&self) -> Option<[f32; 2]> {
        self.touch.picking_at
    }

    /// Where `contact` is now, in points.
    fn point(&self, contact: &Contact) -> Pos2 {
        let [x, y] = contact
            .samples
            .last()
            .map_or([0.0; 2], |sample| sample.position);
        Pos2::new(x, y) / self.pixels_per_point
    }
}

#[cfg(test)]
#[path = "reference_touch_tests.rs"]
mod tests;

/// `contact` as the picture's gestures see it: its current position, in screen pixels.
fn touch(contact: &Contact) -> Option<Touch> {
    let last = contact.samples.last()?;
    Some(Touch {
        id: contact.id,
        phase: contact.phase,
        position: last.position,
        time_ms: last.time_ms,
        open: true,
    })
}
