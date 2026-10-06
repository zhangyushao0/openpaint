//! The three-finger swipe down (Procreate's gesture for its Copy & Paste menu, S40): exactly
//! three fingers on open canvas whose middle moves down at least 48 points, within 30 degrees
//! of straight down, every finger at least half that far down, by 0.8 s after the third one
//! landed. It fires once, as it passes, on a move or the lift that carries the last move. The
//! moved fingers void the three-finger tap (redo, `taps.rs`, which allows 8 points of drift),
//! and three fingers never navigate (a third finger blocks the pair, `pair.rs`). A fourth
//! finger, a finger from a control and a cancel void it until every finger has lifted.

use super::{Touch, TouchAction};
use crate::input::ContactPhase;

/// Points the fingers' middle travels down.
const DISTANCE: f32 = 48.0;
/// Largest sideways share of the way down: tan 30 degrees.
const SLANT: f32 = 0.58;
/// Longest time from the third finger landing to the swipe passing its distance.
const WITHIN_MS: i64 = 800;

#[derive(Debug, Clone, Copy)]
struct Finger {
    id: i32,
    start: [f32; 2],
    current: [f32; 2],
}

#[derive(Debug, Default)]
pub(super) struct Swipe {
    down: Vec<Finger>,
    /// When the third finger landed.
    third_ms: Option<i64>,
    /// Fired, or no swipe any more, until every finger has lifted.
    done: bool,
}

impl Swipe {
    /// The menu, when this frame's `touches` complete the swipe.
    pub(super) fn update(
        &mut self,
        touches: &[Touch],
        pixels_per_point: f32,
    ) -> Option<TouchAction> {
        let mut lifted = Vec::new();
        let mut now_ms = None;
        for touch in touches {
            match touch.phase {
                ContactPhase::Down => {
                    if self.down.is_empty() {
                        *self = Self::default();
                    }
                    self.down.push(Finger {
                        id: touch.id,
                        start: touch.position,
                        current: touch.position,
                    });
                    self.done |= !touch.open || self.down.len() > 3;
                    if self.down.len() == 3 {
                        self.third_ms = Some(touch.time_ms);
                    }
                }
                ContactPhase::Move | ContactPhase::Up => {
                    let Some(finger) = self.down.iter_mut().find(|f| f.id == touch.id) else {
                        continue;
                    };
                    finger.current = touch.position;
                    now_ms = Some(touch.time_ms);
                    if touch.phase == ContactPhase::Up {
                        lifted.push(touch.id);
                    }
                }
                ContactPhase::Cancel => {
                    self.down.retain(|f| f.id != touch.id);
                    self.done = true;
                }
                ContactPhase::Hover | ContactPhase::Leave => {}
            }
        }
        // Every finger's move of the frame first: Android batches the last moves with lifts.
        let fired = now_ms.is_some_and(|now| self.passed(now, pixels_per_point));
        self.done |= fired || !lifted.is_empty();
        self.down.retain(|f| !lifted.contains(&f.id));
        fired.then_some(TouchAction::CopyPaste)
    }

    /// Whether the three fingers down have swiped far enough, straight enough, soon enough.
    fn passed(&self, now_ms: i64, pixels_per_point: f32) -> bool {
        let soon = self
            .third_ms
            .is_some_and(|third| now_ms - third <= WITHIN_MS);
        if self.done || self.down.len() != 3 || !soon {
            return false;
        }
        let distance = DISTANCE * pixels_per_point;
        let moved = |f: &Finger| [f.current[0] - f.start[0], f.current[1] - f.start[1]];
        let [dx, dy] = self
            .down
            .iter()
            .map(moved)
            .fold([0.0, 0.0], |[x, y], [a, b]| [x + a / 3.0, y + b / 3.0]);
        let together = self.down.iter().all(|f| moved(f)[1] >= distance / 2.0);
        dy >= distance && dx.abs() <= dy * SLANT && together
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(id: i32, phase: ContactPhase, at: [f32; 2], time_ms: i64) -> Touch {
        Touch {
            id,
            phase,
            position: at,
            time_ms,
            open: true,
        }
    }

    /// Three fingers 100 px apart at `y`, moved by `by`.
    fn three(phase: ContactPhase, by: [f32; 2], time_ms: i64) -> Vec<Touch> {
        (1..=3)
            .map(|id| {
                let at = [id as f32 * 100.0 + by[0], 300.0 + by[1]];
                touch(id, phase, at, time_ms)
            })
            .collect()
    }

    #[test]
    fn three_fingers_down_far_enough_open_the_menu_once() {
        let mut swipe = Swipe::default();
        assert_eq!(
            swipe.update(&three(ContactPhase::Down, [0.0; 2], 0), 2.0),
            None
        );
        let short = three(ContactPhase::Move, [10.0, 90.0], 100);
        assert_eq!(
            swipe.update(&short, 2.0),
            None,
            "under 96 px at 2 px a point"
        );
        let far = three(ContactPhase::Move, [20.0, 100.0], 160);
        assert_eq!(swipe.update(&far, 2.0), Some(TouchAction::CopyPaste));
        let further = three(ContactPhase::Move, [20.0, 200.0], 200);
        assert_eq!(swipe.update(&further, 2.0), None, "once");
        assert_eq!(
            swipe.update(&three(ContactPhase::Up, [20.0, 200.0], 220), 2.0),
            None
        );
    }

    #[test]
    fn the_lift_carrying_the_last_move_completes_it() {
        let mut swipe = Swipe::default();
        swipe.update(&three(ContactPhase::Down, [0.0; 2], 0), 1.0);
        let up = three(ContactPhase::Up, [0.0, 60.0], 120);
        assert_eq!(swipe.update(&up, 1.0), Some(TouchAction::CopyPaste));
    }

    #[test]
    fn slanted_slow_upward_two_or_four_fingers_and_spreads_do_not() {
        let cases: [(&[i32], [f32; 2], i64); 5] = [
            (&[1, 2, 3], [60.0, 80.0], 100),   // more than 30 degrees off
            (&[1, 2, 3], [0.0, 80.0], 900),    // too late
            (&[1, 2, 3], [0.0, -80.0], 100),   // up
            (&[1, 2], [0.0, 80.0], 100),       // two fingers
            (&[1, 2, 3, 4], [0.0, 80.0], 100), // four
        ];
        for (ids, by, time_ms) in cases {
            let mut swipe = Swipe::default();
            let at = |id: i32, by: [f32; 2]| [id as f32 * 100.0 + by[0], 300.0 + by[1]];
            let down: Vec<_> = ids
                .iter()
                .map(|&id| touch(id, ContactPhase::Down, at(id, [0.0; 2]), 0))
                .collect();
            swipe.update(&down, 1.0);
            let moved: Vec<_> = ids
                .iter()
                .map(|&id| touch(id, ContactPhase::Move, at(id, by), time_ms))
                .collect();
            assert_eq!(
                swipe.update(&moved, 1.0),
                None,
                "{ids:?} by {by:?} at {time_ms}"
            );
        }
        // A pinch whose middle goes down while one finger rises.
        let mut swipe = Swipe::default();
        swipe.update(&three(ContactPhase::Down, [0.0; 2], 0), 1.0);
        let spread = [
            touch(1, ContactPhase::Move, [100.0, 280.0], 100),
            touch(2, ContactPhase::Move, [200.0, 400.0], 100),
            touch(3, ContactPhase::Move, [300.0, 400.0], 100),
        ];
        assert_eq!(swipe.update(&spread, 1.0), None);
    }
}
