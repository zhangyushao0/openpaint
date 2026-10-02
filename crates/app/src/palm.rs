//! Palm rejection: while the pen is on the glass, touches are a resting hand, never a finger.
//! The runtime passes every normalised batch through here before the UI or the session sees
//! it, so a palm can neither paint, nor press a button, nor disturb the stroke.
//!
//! - A touch that lands while a pen contact is down is a palm: it is dropped until it lifts,
//!   even if the pen lifts first.
//! - A pen that lands while touches are down turns those touches into palms: each is cancelled
//!   (a batch of its own, before the pen's) and dropped until it lifts, so a hand that rested
//!   on a button before the pen landed never taps it.
//! - Touches the platform cancels arrive as `Cancel` and never count as a lift.
//!
//! Only the stylus paints (D18), so pen contacts are never filtered; a hovering pen does not
//! count as down, so a finger works whenever the pen is off the glass.

use crate::input::{Contact, ContactPhase, Device, InputBatch, InputSample};

#[derive(Debug, Default)]
pub struct PalmRejection {
    /// Pen and eraser contacts on the glass.
    pens: Vec<i32>,
    /// Touches down and passed on, with their last sample.
    touches: Vec<(i32, Device, InputSample)>,
    /// Touches judged to be palms, until they lift.
    palms: Vec<i32>,
}

impl PalmRejection {
    /// `batch` without its palms. A pen landing on resting touches first yields a batch that
    /// cancels them. Empty when nothing is left.
    pub fn filter(&mut self, batch: InputBatch) -> Vec<InputBatch> {
        let mut cancels = Vec::new();
        let mut kept = Vec::with_capacity(batch.contacts.len());
        for contact in batch.contacts {
            if is_pen(contact.device) {
                if contact.phase == ContactPhase::Down {
                    cancels.extend(self.reject_touches());
                }
                self.track_pen(&contact);
                kept.push(contact);
            } else if self.pass_touch(&contact) {
                kept.push(contact);
            }
        }
        [cancels, kept]
            .into_iter()
            .filter(|contacts| !contacts.is_empty())
            .map(|contacts| InputBatch { contacts })
            .collect()
    }

    /// Every touch passed on so far becomes a palm, cancelled at its last sample.
    fn reject_touches(&mut self) -> Vec<Contact> {
        let touches = std::mem::take(&mut self.touches);
        touches
            .into_iter()
            .map(|(id, device, last)| {
                self.palms.push(id);
                Contact {
                    id,
                    device,
                    phase: ContactPhase::Cancel,
                    samples: vec![last],
                }
            })
            .collect()
    }

    fn track_pen(&mut self, contact: &Contact) {
        match contact.phase {
            ContactPhase::Down if !self.pens.contains(&contact.id) => self.pens.push(contact.id),
            ContactPhase::Up | ContactPhase::Cancel => self.pens.retain(|id| *id != contact.id),
            _ => {}
        }
    }

    /// Whether `contact`, a touch, is passed on; keeps its state either way.
    fn pass_touch(&mut self, contact: &Contact) -> bool {
        let ends = matches!(
            contact.phase,
            ContactPhase::Up | ContactPhase::Cancel | ContactPhase::Leave
        );
        if self.palms.contains(&contact.id) {
            if ends {
                self.palms.retain(|id| *id != contact.id);
            }
            return false;
        }
        if contact.phase == ContactPhase::Down && !self.pens.is_empty() {
            self.palms.push(contact.id);
            return false;
        }
        self.touches.retain(|(id, _, _)| *id != contact.id);
        let touching = matches!(contact.phase, ContactPhase::Down | ContactPhase::Move);
        if touching && let Some(last) = contact.samples.last() {
            self.touches.push((contact.id, contact.device, *last));
        }
        true
    }
}

/// The pen tip and its eraser end; everything else may be a hand.
fn is_pen(device: Device) -> bool {
    matches!(device, Device::Pen | Device::Eraser)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(contacts: &[(i32, Device, ContactPhase)]) -> InputBatch {
        let sample = InputSample {
            position: [10.0, 20.0],
            pressure: 0.5,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms: 0,
        };
        InputBatch {
            contacts: contacts
                .iter()
                .map(|&(id, device, phase)| Contact {
                    id,
                    device,
                    phase,
                    samples: vec![sample],
                })
                .collect(),
        }
    }

    /// `(id, phase)` of every contact passed on, one inner list per batch.
    fn passed(
        filter: &mut PalmRejection,
        contacts: &[(i32, Device, ContactPhase)],
    ) -> Vec<Vec<(i32, ContactPhase)>> {
        filter
            .filter(batch(contacts))
            .iter()
            .map(|batch| batch.contacts.iter().map(|c| (c.id, c.phase)).collect())
            .collect()
    }

    use ContactPhase::{Cancel, Down, Hover, Move, Up};
    use Device::{Finger, Pen};

    #[test]
    fn a_touch_that_lands_while_the_pen_draws_is_dropped_until_it_lifts() {
        let mut filter = PalmRejection::default();
        assert_eq!(passed(&mut filter, &[(0, Pen, Down)]), [[(0, Down)]]);
        assert!(passed(&mut filter, &[(5, Finger, Down)]).is_empty());
        assert_eq!(
            passed(&mut filter, &[(0, Pen, Move), (5, Finger, Move)]),
            [[(0, Move)]]
        );
        assert_eq!(passed(&mut filter, &[(0, Pen, Up)]), [[(0, Up)]]);
        // The pen is off the glass, but the palm stays a palm until it lifts.
        assert!(passed(&mut filter, &[(5, Finger, Move)]).is_empty());
        assert!(passed(&mut filter, &[(5, Finger, Up)]).is_empty());
        // Then a finger is a finger again.
        assert_eq!(passed(&mut filter, &[(6, Finger, Down)]), [[(6, Down)]]);
    }

    #[test]
    fn a_pen_landing_cancels_the_touches_resting_before_it() {
        let mut filter = PalmRejection::default();
        assert_eq!(passed(&mut filter, &[(3, Finger, Down)]), [[(3, Down)]]);
        assert_eq!(passed(&mut filter, &[(3, Finger, Move)]), [[(3, Move)]]);
        let landed = filter.filter(batch(&[(0, Pen, Down)]));
        assert_eq!(
            landed.len(),
            2,
            "the cancel comes in a batch of its own, first"
        );
        let cancel = &landed[0].contacts[0];
        assert_eq!(
            (cancel.id, cancel.device, cancel.phase),
            (3, Finger, Cancel)
        );
        assert_eq!(cancel.samples.len(), 1);
        assert_eq!(landed[1].contacts[0].phase, Down);
        assert_eq!(passed(&mut filter, &[(0, Pen, Up)]), [[(0, Up)]]);
        assert!(passed(&mut filter, &[(3, Finger, Up)]).is_empty());
    }

    #[test]
    fn a_hovering_pen_leaves_fingers_alone() {
        let mut filter = PalmRejection::default();
        assert_eq!(passed(&mut filter, &[(0, Pen, Hover)]), [[(0, Hover)]]);
        assert_eq!(passed(&mut filter, &[(2, Finger, Down)]), [[(2, Down)]]);
        assert_eq!(passed(&mut filter, &[(2, Finger, Up)]), [[(2, Up)]]);
    }

    #[test]
    fn a_cancelled_pen_frees_touches_and_a_lifted_touch_is_not_cancelled_later() {
        let mut filter = PalmRejection::default();
        assert_eq!(passed(&mut filter, &[(2, Finger, Down)]), [[(2, Down)]]);
        assert_eq!(passed(&mut filter, &[(2, Finger, Up)]), [[(2, Up)]]);
        assert_eq!(passed(&mut filter, &[(0, Pen, Down)]), [[(0, Down)]]);
        assert_eq!(passed(&mut filter, &[(0, Pen, Cancel)]), [[(0, Cancel)]]);
        assert_eq!(passed(&mut filter, &[(4, Finger, Down)]), [[(4, Down)]]);
    }

    #[test]
    fn the_eraser_end_counts_as_the_pen() {
        let mut filter = PalmRejection::default();
        assert_eq!(
            passed(&mut filter, &[(1, Device::Eraser, Down)]),
            [[(1, Down)]]
        );
        assert!(passed(&mut filter, &[(2, Device::Unknown, Down)]).is_empty());
    }
}
