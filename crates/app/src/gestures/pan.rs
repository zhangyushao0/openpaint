//! One-finger pan: a single finger that touched down on open canvas moves the view once it has
//! gone 8 points (the dead zone is taken off, so the canvas does not jump). A second finger
//! ends it until every finger has lifted. Touch never paints (D18), so a finger on the canvas
//! is free to move it.

use super::{Touch, vector_after_dead_zone};
use crate::input::ContactPhase;
use crate::view::ViewGesture;

/// Points a finger must travel before it pans.
const THRESHOLD: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
struct Candidate {
    id: i32,
    start: [f32; 2],
    current: [f32; 2],
}

#[derive(Debug, Default)]
pub(super) struct OneFingerPan {
    /// Every finger down, whatever it does.
    down: Vec<i32>,
    candidate: Option<Candidate>,
    engaged: bool,
    blocked: bool,
}

impl OneFingerPan {
    pub(super) fn active(&self) -> bool {
        !self.down.is_empty()
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// The pan of this frame's `touches`, if the lone finger moves the view.
    pub(super) fn update(
        &mut self,
        touches: &[Touch],
        pixels_per_point: f32,
    ) -> Option<ViewGesture> {
        let threshold = THRESHOLD * pixels_per_point;
        let mut translation = [0.0f32; 2];
        let mut center = None;
        for touch in touches {
            match touch.phase {
                ContactPhase::Down => {
                    if !self.down.contains(&touch.id) {
                        self.down.push(touch.id);
                    }
                    if self.down.len() == 1 && !self.blocked && touch.open {
                        self.candidate = Some(Candidate {
                            id: touch.id,
                            start: touch.position,
                            current: touch.position,
                        });
                    } else {
                        self.block();
                        (translation, center) = ([0.0; 2], None);
                    }
                }
                ContactPhase::Move | ContactPhase::Up => {
                    if self.down.len() == 1
                        && !self.blocked
                        && let Some(step) = self.advance(touch.id, touch.position, threshold)
                    {
                        translation = [translation[0] + step[0], translation[1] + step[1]];
                        center = Some(touch.position);
                    }
                    if touch.phase == ContactPhase::Up {
                        self.down.retain(|id| *id != touch.id);
                        if self.down.is_empty() {
                            self.reset();
                        } else {
                            self.block();
                        }
                    }
                }
                ContactPhase::Cancel => {
                    self.down.retain(|id| *id != touch.id);
                    self.block();
                    self.blocked = !self.down.is_empty();
                }
                ContactPhase::Hover | ContactPhase::Leave => {}
            }
        }
        center
            .filter(|_| translation != [0.0; 2])
            .map(|pivot| ViewGesture {
                pivot,
                translation,
                zoom: 1.0,
                rotation: 0.0,
            })
    }

    fn block(&mut self) {
        self.candidate = None;
        self.engaged = false;
        self.blocked = true;
    }

    /// Moves the candidate `id` to `at`; its motion once past the dead zone.
    fn advance(&mut self, id: i32, at: [f32; 2], threshold: f32) -> Option<[f32; 2]> {
        let candidate = self.candidate.as_mut().filter(|c| c.id == id)?;
        let previous = candidate.current;
        candidate.current = at;
        let total = [at[0] - candidate.start[0], at[1] - candidate.start[1]];
        if self.engaged {
            Some([at[0] - previous[0], at[1] - previous[1]])
        } else if total[0].hypot(total[1]) >= threshold {
            self.engaged = true;
            Some(vector_after_dead_zone(total, threshold))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(id: i32, phase: ContactPhase, x: f32) -> Touch {
        Touch {
            id,
            phase,
            position: [x, 40.0],
            time_ms: 0,
            open: true,
        }
    }

    #[test]
    fn a_finger_pans_past_its_dead_zone() {
        let mut pan = OneFingerPan::default();
        assert!(
            pan.update(&[touch(1, ContactPhase::Down, 40.0)], 1.0)
                .is_none()
        );
        assert!(
            pan.update(&[touch(1, ContactPhase::Move, 46.0)], 1.0)
                .is_none()
        );
        let moved = pan
            .update(&[touch(1, ContactPhase::Move, 52.0)], 1.0)
            .expect("pans");
        assert_eq!(moved.translation, [4.0, 0.0]);
        let next = pan
            .update(&[touch(1, ContactPhase::Up, 60.0)], 1.0)
            .expect("the lift");
        assert_eq!(next.translation, [8.0, 0.0]);
        assert!(!pan.active());
    }

    #[test]
    fn a_second_finger_ends_the_pan_until_every_finger_lifts() {
        let mut pan = OneFingerPan::default();
        let starts = [
            touch(1, ContactPhase::Down, 40.0),
            touch(2, ContactPhase::Down, 100.0),
        ];
        assert!(pan.update(&starts, 1.0).is_none());
        let first_stays = [
            touch(2, ContactPhase::Up, 100.0),
            touch(1, ContactPhase::Move, 80.0),
        ];
        assert!(pan.update(&first_stays, 1.0).is_none());
        assert!(
            pan.update(&[touch(1, ContactPhase::Up, 80.0)], 1.0)
                .is_none()
        );
        assert!(
            pan.update(&[touch(3, ContactPhase::Down, 40.0)], 1.0)
                .is_none()
        );
        assert!(
            pan.update(&[touch(3, ContactPhase::Move, 60.0)], 1.0)
                .is_some()
        );
    }

    #[test]
    fn a_finger_from_a_control_never_pans() {
        let mut pan = OneFingerPan::default();
        let mut down = touch(1, ContactPhase::Down, 40.0);
        down.open = false;
        assert!(pan.update(&[down], 1.0).is_none());
        assert!(
            pan.update(&[touch(1, ContactPhase::Move, 90.0)], 1.0)
                .is_none()
        );
    }
}
