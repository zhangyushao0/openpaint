//! Contacts on the Transform tool's frame (draweave's handles): a contact touching down on the
//! turn handle turns the content about the frame's centre, by the short angle from where it
//! started; on a corner it scales the content evenly about the opposite corner; the pen inside
//! the frame moves it. The content follows the contact 1:1: each move is the placement at the
//! drag's start then the change since, never a sum of small steps. The lift settles it as one
//! step; a cancel puts it back. Fingers inside the frame move, pinch and twist it through the
//! canvas's gesture trackers instead (`gesture_placement`).

use std::f32::consts::{PI, TAU};

use openpaint_app::{
    Contact, ContactPhase, Device, Intent, TransformFrame, TransformIntent, ViewGesture,
    ViewMapping,
};
use openpaint_core::Placement;

use crate::transform_bar::{HANDLE_REACH, Handles};

/// The most one frame of a two-finger gesture scales the content (draweave's).
const GESTURE_ZOOM: (f32, f32) = (0.25, 4.0);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Grip {
    Move { from: [f32; 2] },
    Scale { pivot: [f32; 2], distance: f32 },
    Turn { pivot: [f32; 2], angle: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Held {
    id: i32,
    grip: Grip,
    start: Placement,
}

#[derive(Debug, Default)]
pub(crate) struct TransformDrag {
    held: Option<Held>,
    /// The fingers down now first touched inside the frame: their gestures move the content.
    pub(crate) fingers: bool,
}

impl TransformDrag {
    /// The intents of `contact` when it is the frame's: one touching down on open screen on a
    /// handle (pen or finger) or inside the frame (pen), and every later news of it. `None`
    /// when the contact is not the frame's.
    pub(crate) fn route(
        &mut self,
        contact: &Contact,
        frame: &TransformFrame,
        mapping: &ViewMapping,
        (pixels_per_point, open): (f32, bool),
    ) -> Option<Vec<Intent>> {
        let at = contact.samples.last()?.position;
        if contact.phase == ContactPhase::Down && self.held.is_none() && open {
            let pen = matches!(contact.device, Device::Pen | Device::Eraser);
            let grip = grip(frame, mapping, pixels_per_point, at, pen)?;
            self.held = Some(Held {
                id: contact.id,
                grip,
                start: frame.placement,
            });
            return Some(Vec::new());
        }
        let held = self.held.filter(|held| held.id == contact.id)?;
        let place = |placement| Intent::Transform(TransformIntent::Place(placement));
        let settle = Intent::Transform(TransformIntent::Settle);
        let intents = match contact.phase {
            ContactPhase::Cancel => {
                self.held = None;
                vec![place(held.start), settle]
            }
            phase => {
                let mut intents: Vec<Intent> = held
                    .placement(mapping.to_canvas(at))
                    .map(place)
                    .into_iter()
                    .collect();
                if phase == ContactPhase::Up {
                    self.held = None;
                    intents.push(settle);
                }
                intents
            }
        };
        Some(intents)
    }
}

/// What a contact touching down at the screen point `at` takes hold of: the turn handle, a
/// corner, or (the pen) the frame's inside.
fn grip(
    frame: &TransformFrame,
    mapping: &ViewMapping,
    pixels_per_point: f32,
    at: [f32; 2],
    pen: bool,
) -> Option<Grip> {
    let handles = Handles::of(frame, mapping, pixels_per_point);
    let reach = HANDLE_REACH * pixels_per_point;
    let distance = |p: [f32; 2]| (p[0] - at[0]).hypot(p[1] - at[1]);
    let from = mapping.to_canvas(at);
    let corners = frame.corners();
    if distance(handles.turn) <= reach {
        let pivot = frame.centre();
        return Some(Grip::Turn {
            pivot,
            angle: angle(pivot, from),
        });
    }
    let nearest = (0..4)
        .min_by(|a, b| distance(handles.corners[*a]).total_cmp(&distance(handles.corners[*b])))?;
    if distance(handles.corners[nearest]) <= reach {
        let pivot = corners[(nearest + 2) % 4];
        let distance = (from[0] - pivot[0]).hypot(from[1] - pivot[1]).max(0.001);
        return Some(Grip::Scale { pivot, distance });
    }
    (pen && handles.inside(at)).then_some(Grip::Move { from })
}

impl Held {
    /// The placement with the contact at canvas point `at`; `None` past what a placement holds.
    fn placement(&self, at: [f32; 2]) -> Option<Placement> {
        let change = match self.grip {
            Grip::Move { from } => Placement::moved([at[0] - from[0], at[1] - from[1]]),
            Grip::Scale { pivot, distance } => {
                let scale = (at[0] - pivot[0]).hypot(at[1] - pivot[1]) / distance;
                Placement::around(pivot, scale.clamp(0.01, 100.0), 0.0, [false; 2])
            }
            Grip::Turn { pivot, angle: from } => {
                let turn = short_angle(angle(pivot, at) - from);
                Placement::around(pivot, 1.0, turn, [false; 2])
            }
        }?;
        self.start.then(change)
    }
}

fn angle(pivot: [f32; 2], at: [f32; 2]) -> f32 {
    (at[1] - pivot[1]).atan2(at[0] - pivot[0])
}

/// `angle` in `-pi..pi`: a turn across the wrap of `atan2` is the short way round.
pub(crate) fn short_angle(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

/// `current` after one frame of a finger gesture over the frame, through `mapping`: the
/// content point under the gesture's pivot stays under it while the content scales and turns,
/// then moves with the fingers.
pub(crate) fn gesture_placement(
    current: Placement,
    gesture: &ViewGesture,
    mapping: &ViewMapping,
) -> Option<Placement> {
    let pivot = mapping.to_canvas(gesture.pivot);
    let [px, py] = gesture.pivot;
    let [tx, ty] = gesture.translation;
    let moved = mapping.to_canvas([px + tx, py + ty]);
    let zoom = gesture.zoom.clamp(GESTURE_ZOOM.0, GESTURE_ZOOM.1);
    let turn = Placement::around(pivot, zoom, gesture.rotation, [false; 2])?;
    let shift = Placement::moved([moved[0] - pivot[0], moved[1] - pivot[1]])?;
    current.then(turn)?.then(shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_handle_uses_the_short_angle_across_the_wrap_boundary() {
        let from = 179.0_f32.to_radians();
        let to = (-179.0_f32).to_radians();
        assert!((short_angle(to - from).to_degrees() - 2.0).abs() < 0.001);
        assert!((short_angle(from - to).to_degrees() + 2.0).abs() < 0.001);
    }

    #[test]
    fn a_held_handle_places_from_the_drags_start() {
        let start = Placement::moved([10.0, 0.0]).expect("move");
        let mover = Held {
            id: 1,
            grip: Grip::Move { from: [5.0, 5.0] },
            start,
        };
        let placed = mover.placement([8.0, 1.0]).expect("placed");
        assert_eq!(placed.apply([0.0, 0.0]), [13.0, -4.0]);
        let scaler = Held {
            grip: Grip::Scale {
                pivot: [0.0, 0.0],
                distance: 10.0,
            },
            ..mover
        };
        let doubled = scaler.placement([20.0, 0.0]).expect("scaled");
        assert_eq!(
            doubled.apply([1.0, 0.0]),
            [22.0, 0.0],
            "about the pivot, after start"
        );
        let turner = Held {
            grip: Grip::Turn {
                pivot: [0.0, 0.0],
                angle: 0.0,
            },
            start: Placement::IDENTITY,
            ..mover
        };
        let quarter = turner.placement([0.0, 3.0]).expect("turned");
        let [x, y] = quarter.apply([2.0, 0.0]);
        assert!(
            x.abs() < 1e-5 && (y - 2.0).abs() < 1e-5,
            "clockwise on the canvas"
        );
    }
}
