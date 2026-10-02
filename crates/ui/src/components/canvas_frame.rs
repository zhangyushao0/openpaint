//! The canvas frame: the screen area that shows the canvas and owns the contacts that touch
//! down in it. It paints nothing, so the renderer's canvas shows through, and nothing covers
//! it while a stroke is possible.
//!
//! Contacts are routed outside the egui pass, from the area the last pass laid out: egui may
//! run a pass twice, and an intent must never be emitted twice.

use openpaint_app::{ContactPhase, InputBatch, Intent, StrokeIntent};

/// A rectangle of screen pixels: `origin` is its top-left pixel, where canvas pixel (0, 0)
/// shows (one screen pixel per canvas pixel until the view can zoom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenArea {
    pub origin: [i32; 2],
    pub size: [u32; 2],
}

impl ScreenArea {
    /// `rect` in points on a screen of `pixels_per_point`, rounded to whole pixels.
    #[must_use]
    pub fn from_points(rect: egui::Rect, pixels_per_point: f32) -> Self {
        let pixel = |points: f32| (points * pixels_per_point).round();
        let [x0, y0, x1, y1] = [rect.min.x, rect.min.y, rect.max.x, rect.max.y].map(pixel);
        Self {
            origin: [x0 as i32, y0 as i32],
            size: [(x1 - x0).max(0.0) as u32, (y1 - y0).max(0.0) as u32],
        }
    }

    fn contains(&self, [x, y]: [f32; 2]) -> bool {
        let [left, top] = self.origin.map(|side| side as f32);
        let [width, height] = self.size.map(|side| side as f32);
        (left..left + width).contains(&x) && (top..top + height).contains(&y)
    }
}

/// Lays out the canvas frame in the space the other panels left and returns it, in points.
pub fn canvas_frame(ui: &mut egui::Ui) -> egui::Rect {
    egui::CentralPanel::no_frame()
        .show(ui, |ui| ui.max_rect())
        .inner
}

/// Which contacts the canvas owns: each one that touches down inside its area, until it lifts
/// or is cancelled, wherever it moves meanwhile.
#[derive(Debug, Default)]
pub struct CanvasInput {
    owned: Vec<i32>,
}

impl CanvasInput {
    /// The owned contacts of `batches` as stroke intents, positions moved into canvas pixels.
    /// Hover does not reach the canvas yet.
    pub fn route(&mut self, area: &ScreenArea, batches: &[InputBatch]) -> Vec<Intent> {
        let mut intents = Vec::new();
        for contact in batches.iter().flat_map(|batch| &batch.contacts) {
            let starts_here = contact.phase == ContactPhase::Down
                && contact
                    .samples
                    .first()
                    .is_some_and(|first| area.contains(first.position));
            if starts_here && !self.owned.contains(&contact.id) {
                self.owned.push(contact.id);
            }
            if !self.owned.contains(&contact.id) {
                continue;
            }
            if matches!(contact.phase, ContactPhase::Up | ContactPhase::Cancel) {
                self.owned.retain(|id| *id != contact.id);
            }
            if matches!(contact.phase, ContactPhase::Hover | ContactPhase::Leave) {
                continue;
            }
            let mut moved = contact.clone();
            let [x, y] = area.origin.map(|side| side as f32);
            for sample in &mut moved.samples {
                sample.position = [sample.position[0] - x, sample.position[1] - y];
            }
            intents.push(Intent::Stroke(StrokeIntent::Contact(moved)));
        }
        intents
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_app::{Contact, Device, InputSample};

    const AREA: ScreenArea = ScreenArea {
        origin: [0, 84],
        size: [400, 300],
    };

    fn batch(id: i32, phase: ContactPhase, x: f32, y: f32) -> InputBatch {
        let sample = InputSample {
            position: [x, y],
            pressure: 1.0,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms: 0,
        };
        InputBatch {
            contacts: vec![Contact {
                id,
                device: Device::Pen,
                phase,
                samples: vec![sample],
            }],
        }
    }

    fn positions(intents: &[Intent]) -> Vec<[f32; 2]> {
        intents
            .iter()
            .map(|Intent::Stroke(StrokeIntent::Contact(contact))| contact.samples[0].position)
            .collect()
    }

    #[test]
    fn a_contact_that_starts_on_the_canvas_is_owned_until_it_lifts() {
        let mut input = CanvasInput::default();
        let batches = [
            batch(1, ContactPhase::Down, 10.0, 100.0),
            // Moving over the top bar keeps drawing.
            batch(1, ContactPhase::Move, 12.0, 20.0),
            batch(1, ContactPhase::Up, 14.0, 90.0),
            batch(1, ContactPhase::Move, 16.0, 120.0),
        ];
        let intents = input.route(&AREA, &batches);
        assert_eq!(
            positions(&intents),
            [[10.0, 16.0], [12.0, -64.0], [14.0, 6.0]]
        );
    }

    #[test]
    fn contacts_that_start_off_the_canvas_are_not_routed() {
        let mut input = CanvasInput::default();
        let batches = [
            batch(2, ContactPhase::Down, 10.0, 40.0),
            batch(2, ContactPhase::Move, 10.0, 200.0),
            batch(2, ContactPhase::Up, 10.0, 210.0),
            batch(3, ContactPhase::Down, 500.0, 200.0),
        ];
        assert!(input.route(&AREA, &batches).is_empty());
    }

    #[test]
    fn cancel_is_passed_on_and_ends_ownership() {
        let mut input = CanvasInput::default();
        let batches = [
            batch(1, ContactPhase::Down, 10.0, 100.0),
            batch(1, ContactPhase::Cancel, 10.0, 100.0),
            batch(1, ContactPhase::Move, 10.0, 110.0),
            batch(1, ContactPhase::Hover, 10.0, 110.0),
        ];
        assert_eq!(input.route(&AREA, &batches).len(), 2);
    }

    #[test]
    fn the_area_rounds_points_to_whole_pixels() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 48.0), egui::pos2(1645.7, 1028.6));
        let area = ScreenArea::from_points(rect, 1.75);
        assert_eq!(area.origin, [0, 84]);
        assert_eq!(area.size, [2880, 1716]);
    }
}
