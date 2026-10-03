//! Gestures inside a floating picture (the reference window), with the canvas's dead zones: one
//! finger pans the picture once it has gone 8 points, two fingers pinch-zoom it about their
//! middle and pan it (their turn is left out: the picture never rotates). A single contact, the
//! pen's included, that lifts within 8 points of where it touched down with no other contact
//! down meanwhile is a tap. Every contact fed here touched down on the picture, so the canvas's
//! trackers never navigate with it.

use super::filter::NavigationFilter;
use super::pair::PairTracker;
use super::pan::OneFingerPan;
use super::{Touch, distance};
use crate::input::ContactPhase;
use crate::view::ViewGesture;

/// Points a contact may drift and still tap.
const TAP_SLOP: f32 = 8.0;

/// What the hand did on the picture this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PictureGestured {
    /// The picture's pan and zoom, in screen pixels about `pivot`; `rotation` is always 0.
    pub view: Option<ViewGesture>,
    /// A tap ended.
    pub tap: bool,
}

/// The one contact down that may still be a tap.
#[derive(Debug, Clone, Copy)]
struct Tap {
    id: i32,
    start: [f32; 2],
}

#[derive(Debug, Default)]
pub struct PictureGestures {
    pair: PairTracker,
    filter: NavigationFilter,
    pan: OneFingerPan,
    /// Every contact down on the picture, fingers and pen.
    down: Vec<i32>,
    tap: Option<Tap>,
}

impl PictureGestures {
    /// The meaning of this frame's `fingers` and `pens` on the picture (positions in screen
    /// pixels, release positions included). Only fingers pan and zoom.
    pub fn update(
        &mut self,
        fingers: &[Touch],
        pens: &[Touch],
        pixels_per_point: f32,
    ) -> PictureGestured {
        let mut tap = false;
        for touch in fingers.iter().chain(pens) {
            tap |= self.tap(touch, pixels_per_point);
        }
        let one = self.pan.update(fingers, pixels_per_point);
        let delta = self.pair.update(fingers);
        let two = self.filter.update(delta, pixels_per_point);
        if !self.pair.active() {
            self.filter.update(None, pixels_per_point);
        }
        let view = two
            .or(one)
            .map(|gesture| ViewGesture {
                rotation: 0.0,
                ..gesture
            })
            .filter(|g| g.translation != [0.0; 2] || (g.zoom - 1.0).abs() > f32::EPSILON);
        PictureGestured { view, tap }
    }

    /// Follows `touch` for a tap; true when it ends one.
    fn tap(&mut self, touch: &Touch, pixels_per_point: f32) -> bool {
        let ours = self.tap.filter(|tap| tap.id == touch.id);
        match touch.phase {
            ContactPhase::Down => {
                if !self.down.contains(&touch.id) {
                    self.down.push(touch.id);
                }
                self.tap = (self.down.len() == 1).then_some(Tap {
                    id: touch.id,
                    start: touch.position,
                });
                false
            }
            ContactPhase::Move | ContactPhase::Up => {
                let near = ours.is_some_and(|tap| {
                    distance(tap.start, touch.position) <= TAP_SLOP * pixels_per_point
                });
                if ours.is_some() && !near {
                    self.tap = None;
                }
                if touch.phase == ContactPhase::Move {
                    return false;
                }
                self.down.retain(|id| *id != touch.id);
                if ours.is_some() {
                    self.tap = None;
                }
                near
            }
            ContactPhase::Cancel => {
                self.down.retain(|id| *id != touch.id);
                self.tap = None;
                false
            }
            ContactPhase::Hover | ContactPhase::Leave => false,
        }
    }

    /// Every contact so far is forgotten (the picture went away or a control took over).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
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

    #[test]
    fn a_still_contact_taps_and_a_second_one_voids_it() {
        let mut gestures = PictureGestures::default();
        let pen = [touch(7, ContactPhase::Down, 50.0, 50.0)];
        assert!(!gestures.update(&[], &pen, 1.0).tap);
        let up = [touch(7, ContactPhase::Up, 56.0, 50.0)];
        assert!(gestures.update(&[], &up, 1.0).tap, "the pen taps too");
        let two = [
            touch(1, ContactPhase::Down, 50.0, 50.0),
            touch(2, ContactPhase::Down, 90.0, 50.0),
        ];
        gestures.update(&two, &[], 1.0);
        let lifts = [
            touch(2, ContactPhase::Up, 90.0, 50.0),
            touch(1, ContactPhase::Up, 50.0, 50.0),
        ];
        assert!(!gestures.update(&lifts, &[], 1.0).tap);
        gestures.update(&[touch(3, ContactPhase::Down, 50.0, 50.0)], &[], 1.0);
        let dragged = gestures.update(&[touch(3, ContactPhase::Up, 80.0, 50.0)], &[], 1.0);
        assert!(!dragged.tap, "a drag is no tap");
        assert_eq!(dragged.view.map(|v| v.translation), Some([22.0, 0.0]));
    }

    #[test]
    fn a_pinch_zooms_about_its_middle_past_the_dead_zone_and_never_turns() {
        let mut gestures = PictureGestures::default();
        let at = |phase, spread: f32, lift: f32| {
            [
                touch(1, phase, 100.0 - spread, 100.0 - lift),
                touch(2, phase, 200.0 + spread, 100.0 + lift),
            ]
        };
        gestures.update(&at(ContactPhase::Down, 0.0, 0.0), &[], 1.0);
        assert!(
            gestures
                .update(&at(ContactPhase::Move, 1.0, 0.0), &[], 1.0)
                .view
                .is_none()
        );
        let zoomed = gestures
            .update(&at(ContactPhase::Move, 20.0, 4.0), &[], 1.0)
            .view
            .expect("a deliberate pinch");
        assert!(zoomed.zoom > 1.2 && zoomed.rotation == 0.0, "{zoomed:?}");
        assert_eq!(zoomed.pivot, [150.0, 100.0]);
        let released = gestures.update(&at(ContactPhase::Up, 30.0, 4.0), &[], 1.0);
        assert!(
            released.view.is_some_and(|v| v.zoom > 1.0),
            "the lift's move counts"
        );
        assert!(!released.tap);
    }

    #[test]
    fn the_pen_never_pans_the_picture() {
        let mut gestures = PictureGestures::default();
        gestures.update(&[], &[touch(7, ContactPhase::Down, 50.0, 50.0)], 1.0);
        let moved = gestures.update(&[], &[touch(7, ContactPhase::Move, 150.0, 50.0)], 1.0);
        assert_eq!(moved, PictureGestured::default());
    }
}
