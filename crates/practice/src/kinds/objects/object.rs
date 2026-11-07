//! Objects simplified into boxes (S94, V大's weeks 3 and 4 assignment, 几何体 + 点定位 概括单体):
//! an object made from a two-point box by one or two changes (`object_design.rs`), shown as a
//! solid lit from one side (`object_picture.rs`), to be drawn beside it: the box found first from
//! its front corner, three edges and horizon (S84's given), then its points by V大's tricks, big to
//! small (`object_construction.rs`), then the visible edges. The answer is the object in space
//! projected, hidden by its own parts (`object_answer.rs`).
//!
//! An object is laid again from its seed's sequence until it keeps every rule (`object_rules.rs`):
//! the box's (`PerspectiveBox::keeps`), its corners at halves, thirds and quarters a sixth apart, no
//! surface turning by under `CREASE` (a seam the eye could not tell), a round part standing on a
//! top and clear of the blocks, no visible edge under `LEAST_EDGE` (40 pixels of a frame), no two
//! running together, each change showing at least two corners, and every visible edge a step of
//! shade the eye sees (`object_picture.rs`).

use crate::geometry::ellipse::EllipseGuide;
use crate::geometry::perspective::{PerspectiveBox, Place};
use crate::geometry::perspective_eye::{box_corners, directions, fitted, framing};
use crate::kinds::objects::object_answer::seen;
use crate::kinds::objects::object_body::{Body, Placed};
use crate::kinds::objects::object_construction::{Stage, Wanted, construction};
use crate::kinds::objects::object_design::{Carve, Change, Design, Local, SAME, SECOND, designed};
use crate::kinds::objects::object_rules::{creased, fraction, shown, spaced, stands};
use crate::maths::path::Point;

use crate::kinds::points_in_space::space::projected;
use crate::kinds::solids::solid_answer::{SolidAnswer, View};
use crate::maths::random::Random;
use crate::maths::vector::{Vector, add, times, unit};

/// Objects laid before the generator keeps the last one whatever it is.
const TRIES: usize = 400;
/// Of them, two changes laid at most this often before one is enough.
const TWO_TRIES: usize = 60;
/// The box's longer side in the frame, and its middle from the eye (as the tricks' boxes).
const SIDE: [f64; 2] = [0.62, 0.72];
/// A box with a post seen from higher above (the post's ends no slivers), smaller to keep its
/// horizon in the frame.
const POST_SIDE: [f64; 2] = [0.52, 0.62];
const DISTANCE: [f64; 2] = [2.2, 4.0];
/// How often the box lies below the eye (its tops seen), and across it.
const BELOW: f64 = 0.7;
const ACROSS: f64 = 0.9;

/// An object to simplify into its box, generated the same for the same carve and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTask {
    pub carve: Carve,
    /// The box that holds it: its front corner, three edges and horizon given.
    pub seen: PerspectiveBox,
    /// Its visible edges and outlines (`whole` false: hidden stretches are not drawn).
    pub answer: SolidAnswer,
    /// A round part's ends, each its square with its exact ellipse and middles (S93's).
    pub squares: Vec<EllipseGuide>,
    /// V大's construction, the biggest change's stage first.
    pub stages: Vec<Stage>,
    pub(crate) body: Body,
    pub(crate) view: View,
    /// Where the light comes from (one long, towards it).
    pub(crate) light: Vector,
}

impl ObjectTask {
    /// The object `carve` and `seed` make.
    #[must_use]
    pub fn generate(carve: Carve, seed: u64) -> Self {
        let mut random = Random::new(seed);
        // Two changes, when the seed asks them, while they come within `TWO_TRIES`.
        let two = random.unit() < SECOND;
        for k in 0..TRIES {
            if let Some(task) = laid(carve, &mut random, two && k < TWO_TRIES) {
                return task;
            }
        }
        plain(carve)
    }

    /// The box's longer side on the picture: what an object's error is a share of.
    #[must_use]
    pub fn size(&self) -> f64 {
        let low =
            [0, 1].map(|k| (self.seen.corners.iter()).fold(f64::INFINITY, |m, p| m.min(p[k])));
        let high =
            [0, 1].map(|k| (self.seen.corners.iter()).fold(f64::NEG_INFINITY, |m, p| m.max(p[k])));
        (high[0] - low[0]).max(high[1] - low[1])
    }

    /// Where the box's own point `p` lies on the picture: the object in space projected.
    #[must_use]
    pub fn projected(&self, p: [f64; 3]) -> Point {
        self.view.at(self.body.placed.world(p))
    }

    /// Every construction line, every stage's.
    #[must_use]
    pub fn construction(&self) -> Vec<[Point; 2]> {
        (self.stages.iter())
            .flat_map(|stage| stage.lines.iter().copied())
            .collect()
    }

    /// Every corner the construction finds, and a round part's middles.
    #[must_use]
    pub fn found(&self) -> (Vec<Point>, Vec<Point>) {
        let found = self.stages.iter().flat_map(|s| s.found.iter().copied());
        let middles = self.stages.iter().flat_map(|s| s.middles.iter().copied());
        (found.collect(), middles.collect())
    }

    /// Whether the eye sees the box's own point `p` of the object.
    #[must_use]
    pub fn shows(&self, p: [f64; 3]) -> bool {
        let at = self.body.placed.world(p);
        self.body.hit(at).is_none_or(|hit| hit.t >= 1.0 - 1e-6)
    }
}

/// One object of `carve` laid from `random`, of two changes when `two`; none when it breaks a
/// rule.
fn laid(carve: Carve, random: &mut Random, two: bool) -> Option<ObjectTask> {
    let design = designed(carve, random, two)?;
    let post = design.changes.iter().any(|change| change.turned.is_some());
    let (seen_box, placed, view, lights) = looked(design.size, random, post);
    let body = Body::of(&design, placed);
    let early =
        seen_box.keeps(false) && spaced(&design) && creased(&body) && stands(&design, &body);
    if !early {
        return None;
    }
    let (mut task, wanted) = built(carve, &design, (seen_box, body, view, lights[0]));
    let at_fractions = (wanted.iter())
        .flat_map(|want| want.corners.iter().chain(want.ends.iter().flatten()))
        .all(|p| p.iter().all(|v| fraction(*v)));
    if !(at_fractions && task.drawable() && shown(&task, &wanted)) {
        return None;
    }
    // Lit from the other side when every edge does not show so.
    lights.into_iter().find_map(|light| {
        task.light = light;
        task.legible().then(|| task.clone())
    })
}

/// The box `size` seen by the eye looking level, laid from `random`: the box fitted in the frame,
/// where it lies in the world, the view, and the lights it may be lit by (from the left or the
/// right, above; the sequence's first).
fn looked(
    size: [f64; 3],
    random: &mut Random,
    post: bool,
) -> (PerspectiveBox, Placed, View, [Vector; 2]) {
    let yaw = random.range(25.0, 65.0).to_radians();
    let roll = random.unit();
    let place = if post || roll < BELOW {
        Place::Below
    } else if roll < ACROSS {
        Place::Across
    } else {
        Place::Above
    };
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    // Near enough the eye's level that the horizon stays in the frame.
    let height = match place {
        Place::Below if post => -(size[1] / 2.0 + random.range(0.6, 1.3)),
        Place::Below => -(size[1] / 2.0 + random.range(0.1, 0.6)),
        Place::Across => random.range(-0.3, 0.3) * size[1],
        Place::Above => size[1] / 2.0 + random.range(0.1, 0.5),
    };
    let middle = [random.range(-0.2, 0.2) * depth, height, depth];
    let world = box_corners(middle, yaw, size);
    let image = world.map(projected);
    let [left, right, up] = directions(yaw);
    let [low, high] = if post { POST_SIDE } else { SIDE };
    let side = random.range(low, high);
    // A box with a post lies lower in the frame: room above it for the horizon.
    let down = if post { [0.06, 0.12] } else { [-0.04, 0.04] };
    let shift = [random.range(-0.05, 0.05), random.range(down[0], down[1])];
    let (scale, offset) = framing(&image, 0.0, (side, shift), None);
    let vanishing = [Some(projected(left)), Some(projected(right)), None];
    let seen_box = fitted(image, (0.0, vanishing), (side, shift), None);
    let placed = Placed {
        origin: world[0],
        axes: [
            times(right, size[0]),
            times(up, size[1]),
            times(left, size[2]),
        ],
    };
    let from_left = random.unit() < 0.5;
    let light = |from_left: bool| {
        let [lit, dim] = if from_left { [0.9, 0.35] } else { [0.35, 0.9] };
        unit(add(
            times(up, 1.6),
            add(times(right, -lit), times(left, -dim)),
        ))
    };
    let lights = [light(from_left), light(!from_left)];
    (seen_box, placed, View { scale, offset }, lights)
}

/// The object of `design` as `body` in the box `seen_box`: its answer and construction, and what
/// each change asks found.
fn built(
    carve: Carve,
    design: &Design,
    (seen_box, body, view, light): (PerspectiveBox, Body, View, Vector),
) -> (ObjectTask, Vec<Wanted>) {
    let found = seen(&body, view);
    let wanted = wanted(design, &found.corners);
    let task = ObjectTask {
        carve,
        stages: construction(&seen_box, &wanted),
        seen: seen_box,
        answer: found.answer,
        squares: found.guides,
        body,
        view,
        light,
    };
    (task, wanted)
}

/// What each change of `design` asks the construction to find: the corners of the object it
/// made first (of `corners`, none of the box's own), a round part's ends and a cone's point.
fn wanted(design: &Design, corners: &[Local]) -> Vec<Wanted> {
    let own = |p: &Local| p.iter().all(|v| v.abs() <= SAME || (v - 1.0).abs() <= SAME);
    let mut wanted: Vec<Wanted> = design.changes.iter().map(|_| Wanted::default()).collect();
    for corner in corners.iter().filter(|p| !own(p)) {
        let index = (design.changes.iter()).position(|change| claims(change, *corner));
        let index = index.unwrap_or(wanted.len() - 1);
        wanted[index].corners.push(*corner);
    }
    for (change, want) in design.changes.iter().zip(&mut wanted) {
        let Some(turned) = change.turned else {
            continue;
        };
        let end = |at: f64| {
            let (low, high) = (turned.block.low, turned.block.high);
            let [first, second] = match turned.axis {
                0 => [1, 2],
                1 => [0, 2],
                _ => [0, 1],
            };
            let corner = |s: f64, t: f64| {
                let mut p = low;
                (p[turned.axis], p[first], p[second]) = (at, s, t);
                p
            };
            let [s0, s1, t0, t1] = [low[first], high[first], low[second], high[second]];
            [
                corner(s0, t0),
                corner(s1, t0),
                corner(s1, t1),
                corner(s0, t1),
            ]
        };
        let [low, high] = [turned.block.low, turned.block.high].map(|p| p[turned.axis]);
        want.ends.push(end(low));
        if turned.cone {
            want.points.push(end(high));
        } else {
            want.ends.push(end(high));
        }
    }
    wanted
}

/// Whether `corner` is `change`'s: in a block it took out, on a cut's plane, or a corner of its
/// round part's block.
fn claims(change: &Change, corner: Local) -> bool {
    change.out.iter().any(|block| block.holds(corner))
        || change.cuts.iter().any(|cut| cut.passes(corner))
        || change
            .turned
            .is_some_and(|turned| turned.block.holds(corner))
}

/// A plain object, the generator's last resort (never met by a seed tested): the box with a step
/// taken out of its top front, in a box laid from a fixed sequence.
fn plain(carve: Carve) -> ObjectTask {
    let step = crate::kinds::objects::object_design::Block {
        low: [0.0, 0.5, 0.0],
        high: [1.0, 1.0, 0.5],
    };
    let design = Design {
        changes: vec![Change {
            out: vec![step],
            ..Change::default()
        }],
        size: [1.0; 3],
    };
    let (seen_box, placed, view, lights) =
        looked(design.size, &mut Random::new(0x000B_1EC7), false);
    let body = Body::of(&design, placed);
    built(carve, &design, (seen_box, body, view, lights[0])).0
}
