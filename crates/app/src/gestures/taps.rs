//! Finger taps on open canvas: the most fingers down at once decides, when the last one lifts:
//! two undo, three redo. Any finger moving more than 8 points voids the tap; it has no longest
//! duration, but fingers held still repeat it, first after 0.45 s and then every 0.08 s, with
//! nothing more when they lift. A quick pinch (two fingers, the whole gesture within 0.4 s,
//! starting at least 48 points apart and ending at most 0.45 of that) fits the canvas; it is
//! checked before the taps. A finger that touched down on a control or off the canvas is not
//! part of any tap, and neither is its lift.

use super::{Touch, TouchAction, distance};
use crate::input::ContactPhase;

/// Points a finger may drift and still tap.
const TAP_SLOP: f32 = 8.0;
const HOLD_MS: i64 = 450;
const REPEAT_MS: i64 = 80;
const PINCH_MS: i64 = 400;
/// Points between two fingers a quick pinch starts at, at least.
const PINCH_SPAN: f32 = 48.0;
const PINCH_RATIO: f32 = 0.45;

#[derive(Debug, Clone, Copy)]
struct Finger {
    id: i32,
    start: [f32; 2],
    current: [f32; 2],
}

/// A held history shortcut: the action and when it fires next.
#[derive(Debug, Clone, Copy)]
struct Hold {
    action: TouchAction,
    next_ms: i64,
}

#[derive(Debug, Default)]
pub(super) struct Taps {
    down: Vec<Finger>,
    most: usize,
    moved: bool,
    started_ms: i64,
    first_span: Option<f32>,
    last_span: Option<f32>,
    hold: Option<Hold>,
    repeated: bool,
}

impl Taps {
    /// True while fingers rest still for a shortcut that repeats.
    pub(super) fn holding(&self) -> bool {
        self.hold.is_some()
    }

    /// The shortcut this frame's `touches` complete, or a held one that repeats at `now_ms`.
    pub(super) fn update(
        &mut self,
        touches: &[Touch],
        pixels_per_point: f32,
        now_ms: i64,
    ) -> Option<TouchAction> {
        let slop = TAP_SLOP * pixels_per_point;
        let mut completed = None;
        for touch in touches {
            match touch.phase {
                ContactPhase::Down if touch.open => self.down(touch),
                ContactPhase::Move | ContactPhase::Up => {
                    let Some(finger) = self.down.iter_mut().find(|f| f.id == touch.id) else {
                        // A contact a control owns: never a second end of the last tap.
                        continue;
                    };
                    finger.current = touch.position;
                    self.moved |= distance(finger.start, finger.current) > slop;
                    self.measure_span();
                    if touch.phase == ContactPhase::Up {
                        self.down.retain(|f| f.id != touch.id);
                        if self.down.is_empty() {
                            completed = self.complete(touch.time_ms, pixels_per_point);
                        }
                    }
                }
                ContactPhase::Cancel => {
                    self.down.clear();
                    self.moved = true;
                    self.hold = None;
                }
                ContactPhase::Down | ContactPhase::Hover | ContactPhase::Leave => {}
            }
        }
        completed.or_else(|| self.repeat(now_ms))
    }

    fn down(&mut self, touch: &Touch) {
        if self.down.is_empty() {
            *self = Self {
                started_ms: touch.time_ms,
                ..Self::default()
            };
        }
        self.down.push(Finger {
            id: touch.id,
            start: touch.position,
            current: touch.position,
        });
        self.most = self.most.max(self.down.len());
        self.measure_span();
    }

    /// The last finger lifted at `end_ms`.
    fn complete(&mut self, end_ms: i64, pixels_per_point: f32) -> Option<TouchAction> {
        self.hold = None;
        let pinched = self.most == 2
            && end_ms - self.started_ms <= PINCH_MS
            && self
                .first_span
                .zip(self.last_span)
                .is_some_and(|(first, last)| {
                    first >= PINCH_SPAN * pixels_per_point && last / first <= PINCH_RATIO
                });
        if pinched {
            Some(TouchAction::Fit)
        } else if !self.moved && !self.repeated {
            history(self.most)
        } else {
            None
        }
    }

    /// Fingers held still fire their shortcut again after `HOLD_MS`, then every `REPEAT_MS`.
    fn repeat(&mut self, now_ms: i64) -> Option<TouchAction> {
        let held = (!self.moved && !self.down.is_empty() && self.down.len() == self.most)
            .then(|| history(self.most))
            .flatten();
        let Some(action) = held else {
            self.hold = None;
            return None;
        };
        let hold = self.hold.get_or_insert(Hold {
            action,
            next_ms: now_ms + HOLD_MS,
        });
        if hold.action != action {
            *hold = Hold {
                action,
                next_ms: now_ms + HOLD_MS,
            };
        }
        if now_ms < hold.next_ms {
            return None;
        }
        hold.next_ms = now_ms + REPEAT_MS;
        self.repeated = true;
        Some(action)
    }

    fn measure_span(&mut self) {
        if let [a, b] = self.down[..] {
            let span = distance(a.current, b.current);
            self.first_span.get_or_insert(span);
            self.last_span = Some(span);
        }
    }
}

fn history(fingers: usize) -> Option<TouchAction> {
    match fingers {
        2 => Some(TouchAction::Undo),
        3 => Some(TouchAction::Redo),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(id: i32, phase: ContactPhase, x: f32, time_ms: i64) -> Touch {
        Touch {
            id,
            phase,
            position: [x, 20.0],
            time_ms,
            open: true,
        }
    }

    fn fingers(count: i32, phase: ContactPhase, time_ms: i64) -> Vec<Touch> {
        (1..=count)
            .map(|id| touch(id, phase, id as f32 * 100.0, time_ms))
            .collect()
    }

    #[test]
    fn two_and_three_finger_taps_undo_and_redo_when_the_last_finger_lifts() {
        let mut taps = Taps::default();
        assert_eq!(
            taps.update(&fingers(2, ContactPhase::Down, 0), 1.0, 0),
            None
        );
        let up = fingers(2, ContactPhase::Up, 90);
        assert_eq!(taps.update(&up, 1.0, 90), Some(TouchAction::Undo));
        let stray = [touch(9, ContactPhase::Up, 10.0, 100)];
        assert_eq!(taps.update(&stray, 1.0, 100), None, "a control's lift");
        assert_eq!(
            taps.update(&fingers(3, ContactPhase::Down, 200), 1.0, 200),
            None
        );
        let up = fingers(3, ContactPhase::Up, 260);
        assert_eq!(taps.update(&up, 1.0, 260), Some(TouchAction::Redo));
    }

    #[test]
    fn moved_fingers_and_fingers_from_a_control_do_not_tap() {
        let mut taps = Taps::default();
        taps.update(&fingers(2, ContactPhase::Down, 0), 2.0, 0);
        taps.update(&[touch(1, ContactPhase::Move, 117.0, 10)], 2.0, 10);
        assert_eq!(
            taps.update(&fingers(2, ContactPhase::Up, 20), 2.0, 20),
            None
        );
        let mut on_control = fingers(2, ContactPhase::Down, 100);
        on_control[0].open = false;
        taps.update(&on_control, 1.0, 100);
        assert_eq!(
            taps.update(&fingers(2, ContactPhase::Up, 150), 1.0, 150),
            None
        );
    }

    #[test]
    fn held_fingers_repeat_their_shortcut_with_nothing_more_on_the_lift() {
        let mut taps = Taps::default();
        taps.update(&fingers(2, ContactPhase::Down, 0), 1.0, 0);
        assert_eq!(taps.update(&[], 1.0, 449), None);
        assert!(taps.holding());
        assert_eq!(taps.update(&[], 1.0, 450), Some(TouchAction::Undo));
        assert_eq!(taps.update(&[], 1.0, 520), None);
        assert_eq!(taps.update(&[], 1.0, 530), Some(TouchAction::Undo));
        let up = fingers(2, ContactPhase::Up, 540);
        assert_eq!(taps.update(&up, 1.0, 540), None, "nothing more on the lift");
        assert!(!taps.holding());
        taps.update(&fingers(3, ContactPhase::Down, 1000), 1.0, 1000);
        assert_eq!(taps.update(&[], 1.0, 1450), Some(TouchAction::Redo));
    }

    #[test]
    fn a_quick_inward_pinch_fits_the_canvas() {
        let mut taps = Taps::default();
        let down = [
            touch(1, ContactPhase::Down, 20.0, 0),
            touch(2, ContactPhase::Down, 140.0, 0),
        ];
        taps.update(&down, 1.0, 0);
        let moves = [
            touch(1, ContactPhase::Move, 60.0, 100),
            touch(2, ContactPhase::Move, 100.0, 100),
        ];
        taps.update(&moves, 1.0, 100);
        let up = [
            touch(1, ContactPhase::Up, 60.0, 250),
            touch(2, ContactPhase::Up, 100.0, 250),
        ];
        assert_eq!(taps.update(&up, 1.0, 250), Some(TouchAction::Fit));
        // The same pinch, slower, is no shortcut.
        taps.update(&down, 1.0, 1000);
        taps.update(&moves, 1.0, 1300);
        let late = [
            touch(1, ContactPhase::Up, 60.0, 1500),
            touch(2, ContactPhase::Up, 100.0, 1500),
        ];
        assert_eq!(taps.update(&late, 1.0, 1500), None);
    }
}
