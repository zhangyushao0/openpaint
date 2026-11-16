//! A funnel (S156l, Drawabox's funnels; ellipses' 稳): three ellipses aligned on one axis that is
//! not the hand's habit, each wide enough to touch the funnel's two sides, as every cylinder,
//! contour and wheel needs. The axis runs at any angle; the two sides lie symmetric about it, each
//! 5 to 15 degrees off it, meeting beyond the ellipses. Each ellipse is given by its short axis,
//! printed whole on the funnel's and marked across at its ends: centred on it, its long axis
//! square to it and as long as touches both sides. Its degree changes along the funnel, as
//! Drawabox lets it, set by its short axis, so its exact ellipse is one (the design's "the degree
//! is the learner's" cannot hold where its exact answer is laid for the eye, D123). Frame units,
//! generated the same for the same seed.

use crate::geometry::ellipse::{Ellipse, upright};
use crate::maths::path::Point;
use crate::maths::random::Random;

/// Funnels laid before the generator keeps the last one whatever it is.
const TRIES: usize = 400;
/// Each side's angle off the axis (degrees).
const OFF_AXIS: [f64; 2] = [5.0, 15.0];
/// The first ellipse's half width along its long axis, and the widest's at most.
const NARROW: [f64; 2] = [0.08, 0.13];
const WIDEST: f64 = 0.3;
/// The ellipses' roundness (short axis over long) at the funnel's ends, and how far apart at
/// least: the middle one's halfway.
const ROUND: [f64; 2] = [0.25, 0.7];
const SPREAD: f64 = 0.15;
/// Neighbouring ellipses' centres at least this far apart along the axis, at least this much
/// clear of each other on it, and at most this much farther apart than that.
const APART: f64 = 0.12;
const CLEAR: f64 = 0.05;
const MORE: f64 = 0.05;
/// How far the axis and the sides run past the first and the last ellipse; how far inside the
/// frame everything keeps; how far the middle ellipse lies off the frame's middle.
const BEYOND: f64 = 0.06;
const INSIDE: f64 = 0.06;
const SHIFT: f64 = 0.04;
/// A mark's half length across the axis.
pub(crate) const MARK: f64 = 0.014;

/// One of a funnel's ellipses: where it crosses the axis (its short axis's ends, marked), the
/// exact ellipse and where it touches each side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    pub ends: [Point; 2],
    pub exact: Ellipse,
    pub touches: [Point; 2],
}

/// A funnel: its axis and its two sides as printed, its three ellipses from its narrow end, and
/// the unit ways along its axis (towards the wide end) and across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Funnel {
    pub axis: [Point; 2],
    pub sides: [[Point; 2]; 2],
    pub rings: [Ring; 3],
    pub along: Point,
    pub across: Point,
}

impl Funnel {
    /// The funnel `seed` makes.
    #[must_use]
    pub fn generate(seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut last = None;
        for _ in 0..TRIES {
            if let Some(funnel) = laid(&mut random) {
                if funnel.inside() {
                    return funnel;
                }
                last = Some(funnel);
            }
        }
        last.unwrap_or_else(|| shaped([0.0, 1.0], ([0.5, 0.2], 10.0), &[[1.0, 0.3]; 3]))
    }

    /// Whether its sides and its ellipses keep inside the frame.
    fn inside(&self) -> bool {
        let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
        let mut ends = self.sides.iter().flatten().chain(&self.axis);
        let points = ends.all(|p| inside(p[0]) && inside(p[1]));
        // Its reach right and down at most the frame's far side's, left and up its near side's.
        let reach = |e: &Ellipse| {
            [[1.0, 0.0], [0.0, 1.0]]
                .iter()
                .all(|n| e.support(*n) <= 1.0 - INSIDE)
                && [[-1.0, 0.0], [0.0, -1.0]]
                    .iter()
                    .all(|n| e.support(*n) <= -INSIDE)
        };
        points && self.rings.iter().all(|ring| reach(&ring.exact))
    }
}

/// One funnel laid from `random`; none where its ends' roundness lie too near.
fn laid(random: &mut Random) -> Option<Funnel> {
    let off = random.range(OFF_AXIS[0], OFF_AXIS[1]);
    let tan = off.to_radians().tan();
    let ends = [0; 2].map(|_| random.range(ROUND[0], ROUND[1]));
    if (ends[0] - ends[1]).abs() < SPREAD {
        return None;
    }
    let round = [ends[0], (ends[0] + ends[1]) / 2.0, ends[1]];
    // Each ellipse's long half axis over its distance from the apex, and its short one's.
    let long = |r: f64| tan / (1.0 + r * r * tan * tan).sqrt();
    let narrow = random.range(NARROW[0], NARROW[1]);
    let mut at = [narrow / long(round[0]); 3];
    for k in 1..3 {
        let (before, after) = (round[k - 1] * long(round[k - 1]), round[k] * long(round[k]));
        let least = (at[k - 1] * (1.0 + before) + CLEAR) / (1.0 - after);
        at[k] = least.max(at[k - 1] + APART) + random.range(0.0, MORE);
    }
    if at[2] * long(round[2]) > WIDEST {
        return None;
    }
    let turn = random.range(0.0, std::f64::consts::TAU);
    let middle = [0, 1].map(|_| 0.5 + random.range(-SHIFT, SHIFT));
    let placed = std::array::from_fn(|k| [at[k], round[k]]);
    Some(shaped([turn.cos(), turn.sin()], (middle, off), &placed))
}

/// The funnel heading `along` (a unit way, to its wide end), its middle ellipse's centre at
/// `middle`, its sides `off` degrees off its axis, its ellipses each at a distance from the apex
/// with a roundness.
fn shaped(along: Point, (middle, off): (Point, f64), placed: &[[f64; 2]; 3]) -> Funnel {
    let across = [-along[1], along[0]];
    let (sin, cos) = off.to_radians().sin_cos();
    let tan = sin / cos;
    let apex = [0, 1].map(|k| middle[k] - placed[1][0] * along[k]);
    let at =
        |d: f64, side: f64| [0, 1].map(|k| apex[k] + d * along[k] + side * d * tan * across[k]);
    let rings = placed.map(|[d, round]| {
        let long = d * tan / (1.0 + round * round * tan * tan).sqrt();
        let short = round * long;
        let centre = at(d, 0.0);
        let exact = Ellipse {
            centre,
            axes: [long, short],
            angle: upright(across[1].atan2(across[0])),
        };
        let end = |by: f64| [0, 1].map(|k| centre[k] + by * along[k]);
        // Where it touches each side: its point whose normal is the side's, outward.
        let touches = [1.0, -1.0].map(|side: f64| {
            // The side's outward normal along the long axis and the short.
            let (on_long, on_short) = (side * cos, -sin);
            let reach = (long * on_long).hypot(short * on_short);
            [0, 1].map(|k| {
                centre[k]
                    + (long * long * on_long * across[k] + short * short * on_short * along[k])
                        / reach
            })
        });
        Ring {
            ends: [end(-short), end(short)],
            exact,
            touches,
        }
    });
    let from = placed[0][0] - rings[0].exact.axes[1] - BEYOND;
    let to = placed[2][0] + rings[2].exact.axes[1] + BEYOND;
    Funnel {
        axis: [at(from, 0.0), at(to, 0.0)],
        sides: [1.0, -1.0].map(|side| [at(from, side), at(to, side)]),
        rings,
        along,
        across,
    }
}
