//! egui's input, taken from the normaliser's batches: the first contact of each batch drives
//! egui's single pointer, in points. No windowing library sits in between (DECISIONS D11).

use openpaint_app::{ContactPhase, InputBatch};

/// Where a cancelled contact is released: off the screen, further from any press than egui's
/// click distance, so a cancel never taps the button it rested on.
const NOWHERE: egui::Pos2 = egui::pos2(-100_000.0, -100_000.0);

/// The pointer events of `batches` for a screen of `pixels_per_point`.
pub(crate) fn pointer_events(batches: &[InputBatch], pixels_per_point: f32) -> Vec<egui::Event> {
    let mut events = Vec::new();
    for contact in batches.iter().filter_map(|batch| batch.contacts.first()) {
        let Some(last) = contact.samples.last() else {
            continue;
        };
        let pos = egui::pos2(last.position[0], last.position[1]) / pixels_per_point;
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        match contact.phase {
            ContactPhase::Hover | ContactPhase::Move => {
                events.push(egui::Event::PointerMoved(pos));
            }
            ContactPhase::Down => {
                events.extend([egui::Event::PointerMoved(pos), button(pos, true)]);
            }
            ContactPhase::Up => {
                events.extend([
                    egui::Event::PointerMoved(pos),
                    button(pos, false),
                    egui::Event::PointerGone,
                ]);
            }
            // egui has no cancel: the press ends far away, which is a release but no click.
            ContactPhase::Cancel => events.extend([
                egui::Event::PointerMoved(NOWHERE),
                button(NOWHERE, false),
                egui::Event::PointerGone,
            ]),
            ContactPhase::Leave => events.push(egui::Event::PointerGone),
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_app::{Contact, Device, InputSample};

    fn batch(phase: ContactPhase) -> InputBatch {
        let sample = InputSample {
            position: [40.0, 80.0],
            pressure: 1.0,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms: 0,
        };
        InputBatch {
            contacts: vec![Contact {
                id: 0,
                device: Device::Finger,
                phase,
                samples: vec![sample],
            }],
        }
    }

    #[test]
    fn a_tap_presses_and_releases_at_its_point_in_points() {
        let batches = [batch(ContactPhase::Down), batch(ContactPhase::Up)];
        let events = pointer_events(&batches, 2.0);
        let at = egui::pos2(20.0, 40.0);
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let expected = [
            egui::Event::PointerMoved(at),
            press(true),
            egui::Event::PointerMoved(at),
            press(false),
            egui::Event::PointerGone,
        ];
        assert_eq!(events, expected);
    }

    #[test]
    fn a_cancel_releases_off_the_screen() {
        let events = pointer_events(&[batch(ContactPhase::Cancel)], 2.0);
        let released = egui::Event::PointerButton {
            pos: NOWHERE,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert_eq!(events[1], released);
        assert_eq!(events.last(), Some(&egui::Event::PointerGone));
    }
}
