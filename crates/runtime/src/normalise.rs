//! The normaliser: platform batches (`MotionBatch`, from the Android host or a harness
//! recording) become `InputBatch`es, the same on every host. Each pointer gets its own phase;
//! values outside their ranges are clamped; samples with a non-finite position are dropped;
//! times never go back within a contact.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use openpaint_app::{Contact, ContactPhase, Device, InputBatch, InputSample};

use crate::input::{MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};

/// Keeps, per pointer down or hovering, the time of its last sample.
#[derive(Debug, Default)]
pub struct Normaliser {
    last_ms: Vec<(i32, i64)>,
}

impl Normaliser {
    /// The normalised batch, or `None` when it holds nothing the product uses (button-only
    /// events, pointers without a usable sample).
    pub fn normalise(&mut self, batch: &MotionBatch) -> Option<InputBatch> {
        let mut contacts = Vec::with_capacity(batch.pointers.len());
        for (index, pointer) in batch.pointers.iter().enumerate() {
            let Some(phase) = phase(batch.phase, index) else {
                continue;
            };
            let samples = self.samples(pointer);
            if matches!(
                phase,
                ContactPhase::Up | ContactPhase::Cancel | ContactPhase::Leave
            ) {
                self.last_ms.retain(|(id, _)| *id != pointer.id);
            }
            if samples.is_empty() {
                continue;
            }
            contacts.push(Contact {
                id: pointer.id,
                device: device(pointer.tool),
                phase,
                samples,
            });
        }
        (!contacts.is_empty()).then_some(InputBatch { contacts })
    }

    fn samples(&mut self, pointer: &Pointer) -> Vec<InputSample> {
        let slot = match self.last_ms.iter().position(|(id, _)| *id == pointer.id) {
            Some(slot) => slot,
            None => {
                self.last_ms.push((pointer.id, i64::MIN));
                self.last_ms.len() - 1
            }
        };
        let mut last = self.last_ms[slot].1;
        let samples = pointer
            .samples
            .iter()
            .filter(|sample| sample.position.iter().all(|value| value.is_finite()))
            .map(|sample| {
                last = last.max(sample.time_ms);
                normalised(sample, last)
            })
            .collect();
        self.last_ms[slot].1 = last;
        samples
    }
}

/// What `pointer` (at `index` in its batch) did, or `None` for batches without contact news.
fn phase(batch: MotionPhase, index: usize) -> Option<ContactPhase> {
    Some(match batch {
        MotionPhase::Down => ContactPhase::Down,
        MotionPhase::Move => ContactPhase::Move,
        MotionPhase::Up => ContactPhase::Up,
        MotionPhase::Cancel => ContactPhase::Cancel,
        MotionPhase::PointerDown { index: down } if down == index => ContactPhase::Down,
        MotionPhase::PointerUp { index: up } if up == index => ContactPhase::Up,
        MotionPhase::PointerDown { .. } | MotionPhase::PointerUp { .. } => ContactPhase::Move,
        MotionPhase::HoverEnter | MotionPhase::HoverMove => ContactPhase::Hover,
        MotionPhase::HoverExit => ContactPhase::Leave,
        MotionPhase::ButtonPress | MotionPhase::ButtonRelease => return None,
    })
}

fn device(tool: ToolKind) -> Device {
    match tool {
        ToolKind::Stylus => Device::Pen,
        ToolKind::Eraser => Device::Eraser,
        ToolKind::Finger => Device::Finger,
        ToolKind::Mouse => Device::Mouse,
        ToolKind::Unknown => Device::Unknown,
    }
}

fn normalised(sample: &PointerSample, time_ms: i64) -> InputSample {
    InputSample {
        position: sample.position,
        pressure: clamped(sample.pressure, 1.0),
        tilt: clamped(sample.tilt, FRAC_PI_2),
        azimuth: wrapped(sample.orientation),
        time_ms,
    }
}

/// `value` in `0..=max`; not a number counts as 0.
fn clamped(value: f32, max: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, max)
    }
}

/// `angle` in `-pi..=pi`; not finite counts as 0.
fn wrapped(angle: f32) -> f32 {
    if !angle.is_finite() {
        return 0.0;
    }
    let turned = (angle + PI).rem_euclid(TAU) - PI;
    turned.clamp(-PI, PI)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Buttons;

    fn sample(x: f32, pressure: f32, time_ms: i64) -> PointerSample {
        PointerSample {
            position: [x, 20.0],
            pressure,
            tilt: 0.5,
            orientation: 0.25,
            time_ms,
        }
    }

    fn pointer(id: i32, tool: ToolKind, samples: Vec<PointerSample>) -> Pointer {
        Pointer { id, tool, samples }
    }

    fn batch(phase: MotionPhase, pointers: Vec<Pointer>) -> MotionBatch {
        MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers,
        }
    }

    fn phases(batch: &InputBatch) -> Vec<(i32, ContactPhase)> {
        batch.contacts.iter().map(|c| (c.id, c.phase)).collect()
    }

    #[test]
    fn a_pen_stroke_keeps_its_history_samples_in_order() {
        let mut normaliser = Normaliser::default();
        let samples = vec![sample(1.0, 0.2, 100), sample(2.0, 0.4, 104)];
        let moved = batch(
            MotionPhase::Move,
            vec![pointer(0, ToolKind::Stylus, samples)],
        );
        let batch = normaliser.normalise(&moved).expect("batch");
        let contact = &batch.contacts[0];
        assert_eq!(
            (contact.device, contact.phase),
            (Device::Pen, ContactPhase::Move)
        );
        let xs: Vec<f32> = contact.samples.iter().map(|s| s.position[0]).collect();
        assert_eq!(xs, [1.0, 2.0]);
        assert_eq!(contact.samples[1].pressure, 0.4);
        assert_eq!(contact.samples[1].tilt, 0.5);
        assert_eq!(contact.samples[1].azimuth, 0.25);
        assert_eq!(contact.samples[1].time_ms, 104);
    }

    #[test]
    fn each_pointer_gets_its_own_phase() {
        let mut normaliser = Normaliser::default();
        let two = |phase| {
            batch(
                phase,
                vec![
                    pointer(4, ToolKind::Finger, vec![sample(1.0, 1.0, 5)]),
                    pointer(9, ToolKind::Finger, vec![sample(9.0, 1.0, 5)]),
                ],
            )
        };
        let down = normaliser.normalise(&two(MotionPhase::PointerDown { index: 1 }));
        let up = normaliser.normalise(&two(MotionPhase::PointerUp { index: 0 }));
        let cancel = normaliser.normalise(&two(MotionPhase::Cancel));
        use ContactPhase::{Cancel, Down, Move, Up};
        assert_eq!(phases(&down.expect("down")), [(4, Move), (9, Down)]);
        assert_eq!(phases(&up.expect("up")), [(4, Up), (9, Move)]);
        assert_eq!(phases(&cancel.expect("cancel")), [(4, Cancel), (9, Cancel)]);
    }

    #[test]
    fn values_are_clamped_into_their_ranges() {
        let mut normaliser = Normaliser::default();
        let mut wild = sample(1.0, 1.08, 0);
        wild.tilt = 2.0;
        wild.orientation = 4.0;
        let mut negative = sample(2.0, -0.1, 1);
        negative.tilt = f32::NAN;
        negative.orientation = -4.0;
        let samples = vec![wild, negative];
        let hover = batch(
            MotionPhase::HoverMove,
            vec![pointer(0, ToolKind::Stylus, samples)],
        );
        let batch = normaliser.normalise(&hover).expect("batch");
        let [a, b] = [batch.contacts[0].samples[0], batch.contacts[0].samples[1]];
        assert_eq!(batch.contacts[0].phase, ContactPhase::Hover);
        assert_eq!((a.pressure, a.tilt), (1.0, FRAC_PI_2));
        assert!((a.azimuth - (4.0 - TAU)).abs() < 1e-6);
        assert_eq!((b.pressure, b.tilt), (0.0, 0.0));
        assert!((b.azimuth - (TAU - 4.0)).abs() < 1e-6);
    }

    #[test]
    fn time_never_goes_back_within_a_contact_and_restarts_after_it() {
        let mut normaliser = Normaliser::default();
        let stylus = |phase, samples| batch(phase, vec![pointer(0, ToolKind::Stylus, samples)]);
        let times = |batch: Option<InputBatch>| -> Vec<i64> {
            let batch = batch.expect("batch");
            batch.contacts[0]
                .samples
                .iter()
                .map(|s| s.time_ms)
                .collect()
        };
        let down = normaliser.normalise(&stylus(MotionPhase::Down, vec![sample(0.0, 1.0, 50)]));
        assert_eq!(times(down), [50]);
        let back = vec![sample(1.0, 1.0, 40), sample(2.0, 1.0, 60)];
        let moved = normaliser.normalise(&stylus(MotionPhase::Move, back));
        assert_eq!(times(moved), [50, 60]);
        let up = normaliser.normalise(&stylus(MotionPhase::Up, vec![sample(3.0, 1.0, 55)]));
        assert_eq!(times(up), [60]);
        let again = normaliser.normalise(&stylus(MotionPhase::Down, vec![sample(0.0, 1.0, 10)]));
        assert_eq!(times(again), [10]);
    }

    #[test]
    fn unusable_samples_and_button_events_are_dropped() {
        let mut normaliser = Normaliser::default();
        let lost = vec![sample(f32::NAN, 1.0, 0), sample(f32::INFINITY, 1.0, 1)];
        let moved = batch(MotionPhase::Move, vec![pointer(0, ToolKind::Stylus, lost)]);
        assert_eq!(normaliser.normalise(&moved), None);
        let press = batch(
            MotionPhase::ButtonPress,
            vec![pointer(0, ToolKind::Stylus, vec![sample(1.0, 0.0, 2)])],
        );
        assert_eq!(normaliser.normalise(&press), None);
        let mixed = vec![sample(f32::NAN, 1.0, 3), sample(4.0, 1.0, 4)];
        let moved = batch(MotionPhase::Move, vec![pointer(0, ToolKind::Stylus, mixed)]);
        let batch = normaliser.normalise(&moved).expect("batch");
        assert_eq!(batch.contacts[0].samples.len(), 1);
    }
}
