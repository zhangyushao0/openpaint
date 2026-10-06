//! The normaliser: platform batches (`MotionBatch`, from the Android host or a harness
//! recording) become `InputBatch`es, the same on every host. Each pointer gets its own phase;
//! values outside their ranges are clamped; samples with a non-finite position are dropped;
//! times never go back within a contact.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use openpaint_app::{Contact, ContactPhase, Device, InputBatch, InputSample, PenButtons};

use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};

/// Keeps, per pointer down or hovering, the time of its last sample, and the pen's buttons as
/// they were last reported.
#[derive(Debug, Default)]
pub struct Normaliser {
    last_ms: Vec<(i32, i64)>,
    pen: Buttons,
}

impl Normaliser {
    /// The normalised batch, or `None` when it holds nothing the product uses (pointers without
    /// a usable sample). A pen's batch carries its side buttons; a pen's button press or release
    /// with no other change is a batch of no contacts (S41).
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
        let pen = pen_buttons(batch);
        if pen.is_some() && batch.buttons != self.pen {
            // What the platform reports for the pen's buttons, for the author's pen to confirm.
            log::info!("pen buttons {:?} ({:?})", batch.buttons, batch.phase);
            self.pen = batch.buttons;
        }
        let button = matches!(
            batch.phase,
            MotionPhase::ButtonPress | MotionPhase::ButtonRelease
        );
        (!contacts.is_empty() || (button && pen.is_some())).then_some(InputBatch { contacts, pen })
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
        MotionPhase::PointerCancel { index: gone } if gone == index => ContactPhase::Cancel,
        MotionPhase::PointerDown { .. }
        | MotionPhase::PointerUp { .. }
        | MotionPhase::PointerCancel { .. } => ContactPhase::Move,
        MotionPhase::HoverEnter | MotionPhase::HoverMove => ContactPhase::Hover,
        MotionPhase::HoverExit => ContactPhase::Leave,
        MotionPhase::ButtonPress | MotionPhase::ButtonRelease => return None,
    })
}

/// The side buttons of a batch the pen (tip or eraser end) is in: `BUTTON_STYLUS_PRIMARY` and
/// `BUTTON_STYLUS_SECONDARY`, or the mouse-like `BUTTON_SECONDARY` and `BUTTON_TERTIARY` that
/// older Android versions and some pens report for the same buttons. `None` without the pen.
fn pen_buttons(batch: &MotionBatch) -> Option<PenButtons> {
    let pen = |pointer: &Pointer| matches!(pointer.tool, ToolKind::Stylus | ToolKind::Eraser);
    batch.pointers.iter().any(pen).then_some(PenButtons {
        primary: batch.buttons.stylus_primary || batch.buttons.secondary,
        secondary: batch.buttons.stylus_secondary || batch.buttons.tertiary,
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
        let palm = normaliser.normalise(&two(MotionPhase::PointerCancel { index: 1 }));
        let cancel = normaliser.normalise(&two(MotionPhase::Cancel));
        use ContactPhase::{Cancel, Down, Move, Up};
        assert_eq!(phases(&down.expect("down")), [(4, Move), (9, Down)]);
        assert_eq!(phases(&up.expect("up")), [(4, Up), (9, Move)]);
        assert_eq!(phases(&palm.expect("palm")), [(4, Move), (9, Cancel)]);
        assert_eq!(phases(&cancel.expect("cancel")), [(4, Cancel), (9, Cancel)]);
    }

    #[test]
    fn the_pens_batches_carry_its_side_buttons_and_touches_none() {
        let mut normaliser = Normaliser::default();
        let mut hover = batch(
            MotionPhase::HoverMove,
            vec![pointer(0, ToolKind::Stylus, vec![sample(1.0, 0.0, 0)])],
        );
        hover.buttons.stylus_primary = true;
        let held = normaliser.normalise(&hover).expect("hover").pen;
        assert_eq!(
            held,
            Some(PenButtons {
                primary: true,
                secondary: false
            })
        );
        // The mouse-like bits some Android versions give a pen's buttons.
        let mut legacy = batch(
            MotionPhase::Down,
            vec![pointer(0, ToolKind::Eraser, vec![sample(1.0, 0.5, 4)])],
        );
        legacy.buttons.secondary = true;
        legacy.buttons.tertiary = true;
        let both = normaliser.normalise(&legacy).expect("down");
        assert_eq!(both.contacts[0].device, Device::Eraser);
        assert_eq!(
            both.pen,
            Some(PenButtons {
                primary: true,
                secondary: true
            })
        );
        let mut finger = batch(
            MotionPhase::Down,
            vec![pointer(3, ToolKind::Finger, vec![sample(1.0, 1.0, 5)])],
        );
        finger.buttons.stylus_primary = true;
        assert_eq!(normaliser.normalise(&finger).expect("touch").pen, None);
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
    fn unusable_samples_are_dropped_and_a_button_press_carries_only_the_buttons() {
        let mut normaliser = Normaliser::default();
        let lost = vec![sample(f32::NAN, 1.0, 0), sample(f32::INFINITY, 1.0, 1)];
        let moved = batch(MotionPhase::Move, vec![pointer(0, ToolKind::Stylus, lost)]);
        assert_eq!(normaliser.normalise(&moved), None);
        let mut press = batch(
            MotionPhase::ButtonPress,
            vec![pointer(0, ToolKind::Stylus, vec![sample(1.0, 0.0, 2)])],
        );
        press.buttons.stylus_secondary = true;
        let buttons = PenButtons {
            primary: false,
            secondary: true,
        };
        let pressed = InputBatch {
            contacts: Vec::new(),
            pen: Some(buttons),
        };
        assert_eq!(normaliser.normalise(&press), Some(pressed));
        let mouse = batch(
            MotionPhase::ButtonPress,
            vec![pointer(0, ToolKind::Mouse, vec![sample(1.0, 0.0, 2)])],
        );
        assert_eq!(normaliser.normalise(&mouse), None, "not the pen's");
        let mixed = vec![sample(f32::NAN, 1.0, 3), sample(4.0, 1.0, 4)];
        let moved = batch(MotionPhase::Move, vec![pointer(0, ToolKind::Stylus, mixed)]);
        let batch = normaliser.normalise(&moved).expect("batch");
        assert_eq!(batch.contacts[0].samples.len(), 1);
    }
}
