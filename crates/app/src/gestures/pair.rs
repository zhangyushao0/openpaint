//! The navigation pair: exactly two fingers that both touched down on open canvas. Its motion
//! in a frame (pan of the midpoint, zoom of the span, turn of the line between them) comes from
//! the raw contact positions, release positions included: Android batches the last move with
//! the lift. A third finger, a cancel, or a finger that touched down on a control or off the
//! canvas blocks navigation until every finger has lifted.

use std::f32::consts::{PI, TAU};

use super::Touch;
use crate::input::ContactPhase;

/// One finger down: where it is now.
#[derive(Debug, Clone, Copy)]
struct Finger {
    id: i32,
    at: [f32; 2],
}

/// Raw two-finger motion over one frame, in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PairDelta {
    /// The midpoint at the end of the frame.
    pub(super) center: [f32; 2],
    pub(super) translation: [f32; 2],
    pub(super) zoom: f32,
    pub(super) rotation: f32,
}

#[derive(Debug, Default)]
pub(super) struct PairTracker {
    fingers: Vec<Finger>,
    blocked: bool,
}

impl PairTracker {
    pub(super) fn active(&self) -> bool {
        self.pair().is_some()
    }

    fn pair(&self) -> Option<[[f32; 2]; 2]> {
        (!self.blocked && self.fingers.len() == 2).then(|| [self.fingers[0].at, self.fingers[1].at])
    }

    fn pair_ids(&self) -> Option<[i32; 2]> {
        self.pair()
            .map(|_| [self.fingers[0].id, self.fingers[1].id])
    }

    /// The pair's motion over this frame's `touches`; `None` without a pair or when it broke.
    pub(super) fn update(&mut self, touches: &[Touch]) -> Option<PairDelta> {
        let mut before = self.pair();
        let mut after = before;
        let mut ids = self.pair_ids();
        let (mut ended, mut cancelled) = (false, false);
        for touch in touches {
            let index = self.fingers.iter().position(|f| f.id == touch.id);
            match touch.phase {
                ContactPhase::Down => {
                    if index.is_some() {
                        continue;
                    }
                    self.fingers.push(Finger {
                        id: touch.id,
                        at: touch.position,
                    });
                    if ended || self.fingers.len() > 2 || !touch.open {
                        self.blocked = true;
                        cancelled = true;
                    }
                    if before.is_none() && !cancelled {
                        before = self.pair();
                        after = before;
                        ids = self.pair_ids();
                    }
                }
                ContactPhase::Move | ContactPhase::Up => {
                    let Some(index) = index else {
                        continue;
                    };
                    self.fingers[index].at = touch.position;
                    if let (Some(ids), Some(after)) = (ids, after.as_mut())
                        && let Some(slot) = ids.iter().position(|id| *id == touch.id)
                    {
                        after[slot] = touch.position;
                    }
                    if touch.phase == ContactPhase::Up {
                        ended |= self.fingers.len() == 2;
                        self.fingers.remove(index);
                        self.blocked = !self.fingers.is_empty();
                    }
                }
                ContactPhase::Cancel => {
                    if let Some(index) = index {
                        self.fingers.remove(index);
                    }
                    cancelled = true;
                    self.blocked = !self.fingers.is_empty();
                }
                ContactPhase::Hover | ContactPhase::Leave => {}
            }
        }
        let result = if cancelled {
            None
        } else {
            before.zip(after).and_then(|(from, to)| delta(from, to))
        };
        if self.fingers.is_empty() {
            self.blocked = false;
        }
        result
    }
}

/// The motion from the pair at `from` to the pair at `to`.
fn delta(from: [[f32; 2]; 2], to: [[f32; 2]; 2]) -> Option<PairDelta> {
    let span = |[a, b]: [[f32; 2]; 2]| [b[0] - a[0], b[1] - a[1]];
    let mid = |[a, b]: [[f32; 2]; 2]| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let (before, after) = (span(from), span(to));
    let (length_before, length_after) = (before[0].hypot(before[1]), after[0].hypot(after[1]));
    if length_before < 1.0 || length_after < 1.0 {
        return None;
    }
    let angle = |[x, y]: [f32; 2]| y.atan2(x);
    let rotation = (angle(after) - angle(before) + PI).rem_euclid(TAU) - PI;
    let (start, center) = (mid(from), mid(to));
    Some(PairDelta {
        center,
        translation: [center[0] - start[0], center[1] - start[1]],
        zoom: length_after / length_before,
        rotation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(id: i32, phase: ContactPhase, x: f32, y: f32) -> Touch {
        Touch {
            id,
            phase,
            position: [x, y],
            time_ms: 0,
            open: true,
        }
    }

    fn pair(phase: ContactPhase, offset: f32) -> Vec<Touch> {
        vec![
            touch(1, phase, 100.0 + offset, 100.0),
            touch(2, phase, 200.0 + offset, 100.0),
        ]
    }

    #[test]
    fn a_pair_reports_pan_zoom_and_turn_including_the_release_positions() {
        let mut tracker = PairTracker::default();
        assert_eq!(
            tracker
                .update(&pair(ContactPhase::Down, 0.0))
                .map(|d| d.zoom),
            Some(1.0)
        );
        let moved = tracker
            .update(&pair(ContactPhase::Move, 20.0))
            .expect("pair");
        assert_eq!(moved.translation, [20.0, 0.0]);
        // The last move arrives with the lifts: it still counts.
        let lifted = [
            touch(1, ContactPhase::Up, 120.0, 100.0),
            touch(2, ContactPhase::Up, 120.0, 200.0),
        ];
        let turned = tracker.update(&lifted).expect("the release moves");
        assert!((turned.rotation - PI / 2.0).abs() < 1e-5);
        assert!(!tracker.active());
    }

    #[test]
    fn a_cancel_or_a_third_finger_blocks_the_pair_until_all_lift() {
        for cancel in [false, true] {
            let mut tracker = PairTracker::default();
            tracker.update(&pair(ContactPhase::Down, 0.0));
            assert!(tracker.update(&pair(ContactPhase::Move, 20.0)).is_some());
            let mut interrupted = pair(ContactPhase::Move, 40.0);
            if cancel {
                interrupted.extend(pair(ContactPhase::Cancel, 40.0));
            } else {
                interrupted.push(touch(3, ContactPhase::Down, 300.0, 100.0));
            }
            assert!(tracker.update(&interrupted).is_none());
            assert!(!tracker.active());
            assert!(tracker.update(&pair(ContactPhase::Move, 60.0)).is_none());
            let mut lift = pair(ContactPhase::Up, 60.0);
            lift.push(touch(3, ContactPhase::Up, 300.0, 100.0));
            assert!(tracker.update(&lift).is_none());
            tracker.update(&pair(ContactPhase::Down, 0.0));
            assert!(tracker.update(&pair(ContactPhase::Move, 20.0)).is_some());
        }
    }

    #[test]
    fn a_finger_that_touched_down_on_a_control_blocks_navigation() {
        let mut tracker = PairTracker::default();
        let mut on_control = touch(1, ContactPhase::Down, 100.0, 100.0);
        on_control.open = false;
        let start = [on_control, touch(2, ContactPhase::Down, 200.0, 100.0)];
        assert!(tracker.update(&start).is_none());
        let moves = [
            touch(1, ContactPhase::Move, 200.0, 200.0),
            touch(2, ContactPhase::Move, 400.0, 200.0),
        ];
        assert!(tracker.update(&moves).is_none());
    }
}
