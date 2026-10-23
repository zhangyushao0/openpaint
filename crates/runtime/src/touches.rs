//! Whether the screen is idle for background work (DECISIONS D96): a frame gives it
//! `BACKGROUND_NS` (4 ms of GPU) while a finger or the pen touches the screen or the pen hovers
//! over it (a stroke is coming), and for `IDLE_AFTER` after the last of these; once nothing has for
//! that long, background work runs at the GPU's pace between the frames drawn, the GPU's queue of
//! it kept within `IDLE_BACKGROUND_NS`, so the first frame of a touch waits for at most that much.
//! A hand arriving is seen as its input comes (`Runtime::motion`), not at the next frame: the
//! raster worker drops the tiles it was handed while nothing touched at once. Times are the
//! frame clock's (`clock.rs`): real time on a device, the held time in the harness and tests.

use openpaint_app::{ContactPhase, Device, InputBatch};

/// Seconds after the last touch or hover that background work keeps the smaller budget.
pub(crate) const IDLE_AFTER: f64 = 0.5;

#[derive(Debug, Default)]
pub(crate) struct Touches {
    /// When a finger or the pen last touched or hovered, on the frame clock; `None` before the
    /// first.
    last: Option<f64>,
    /// The contacts on the glass or hovering over it now, by device and id.
    on: Vec<(Device, i32)>,
}

impl Touches {
    /// Input `batches` arrived at `now`: returns whether a hand is in them (a contact touching,
    /// moving, lifting or hovering).
    pub(crate) fn saw(&mut self, batches: &[InputBatch], now: f64) -> bool {
        let mut hand = false;
        for contact in batches.iter().flat_map(|batch| &batch.contacts) {
            hand = true;
            let key = (contact.device, contact.id);
            let here = self.on.iter().position(|on| *on == key);
            match (contact.phase, here) {
                (ContactPhase::Up | ContactPhase::Cancel | ContactPhase::Leave, Some(at)) => {
                    self.on.swap_remove(at);
                }
                (ContactPhase::Down | ContactPhase::Move | ContactPhase::Hover, None) => {
                    self.on.push(key);
                }
                _ => {}
            }
        }
        if hand {
            self.last = Some(now);
        }
        hand
    }

    /// No contact is on the glass or hovering over it, and no stroke is drawn (`drawing`): a
    /// frame has no input to take, so it may wait for the one before (`pacing.rs`).
    pub(crate) fn resting(&self, drawing: bool) -> bool {
        self.on.is_empty() && !drawing
    }

    /// Whether the screen is idle at `now`; `drawing` while a stroke is drawn (a pen held still
    /// on the glass sends nothing).
    pub(crate) fn idle(&mut self, drawing: bool, now: f64) -> bool {
        if drawing {
            self.last = Some(now);
        }
        self.last.is_none_or(|last| now - last >= IDLE_AFTER)
    }
}

#[cfg(test)]
mod tests {
    use openpaint_app::{Contact, ContactPhase, Device, InputBatch, InputSample};

    use super::Touches;

    fn pen(phase: ContactPhase) -> InputBatch {
        InputBatch {
            contacts: vec![Contact {
                id: 0,
                device: Device::Pen,
                phase,
                samples: vec![InputSample {
                    position: [10.0, 10.0],
                    pressure: 0.0,
                    tilt: 0.0,
                    azimuth: 0.0,
                    time_ms: 0,
                }],
            }],
            ..InputBatch::default()
        }
    }

    #[test]
    fn a_pen_hovering_counts_as_a_hand_and_ends_the_idle_screen() {
        let mut touches = Touches::default();
        assert!(touches.idle(false, 1.0), "nothing touched yet");
        assert!(!touches.saw(&[InputBatch::default()], 1.0), "no contact");
        assert!(touches.idle(false, 1.0) && touches.resting(false));
        assert!(
            touches.saw(&[pen(ContactPhase::Hover)], 1.0),
            "the pen coming into hover"
        );
        assert!(!touches.idle(false, 1.4), "a stroke is coming");
        assert!(!touches.resting(false), "the pen is over the glass");
        touches.saw(&[pen(ContactPhase::Down), pen(ContactPhase::Up)], 1.4);
        assert!(touches.resting(false), "lifted");
        assert!(touches.idle(false, 1.9), "half a second after the lift");
        touches.saw(&[pen(ContactPhase::Hover)], 2.0);
        assert!(!touches.resting(false), "hovering again");
        touches.saw(&[pen(ContactPhase::Leave)], 2.0);
        assert!(touches.resting(false), "out of range");
        assert!(!touches.resting(true), "but for a stroke drawn");
        let mut drawing = Touches::default();
        assert!(!drawing.idle(true, 1.0), "a pen held still on the glass");
    }
}
