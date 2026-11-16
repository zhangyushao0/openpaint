//! A form intersection's task (S156k): two forms laid to pass into each other (`placing.rs`) in
//! one box's perspective, the main form's block seen as the solids' boxes are (`PerspectiveBox`,
//! its rules kept); both printed drawn through at 入门 and 熟练 (what the eye sees whole, what is
//! hidden or inside the other dashed), only what the eye sees of them at 稳; the exact line where
//! they meet (`pairs.rs`) seen and hidden by the depth test against both.
//!
//! The rules hold seed after seed or the forms are laid again: the box's rules (its front angle 100
//! to 150 degrees); both forms whole in the frame; the meeting line's seen part 0.15 long at least;
//! each pierce point 0.05 at least from the outline of the form it pierces, legible.

use crate::geometry::ellipse::{EllipseGuide, square_on};
use crate::geometry::perspective::PerspectiveBox;
use crate::geometry::perspective_eye::{box_corners, directions, fitted, framing, projected};
use crate::kinds::contours::seen::{SEEN, State, stretches};
use crate::kinds::intersections::forms::Form;
use crate::kinds::intersections::pairs::{Met, met};
use crate::kinds::intersections::placing::formed;
use crate::kinds::intersections::track::Track;
use crate::kinds::objects::object_body::{Body, Placed};
use crate::kinds::solids::solid_answer::{Stretch, View};
use crate::maths::path::{Point, distance, length, nearest_on_segment};
use crate::maths::random::Random;
use crate::maths::vector::Vector;

/// Scenes laid before the generator keeps a plain one.
const TRIES: usize = 600;
/// The main form's block's longer side in the frame; how far it lies from the eye.
const SIDE: [f64; 2] = [0.5, 0.6];
const DISTANCE: [f64; 2] = [3.0, 4.6];
/// How often the scene lies below the eye's level (else across it); how far its top lies under
/// that level then (world units) and how far below the frame's middle its block is laid.
const BELOW: f64 = 0.7;
const UNDER: [f64; 2] = [0.5, 1.0];
const LOWER: [f64; 2] = [0.08, 0.15];
/// The rules: paper kept round the forms; the meeting line's seen length; a seen pierce point's
/// room from the outline of the form it pierces; round a corner less, where the three it shows lie
/// on the main box's top and its back edges, never all as far in.
const MARGIN: f64 = 0.04;
const SEEN_LONG: f64 = 0.15;
const LEGIBLE: f64 = 0.05;
const CORNER_LEGIBLE: f64 = 0.03;
/// Within this (world units) a point lies on a surface, not inside the form.
const TOUCH: f64 = 1e-9;
/// A form's surfaces are numbered from here, the other's after.
const SURFACES: [u32; 2] = [0, 100];

/// A form intersection's variant: which two forms meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Meeting {
    /// A box into a box across one of its edges (入门).
    Edge,
    /// A box into a box round one of its corners (入门).
    Corner,
    /// A cylinder through a box's face (熟练).
    CylinderIntoBox,
    /// A box into a cylinder's side (熟练).
    BoxIntoCylinder,
    /// A thinner cylinder into a cylinder's side, square to it (稳).
    Cylinders,
    /// A ball sunk into a cylinder's side (稳).
    Sphere,
}

impl Meeting {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 6] = [
        Self::Edge,
        Self::Corner,
        Self::CylinderIntoBox,
        Self::BoxIntoCylinder,
        Self::Cylinders,
        Self::Sphere,
    ];

    /// Whether both forms are printed drawn through (else only what the eye sees of them).
    #[must_use]
    pub const fn through(self) -> bool {
        !matches!(self, Self::Cylinders | Self::Sphere)
    }

    /// How far a seen pierce point keeps from the outline of the form it pierces.
    const fn legible(self) -> f64 {
        match self {
            Self::Corner => CORNER_LEGIBLE,
            _ => LEGIBLE,
        }
    }
}

/// How the line where they meet is found, in the frame: lines carried along faces and sides,
/// sections' squares (their sides and diagonals) and circles, the points they find.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Way {
    pub lines: Vec<[Point; 2]>,
    pub squares: Vec<EllipseGuide>,
    pub rings: Vec<EllipseGuide>,
    pub points: Vec<Point>,
}

/// A form intersection, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionTask {
    pub meeting: Meeting,
    /// The main form's block as the eye sees it: its horizon, vanishing points, front angle.
    pub seen: PerspectiveBox,
    /// Both forms' lines as printed: seen whole, hidden or inside the other dashed (or left out).
    pub printed: Vec<Stretch>,
    /// Where they meet, exact: seen and hidden.
    pub answer: Vec<Stretch>,
    /// Where an edge or a side's line pierces the other form.
    pub pierced: Vec<Point>,
    pub found: Way,
    pub(crate) forms: [Form; 2],
    pub(crate) placed: Placed,
    pub(crate) body: Body,
    pub(crate) view: View,
    pub(crate) met: Met,
}

impl IntersectionTask {
    /// The task `meeting` and `seed` make.
    #[must_use]
    pub fn generate(meeting: Meeting, seed: u64) -> Self {
        let mut random = Random::new(seed);
        (0..TRIES)
            .find_map(|_| laid(meeting, &mut random, true))
            .unwrap_or_else(|| {
                let mut random = Random::new(0x0156_1A75);
                laid(meeting, &mut random, false).unwrap_or_else(|| unreachable!("no rules"))
            })
    }

    /// Whether the eye sees `p`, a point of a surface: nothing nearer on its ray.
    pub(crate) fn sees(&self, p: Vector) -> bool {
        self.body.hit(p).is_none_or(|hit| hit.t >= 1.0 - SEEN)
    }
}

/// One scene laid from `random`; none when it breaks a rule (and `rules`).
fn laid(meeting: Meeting, random: &mut Random, rules: bool) -> Option<IntersectionTask> {
    let forms = formed(meeting, random);
    let (seen, placed, view) = looked(&forms[0], random);
    let mut body = Body {
        placed,
        hulls: Vec::new(),
        rounds: Vec::new(),
        balls: Vec::new(),
    };
    for (form, surface) in forms.iter().zip(SURFACES) {
        form.built(&mut body, placed, surface);
    }
    let met = met(&forms, placed);
    let mut task = IntersectionTask {
        meeting,
        seen,
        printed: Vec::new(),
        answer: Vec::new(),
        pierced: met.pierce.iter().map(|(p, _)| view.at(*p)).collect(),
        found: found(&met, view),
        forms,
        placed,
        body,
        view,
        met,
    };
    task.answer = task.meeting_line();
    task.printed = task.printed_lines();
    (!rules || task.keeps()).then_some(task)
}

/// The main form's block seen by the eye looking level from above or across its level, turned
/// 25 to 65 degrees, fitted in the frame: its box, the scene in the world, the view.
fn looked(main: &Form, random: &mut Random) -> (PerspectiveBox, Placed, View) {
    let size = match *main {
        Form::Box(block) | Form::Cylinder(block, _) => block.high,
        Form::Sphere(_, radius) => [2.0 * radius; 3],
    };
    let yaw = random.range(25.0, 65.0).to_radians();
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    // Seen from above, its top no sliver and the frame's lower half its place, the horizon in it.
    let below = random.unit() < BELOW;
    let (height, lower) = if below {
        let under = random.range(UNDER[0], UNDER[1]);
        (-(size[1] / 2.0 + under), random.range(LOWER[0], LOWER[1]))
    } else {
        (
            random.range(-0.25, 0.25) * size[1],
            random.range(-0.05, 0.05),
        )
    };
    let across = random.range(-0.15, 0.15) * depth;
    let middle = [across, height, depth];
    let world = box_corners(middle, yaw, size);
    let image = world.map(projected);
    let [left, right, up] = directions(yaw);
    let side = random.range(SIDE[0], SIDE[1]);
    let shift = [random.range(-0.05, 0.05), lower];
    let (scale, offset) = framing(&image, 0.0, (side, shift), None);
    let vanishing = [Some(projected(left)), Some(projected(right)), None];
    let seen = fitted(image, (0.0, vanishing), (side, shift), None);
    let placed = Placed {
        origin: world[0],
        axes: [right, up, left],
    };
    (seen, placed, View { scale, offset })
}

/// `met`'s way to find the line, in the frame.
fn found(met: &Met, view: View) -> Way {
    let seen = |circle: &crate::kinds::contours::sections::Circle| {
        square_on(circle.square().map(|p| view.at(p)))
    };
    Way {
        lines: met
            .lines
            .iter()
            .map(|line| line.map(|p| view.at(p)))
            .collect(),
        squares: met.squares.iter().map(seen).collect(),
        rings: met.rings.iter().map(seen).collect(),
        points: met.found.iter().map(|p| view.at(*p)).collect(),
    }
}

impl IntersectionTask {
    /// Where the forms meet, in stretches seen and hidden.
    fn meeting_line(&self) -> Vec<Stretch> {
        let state = |p: Vector| {
            if self.sees(p) {
                State::Seen
            } else {
                State::Hidden
            }
        };
        (self.met.tracks.iter())
            .flat_map(|track| along(track, self.view, &state))
            .collect()
    }

    /// Both forms' lines as printed: seen where on the surface the eye sees, else (inside the
    /// other or hidden) dashed where drawn through, left out where not.
    fn printed_lines(&self) -> Vec<Stretch> {
        let through = self.meeting.through();
        let mut printed = Vec::new();
        for (own, form) in self.forms.iter().enumerate() {
            let other = self.forms[1 - own];
            let state = |p: Vector| {
                let inside = other.off(self.placed.local(p)) < -TOUCH;
                if !inside && self.sees(p) {
                    State::Seen
                } else if through {
                    State::Hidden
                } else {
                    State::Off
                }
            };
            for track in form.lines(self.placed) {
                printed.extend(along(&track, self.view, &state));
            }
        }
        printed
    }

    /// Whether it keeps every rule.
    fn keeps(&self) -> bool {
        let inside = |p: &Point| (0..2).all(|k| (MARGIN..=1.0 - MARGIN).contains(&p[k]));
        let every = (self.forms.iter()).flat_map(|form| form.hull_points(self.placed));
        let framed = every.map(|p| self.view.at(p)).all(|p| inside(&p));
        let seen: f64 = (self.answer.iter())
            .filter(|s| !s.hidden)
            .map(|s| length(&s.points))
            .sum();
        let legible = (self.met.pierce.iter())
            .filter(|(p, _)| self.sees(*p))
            .all(|(p, k)| room(&self.outline(*k), self.view.at(*p)) >= self.meeting.legible());
        self.seen.keeps(false) && framed && seen >= SEEN_LONG && legible
    }

    /// The outline of form `k`'s picture: the convex hull of its points seen.
    pub(crate) fn outline(&self, k: usize) -> Vec<Point> {
        let points: Vec<Point> = (self.forms[k].hull_points(self.placed).into_iter())
            .map(|p| self.view.at(p))
            .collect();
        convex_hull(points)
    }
}

/// `track` through `view` in stretches as `state` says of its points.
fn along(track: &Track, view: View, state: &impl Fn(Vector) -> State) -> Vec<Stretch> {
    stretches(&|t| track.at(t), track.span(), view, &|t| {
        state(track.at(t))
    })
}

/// How far `p` lies from the outline `hull` (a closed polygon) at least.
pub(crate) fn room(hull: &[Point], p: Point) -> f64 {
    (0..hull.len())
        .map(|k| {
            let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
            distance(p, nearest_on_segment(a, b, p))
        })
        .fold(f64::INFINITY, f64::min)
}

/// The convex hull of `points`, in order round (Andrew's monotone chain).
fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let turn = |o: Point, a: Point, b: Point| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let chain = |ordered: &mut dyn Iterator<Item = Point>| {
        let mut held: Vec<Point> = Vec::new();
        for p in ordered {
            while let [.., a, b] = held[..]
                && turn(a, b, p) <= 0.0
            {
                held.pop();
            }
            held.push(p);
        }
        held.pop();
        held
    };
    let mut hull = chain(&mut points.iter().copied());
    hull.extend(chain(&mut points.iter().rev().copied()));
    hull
}
