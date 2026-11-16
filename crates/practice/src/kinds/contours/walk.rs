//! A sausage's spine as a turtle walks it on the ground (S156k): straight on, round an arc, straight
//! on, its points then placed in front of the eye, which looks down at them a little.
//!
//! An arc is walked in points `ARC_STEP` apart at most: a centripetal Catmull-Rom spline
//! (`space_path`) gathers each point's turn at the point, so a U-turn through three points would
//! bend tighter than the tube is thick; points along the arc keep its radius.

use crate::kinds::contours::task::Bend;
use crate::maths::random::Random;
use crate::maths::vector::Vector;

/// An arc's turn between two points at most, degrees.
const ARC_STEP: f64 = 20.0;
/// The tube's distance from the eye; how far down the eye looks at it (degrees), at 稳 lower, so
/// its near part passes in front of its far one.
const DISTANCE: [f64; 2] = [3.2, 4.2];
const LOOK: [f64; 2] = [16.0, 26.0];
const LOW_LOOK: [f64; 2] = [12.0, 18.0];
/// How far either end of the spine rises above the tube lying on the ground, radii.
const LIFT: f64 = 0.6;

/// A step of the walk: straight on so far, or round an arc turning so many degrees (to the left
/// as the ground is seen from above when positive) of so great a radius; radii of the tube.
#[derive(Debug, Clone, Copy)]
enum Step {
    Straight(f64),
    Arc(f64, f64),
}

/// The spine's points for `bend` of a tube `radius` thick, laid from `random`: the near end first
/// where it goes back (入门).
pub(crate) fn walked(bend: Bend, radius: f64, random: &mut Random) -> Vec<Vector> {
    let (start, steps) = stepped(bend, random);
    let mut heading = start.to_radians();
    let mut at = [0.0, 0.0];
    let mut ground = vec![at];
    let mut go = |at: &mut [f64; 2], heading: f64, long: f64| {
        *at = [at[0] + long * heading.cos(), at[1] + long * heading.sin()];
        ground.push(*at);
    };
    for step in steps {
        match step {
            Step::Straight(long) => go(&mut at, heading, long * radius),
            Step::Arc(turn, round) => {
                let count = (turn.abs() / ARC_STEP).ceil().max(1.0);
                let each = (turn / count).to_radians();
                let chord = 2.0 * round * radius * (each / 2.0).sin().abs();
                for _ in 0..count as usize {
                    go(&mut at, heading + each / 2.0, chord);
                    heading += each;
                }
            }
        }
    }
    placed(&ground, (bend, radius), random)
}

/// The heading to start with (degrees, from the eye's right towards away from it) and the steps.
fn stepped(bend: Bend, random: &mut Random) -> (f64, Vec<Step>) {
    let mut range = |[low, high]: [f64; 2]| random.range(low, high);
    let right = range([0.0, 1.0]) < 0.5;
    let either = |degrees: f64| if right { degrees } else { 180.0 - degrees };
    let way = if right { -1.0 } else { 1.0 };
    match bend {
        // Going back across, one gentle bend either way.
        Bend::Gentle => {
            let turn = if range([0.0, 1.0]) < 0.5 { -1.0 } else { 1.0 } * range([25.0, 45.0]);
            let steps = vec![
                Step::Straight(range([4.0, 5.0])),
                Step::Arc(turn, range([6.0, 9.0])),
                Step::Straight(range([4.0, 5.0])),
            ];
            (either(range([35.0, 60.0])), steps)
        }
        // Across and a little back, then turning towards the eye.
        Bend::Strong => {
            let steps = vec![
                Step::Straight(range([4.0, 5.0])),
                Step::Arc(way * range([70.0, 90.0]), range([3.0, 3.6])),
                Step::Straight(range([4.0, 5.0])),
            ];
            (either(range([15.0, 30.0])), steps)
        }
        // Across, round towards the eye and back in front of itself, then the other way.
        Bend::Crossing => {
            let tilt = if range([0.0, 1.0]) < 0.5 { -1.0 } else { 1.0 } * range([12.0, 25.0]);
            let steps = vec![
                Step::Straight(range([3.0, 4.0])),
                Step::Arc(way * range([180.0, 200.0]), range([3.0, 3.4])),
                Step::Straight(range([2.0, 3.0])),
                Step::Arc(-way * range([50.0, 80.0]), range([3.0, 3.6])),
                Step::Straight(range([1.0, 2.0])),
            ];
            (either(tilt), steps)
        }
    }
}

/// The walk on the ground `ground` put in front of the eye: its middle `DISTANCE` away and aside a
/// little, the tube lying on the ground the eye looks down at, its ends risen a little (not at 稳).
fn placed(ground: &[[f64; 2]], (bend, radius): (Bend, f64), random: &mut Random) -> Vec<Vector> {
    let middle = [0, 1].map(|k| {
        let values = ground.iter().map(|p| p[k]);
        let low = values.clone().fold(f64::INFINITY, f64::min);
        (low + values.fold(f64::NEG_INFINITY, f64::max)) / 2.0
    });
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    let look = if bend == Bend::Crossing {
        LOW_LOOK
    } else {
        LOOK
    };
    let floor = -depth * random.range(look[0], look[1]).to_radians().tan();
    let aside = random.range(-0.15, 0.15) * depth;
    let lifts = if bend == Bend::Crossing {
        [0.0; 2]
    } else {
        [0; 2].map(|_| random.range(0.0, LIFT))
    };
    let last = (ground.len() - 1).max(1) as f64;
    (ground.iter().enumerate())
        .map(|(k, [x, z])| {
            let share = k as f64 / last;
            let lift = lifts[0] + (lifts[1] - lifts[0]) * share;
            [
                x - middle[0] + aside,
                floor + radius * (1.0 + lift),
                z - middle[1] + depth,
            ]
        })
        .collect()
}
