//! How the eye sees a placement's ground (S153): looking level at 入门 and 熟练 (two points: the
//! uprights stay upright, the ground's edges vanish on the horizon), looking down at 稳 (three
//! points, the uprights meeting below); the ground turned 25 to 65 degrees from the eye's line, its
//! near corner nearest, the eye above every piece; its image fitted in the copy's frame with the
//! horizon. The world: y up, z away, the eye at the origin, the ground below it.
//!
//! A sight keeps the rules of V大's boxes where they apply (`keeps`): everything inside the frame,
//! the horizon in it above the pieces, the ground's vanishing points at sane distances and not too
//! near its corner (`FAR`), a third point far but near enough that the uprights visibly lean
//! (`THIRD`), the ground's near angle wide, no sliver of a ground and no footprint edge too short to
//! read.

use crate::geometry::perspective::{FAR, THIRD, angle_between};
use crate::geometry::perspective_eye::{directions, projected};
use crate::kinds::placements::plan::Plan;
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;
use crate::maths::vector::{Vector, add, times};

/// The ground's turn from the eye's line, degrees; at 稳 how far the eye looks down.
const YAW: [f64; 2] = [25.0, 65.0];
const PITCH: [f64; 2] = [18.0, 30.0];
/// The ground's middle from the eye, over the ground's longer side; looking level, the eye's height
/// over that side; looking down, how far above the eye's line the middle lies, over its distance.
const DISTANCE: [f64; 2] = [1.7, 2.5];
const HEIGHT: [f64; 2] = [0.5, 0.8];
const RISE: [f64; 2] = [-0.04, 0.08];
/// How far the eye stands above the tallest piece at least (units); how far the ground's middle
/// lies off the eye's line sideways, over its distance.
const ABOVE: f64 = 0.8;
const ACROSS: f64 = 0.12;
/// The scene's width in the frame, its height with the horizon at most, where its lowest point
/// lies, and how far its middle moves off the frame's.
const WIDTH: [f64; 2] = [0.70, 0.80];
const TALL: f64 = 0.76;
const BOTTOM: [f64; 2] = [0.84, 0.89];
const SHIFT: f64 = 0.03;
/// How far inside the frame everything keeps; how far the horizon keeps above the pieces.
const INSIDE: f64 = 0.06;
const CLEAR: f64 = 0.04;
/// How near the ground's near corner its vanishing points may come; the ground's near angle,
/// degrees; its least width (its area over its longest side); a footprint's shortest edge.
const NEAR: f64 = 0.55;
const FRONT: [f64; 2] = [100.0, 160.0];
const NARROWEST: f64 = 0.12;
const SHORTEST: f64 = 0.03;

/// How the eye sees the ground: how far it looks down (radians, negative; none at two points), the
/// ground's turn, its near corner in the world, and its image in the copy's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sight {
    pub(crate) pitch: f64,
    pub(crate) yaw: f64,
    pub(crate) near: Vector,
    pub(crate) view: View,
}

impl Sight {
    /// A sight of `plan` laid from `random`, looking down where `down`; none where the eye would
    /// stand too low.
    pub(crate) fn looked(plan: &Plan, down: bool, random: &mut Random) -> Option<Self> {
        let yaw = random.range(YAW[0], YAW[1]).to_radians();
        let pitch = if down {
            -random.range(PITCH[0], PITCH[1]).to_radians()
        } else {
            0.0
        };
        let [width, depth] = plan.ground;
        let distance = random.range(DISTANCE[0], DISTANCE[1]) * width.max(depth);
        let across = random.range(-ACROSS, ACROSS) * distance;
        let tallest = (plan.pieces.iter()).fold(0.0, |most: f64, piece| most.max(piece.height));
        let rise = if down {
            random.range(RISE[0], RISE[1]) * distance
        } else {
            -(random.range(HEIGHT[0], HEIGHT[1]) * width.max(depth)).max(tallest + ABOVE)
        };
        let middle = world_of(pitch, [across, rise, distance]);
        if -middle[1] < tallest + ABOVE {
            return None;
        }
        let [left, right, _] = directions(yaw);
        let near = add(
            add(middle, times(right, -width / 2.0)),
            times(left, -depth / 2.0),
        );
        let unfitted = View {
            scale: 1.0,
            offset: [0.0, 0.0],
        };
        let mut sight = Self {
            pitch,
            yaw,
            near,
            view: unfitted,
        };
        sight.view = sight.fitted(plan, random)?;
        Some(sight)
    }

    /// The view that lays the scene's image in the frame: its width `WIDTH` (its height with the
    /// horizon `TALL` at most), its lowest point at `BOTTOM`; none where a point lies behind the
    /// eye.
    fn fitted(&self, plan: &Plan, random: &mut Random) -> Option<View> {
        let mut points: Vec<Point> = Vec::new();
        for p in self.scene(plan) {
            let viewed = self.viewed(p);
            if viewed[2] <= 0.3 {
                return None;
            }
            points.push(projected(viewed));
        }
        let horizon = self.pitch.tan();
        let low = [0, 1].map(|k| points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
        let high = [0, 1].map(|k| {
            points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let top = low[1].min(horizon);
        let width = random.range(WIDTH[0], WIDTH[1]);
        let scale = (width / (high[0] - low[0])).min(TALL / (high[1] - top));
        let bottom = random.range(BOTTOM[0], BOTTOM[1]);
        let shift = random.range(-SHIFT, SHIFT);
        let offset = [
            0.5 + shift - (low[0] + high[0]) / 2.0 * scale,
            bottom - high[1] * scale,
        ];
        Some(View { scale, offset })
    }

    /// Every point of the scene in the world: the ground's corners, each piece's footprint's at
    /// its foot and top, the unit upright's top.
    fn scene(&self, plan: &Plan) -> Vec<Vector> {
        let [width, depth] = plan.ground;
        let mut points: Vec<Vector> = [[0.0, 0.0], [width, 0.0], [width, depth], [0.0, depth]]
            .map(|p| self.world(p, 0.0))
            .to_vec();
        for piece in &plan.pieces {
            for corner in plan.corners(piece) {
                points.extend([0.0, piece.height].map(|h| self.world(corner, h)));
            }
        }
        points.push(self.world([0.0, 0.0], 1.0));
        points
    }

    /// The world point at `[along, away]` on the plan, `height` above the ground.
    pub(crate) fn world(&self, [along, away]: Point, height: f64) -> Vector {
        let [left, right, up] = directions(self.yaw);
        let on = add(add(self.near, times(right, along)), times(left, away));
        add(on, times(up, height))
    }

    /// `p` in the eye's view: right, up, forward.
    pub(crate) fn viewed(&self, p: Vector) -> Vector {
        let (sin, cos) = self.pitch.sin_cos();
        [p[0], p[1] * cos - p[2] * sin, p[1] * sin + p[2] * cos]
    }

    /// Where the world point `p` lies in the copy's frame.
    pub(crate) fn at(&self, p: Vector) -> Point {
        self.view.at(self.viewed(p))
    }

    /// Where the plan's point `p` lies in the copy's frame, `height` above the ground.
    pub(crate) fn seen(&self, p: Point, height: f64) -> Point {
        self.at(self.world(p, height))
    }

    /// Where the world direction `d` vanishes in the copy's frame; none where it lies parallel to
    /// the picture.
    pub(crate) fn vanishing(&self, d: Vector) -> Option<Point> {
        let [x, y, z] = self.viewed(d);
        let View { scale, offset } = self.view;
        (z.abs() > 1e-9).then(|| [x / z * scale + offset[0], -y / z * scale + offset[1]])
    }

    /// The ground's vanishing points, to the left, to the right and up (`Direction`'s order).
    pub(crate) fn vanishing_points(&self) -> [Option<Point>; 3] {
        directions(self.yaw).map(|d| self.vanishing(d))
    }

    /// The horizon's height in the copy's frame.
    pub(crate) fn horizon(&self) -> f64 {
        self.view.offset[1] + self.view.scale * self.pitch.tan()
    }

    /// Whether it keeps the rules for `plan` (the module's).
    pub(crate) fn keeps(&self, plan: &Plan) -> bool {
        let points: Vec<Point> = self.scene(plan).into_iter().map(|p| self.at(p)).collect();
        let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
        let horizon = self.horizon();
        let highest = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let [width, depth] = plan.ground;
        let ground =
            [[0.0, 0.0], [width, 0.0], [width, depth], [0.0, depth]].map(|p| self.seen(p, 0.0));
        let [left, right, up] = self.vanishing_points();
        let middle = [0.5, 0.5];
        let sides = [left, right].iter().all(|v| {
            v.is_some_and(|v| distance(v, middle) <= FAR && distance(v, ground[0]) >= NEAR)
        });
        let third = match (self.pitch != 0.0, up) {
            (true, Some(v)) => (THIRD[0]..=THIRD[1]).contains(&distance(v, middle)),
            (false, _) => true,
            (true, None) => false,
        };
        let sub = |a: Point, b: Point| [a[0] - b[0], a[1] - b[1]];
        let angle = angle_between(sub(ground[1], ground[0]), sub(ground[3], ground[0]));
        let edges = plan.pieces.iter().all(|piece| {
            let feet = plan.corners(piece).map(|p| self.seen(p, 0.0));
            (0..4).all(|k| distance(feet[k], feet[(k + 1) % 4]) >= SHORTEST)
        });
        points.iter().all(|p| inside(p[0]) && inside(p[1]))
            && inside(horizon)
            && horizon <= highest - CLEAR
            && sides
            && third
            && (FRONT[0]..=FRONT[1]).contains(&angle)
            && narrowest(&ground) >= NARROWEST
            && edges
    }
}

/// The world point at `p` in the view of an eye looking down `-pitch` (its view: right, up,
/// forward).
fn world_of(pitch: f64, p: Vector) -> Vector {
    let (sin, cos) = pitch.sin_cos();
    [p[0], p[1] * cos + p[2] * sin, -p[1] * sin + p[2] * cos]
}

/// How wide the quadrilateral `quad` shows: its area over its longest side.
fn narrowest(quad: &[Point; 4]) -> f64 {
    let area = (0..4)
        .map(|i| {
            let (a, b) = (quad[i], quad[(i + 1) % 4]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        .abs()
        / 2.0;
    let longest = (0..4)
        .map(|i| distance(quad[i], quad[(i + 1) % 4]))
        .fold(0.0, f64::max);
    area / longest.max(f64::MIN_POSITIVE)
}
