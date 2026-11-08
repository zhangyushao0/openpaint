//! Solids in boxes (S93, V大's week 3: never draw a cylinder straight off, draw the box first): a
//! cylinder with square ends, upright or lying along either horizontal direction; a cone on a
//! square face, its point at the opposite face's centre, upright or lying; a sphere in a cube.
//! Each lies in a two-point box drawn whole (its hidden edges dashed), seen by `space.rs`'s eye.
//!
//! V大 finds a solid in its box by 点定位: a square face's diagonals cross at its centre, lines
//! through it towards the vanishing points (or upright) meet the edges at their middles, and the
//! ellipse through the four middles touches the edges there; a cylinder's other end alike and two
//! lines touching both; a cone's point where the opposite face's diagonals cross, and two lines
//! from it touching the base; a sphere's middle section (the box halved in height by its side
//! faces' diagonals) and its ellipse, then a circle through the ends of that ellipse's long axis.
//! The answer is the solid in space projected (`solid_answer.rs`), so V大's lines on the picture
//! land on it (`tests/solids/`). His sphere is "not exact, but fine in most views": cubes are laid
//! only where his circle lies within the Draw pen's width (`PEN_WIDTH`) of the exact outline.
//!
//! No end or section is a sliver (`ROUND`: the horizon not through it), no side short
//! (`SIDE_LEAST`: a cylinder not seen down its axis, a cone's point not inside its base) and the
//! box keeps V大's rules, its vanishing points not too close (`PerspectiveBox::keeps`).
//!
//! Frame units throughout (the frame's side is 1, y down).

use crate::geometry::ellipse::{EllipseGuide, square_on};
use crate::geometry::off::PEN_WIDTH;
use crate::geometry::perspective::{PerspectiveBox, level, place};
use crate::geometry::perspective_eye::projected;
use crate::geometry::perspective_eye::{box_corners, directions, fitted, framing};
use crate::kinds::solids::solid_answer::{Body, SolidAnswer, Square, View, answer};
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;
use crate::maths::vector::{Vector, add, sub};
use crate::sessions::plan::Family;

/// Boxes laid before the generator keeps the last one whatever it is (every seed tested keeps
/// the rules well within).
const TRIES: usize = 400;
/// The box's longer side in the frame; a sphere's cube smaller, so V大's circle stays near.
const SIDE: [f64; 2] = [0.5, 0.62];
const CUBE: [f64; 2] = [0.42, 0.52];
/// The box's middle from the eye, as near as the tricks' boxes.
const DISTANCE: [f64; 2] = [2.2, 4.0];
/// A cylinder's and a cone's length along its axis, shares of its square's side.
const LONG: [f64; 2] = [1.1, 2.0];
const CONE_LONG: [f64; 2] = [1.0, 1.8];
/// Every ellipse at least this round (minor over major, about 13 degrees): no sliver.
const ROUND: f64 = 0.22;
/// A side at least this long on the picture.
const SIDE_LEAST: f64 = 0.1;
/// How far a sphere's centre lies off the horizon (degrees), so its middle section is no sliver
/// and V大's circle stays near; and across, off the eye's line.
const SECTION: [f64; 2] = [11.0, 19.0];
const ACROSS: f64 = 9.0;
/// Outlines compared at this many points.
const PIECES: usize = 360;

/// The solids V大 finds in a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Solid {
    Cylinder,
    Cone,
    Sphere,
}

impl Solid {
    pub const ALL: [Self; 3] = [Self::Cylinder, Self::Cone, Self::Sphere];

    #[must_use]
    pub const fn family(self) -> Family {
        match self {
            Self::Cylinder => Family::Cylinder,
            Self::Cone => Family::Cone,
            Self::Sphere => Family::Sphere,
        }
    }
}

/// How a solid's axis lies: upright, or along the left or the right vanishing direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    Upright,
    Left,
    Right,
}

impl Pose {
    /// The bit of a box's corners (`PerspectiveBox`) its axis runs along.
    const fn bit(self) -> usize {
        match self {
            Self::Right => 1,
            Self::Upright => 2,
            Self::Left => 4,
        }
    }
}

/// A solid to find in its box, generated the same for the same solid and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidTask {
    pub solid: Solid,
    pub pose: Pose,
    /// The box, every edge drawn (the hidden ones dashed).
    pub seen: PerspectiveBox,
    /// The squares V大 finds ellipses in, each with its exact ellipse and four middles: a
    /// cylinder's ends (the one seen whole first), a cone's base, a sphere's middle section.
    pub squares: Vec<EllipseGuide>,
    /// A cone's face opposite its base, its corners round: its diagonals cross at the point.
    pub top: Option<[Point; 4]>,
    pub point: Option<Point>,
    /// A sphere's cube's two side faces seen (near bottom, far bottom, far top, near top): halved
    /// in height through their diagonals' crossings.
    pub halved: Vec<[Point; 4]>,
    pub answer: SolidAnswer,
}

impl SolidTask {
    /// The task of `solid` seeded `seed`: boxes laid from the seed's sequence until one keeps
    /// every rule.
    #[must_use]
    pub fn generate(solid: Solid, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut task = laid(solid, &mut random);
        for _ in 1..TRIES {
            if task.keeps() {
                break;
            }
            task = laid(solid, &mut random);
        }
        task
    }

    /// The box's longer side on the picture: what a solid's error is a share of.
    #[must_use]
    pub fn size(&self) -> f64 {
        let low =
            [0, 1].map(|k| (self.seen.corners.iter()).fold(f64::INFINITY, |m, p| m.min(p[k])));
        let high =
            [0, 1].map(|k| (self.seen.corners.iter()).fold(f64::NEG_INFINITY, |m, p| m.max(p[k])));
        (high[0] - low[0]).max(high[1] - low[1])
    }

    /// V大's circle for a sphere: about the middle section's ellipse's centre, through the ends of
    /// its long axis.
    #[must_use]
    pub fn circle(&self) -> Option<(Point, f64)> {
        let section = self
            .squares
            .first()
            .filter(|_| self.solid == Solid::Sphere)?;
        Some((section.exact.centre, section.exact.axes[0]))
    }

    /// How far V大's circle lies from the exact outline at most, either way.
    #[must_use]
    pub fn circle_off(&self) -> Option<f64> {
        let (centre, radius) = self.circle()?;
        let outline = self.answer.ellipses.first()?;
        let step = std::f64::consts::TAU / PIECES as f64;
        let from_outline = (0..PIECES)
            .map(|k| (distance(outline.at(k as f64 * step), centre) - radius).abs())
            .fold(0.0, f64::max);
        let from_circle = (0..PIECES)
            .map(|k| {
                let (sin, cos) = (k as f64 * step).sin_cos();
                outline.distance([centre[0] + radius * cos, centre[1] + radius * sin])
            })
            .fold(0.0, f64::max);
        Some(from_outline.max(from_circle))
    }

    /// V大's construction as lines on the picture, what Compare draws in pencil: each square's
    /// diagonals and the lines through its centre to its middles; a cone's opposite face's
    /// diagonals; a sphere's side faces' diagonals and the lines halving them, round its middle
    /// section, before that section's.
    #[must_use]
    pub fn construction(&self) -> Vec<[Point; 2]> {
        let mut lines = self.halving();
        for square in &self.squares {
            lines.extend(diagonals(&square.corners));
            lines.extend(middle_lines(square));
        }
        if let Some(top) = &self.top {
            lines.extend(diagonals(top));
        }
        lines
    }

    /// A sphere's box halved in height: its side faces' diagonals, the lines through their
    /// crossings round the middle section (none for another solid).
    #[must_use]
    pub fn halving(&self) -> Vec<[Point; 2]> {
        let Some(section) = self.squares.first().filter(|_| self.solid == Solid::Sphere) else {
            return Vec::new();
        };
        let mut lines: Vec<[Point; 2]> = self.halved.iter().flat_map(diagonals).collect();
        let [front, right, back, left] = section.corners;
        lines.extend([[front, left], [front, right], [right, back], [left, back]]);
        lines
    }

    /// Every middle V大 finds: where each square's ellipse touches its edges.
    #[must_use]
    pub fn middles(&self) -> Vec<Point> {
        self.squares
            .iter()
            .flat_map(|square| square.touches)
            .collect()
    }

    /// Whether it keeps every rule.
    fn keeps(&self) -> bool {
        let ellipses = || {
            (self.squares.iter().map(|square| &square.exact))
                .chain(&self.answer.ellipses)
                .all(|e| e.roundness() >= ROUND)
        };
        let sides = || match self.solid {
            Solid::Sphere => self.circle_off().is_some_and(|off| off <= PEN_WIDTH),
            Solid::Cylinder | Solid::Cone => {
                self.answer.sides.len() == 2
                    && (self.answer.sides.iter()).all(|[a, b]| distance(*a, *b) >= SIDE_LEAST)
            }
        };
        self.seen.keeps(false) && ellipses() && sides()
    }
}

/// `corners`' (in order round) two diagonals.
#[must_use]
pub fn diagonals(corners: &[Point; 4]) -> [[Point; 2]; 2] {
    [[corners[0], corners[2]], [corners[1], corners[3]]]
}

/// The lines through `square`'s centre between the middles of its opposite edges.
#[must_use]
pub fn middle_lines(square: &EllipseGuide) -> [[Point; 2]; 2] {
    let t = square.touches;
    [[t[0], t[2]], [t[1], t[3]]]
}

/// One task laid from `random`: a two-point box seen by the eye looking level, sized for its
/// solid (square ends, a cube for a sphere), the solid in it.
fn laid(solid: Solid, random: &mut Random) -> SolidTask {
    let yaw = random.range(25.0, 65.0).to_radians();
    let pose = match (solid, random.unit()) {
        (Solid::Sphere, _) => Pose::Upright,
        (_, roll) if roll < 0.5 => Pose::Upright,
        (_, roll) if roll < 0.75 => Pose::Left,
        _ => Pose::Right,
    };
    let square = random.range(0.7, 1.2);
    let [low, high] = match solid {
        Solid::Cylinder => LONG,
        Solid::Cone => CONE_LONG,
        Solid::Sphere => [1.0, 1.0],
    };
    let long = square * random.range(low, high);
    let size = match pose {
        Pose::Upright => [square, long, square],
        Pose::Right => [long, square, square],
        Pose::Left => [square, square, long],
    };
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    let middle = if solid == Solid::Sphere {
        // Seen from above more often than from below.
        let below = random.unit() < 0.65;
        let up = random.range(SECTION[0], SECTION[1]).to_radians().tan() * depth;
        let across = random.range(-ACROSS, ACROSS).to_radians().tan() * depth;
        [across, if below { -up } else { up }, depth]
    } else {
        let across = random.range(-0.2, 0.2) * depth;
        [
            across,
            level(place(random.unit(), None), size[1], random),
            depth,
        ]
    };
    let world = box_corners(middle, yaw, size);
    let image = world.map(projected);
    let [left, right, _] = directions(yaw).map(projected);
    let [low, high] = if solid == Solid::Sphere { CUBE } else { SIDE };
    let side = random.range(low, high);
    let shift = [random.range(-0.06, 0.06), random.range(-0.05, 0.05)];
    let (scale, offset) = framing(&image, 0.0, (side, shift), None);
    let seen = fitted(
        image,
        (0.0, [Some(left), Some(right), None]),
        (side, shift),
        None,
    );
    let base_far = random.unit() < 0.5;
    built(
        (solid, pose),
        seen,
        &world,
        (View { scale, offset }, base_far),
    )
}

/// The solid of `solid` in the box `seen` (its corners `world` in space) through `view`; a lying
/// cone's base on the far end when `base_far`.
fn built(
    (solid, pose): (Solid, Pose),
    seen: PerspectiveBox,
    world: &[Vector; 8],
    (view, base_far): (View, bool),
) -> SolidTask {
    let at = |p: Vector| view.at(p);
    let bit = pose.bit();
    let (body, squares, top) = match solid {
        Solid::Cylinder => {
            let ends = nearer_first([face(world, bit, false), face(world, bit, true)]);
            (Body::Cylinder(ends), ends.to_vec(), None)
        }
        Solid::Cone => {
            let far = base_far && pose != Pose::Upright;
            let (base, top) = (face(world, bit, far), face(world, bit, !far));
            (Body::Cone(base, top.centre()), vec![base], Some(top))
        }
        Solid::Sphere => {
            let middle =
                |a: usize, b: usize| add(world[a], sub(world[b], world[a]).map(|v| v / 2.0));
            let section = Square {
                corner: middle(0, 2),
                u: sub(world[1], world[0]),
                v: sub(world[4], world[0]),
            };
            let radius = section.u.iter().map(|v| v * v).sum::<f64>().sqrt() / 2.0;
            (Body::Sphere(section.centre(), radius), vec![section], None)
        }
    };
    let guides: Vec<EllipseGuide> = (squares.iter())
        .map(|square| square_on(square.corners().map(at)))
        .collect();
    let halved = if solid == Solid::Sphere {
        vec![
            [0, 4, 6, 2].map(|i| seen.corners[i]),
            [0, 1, 3, 2].map(|i| seen.corners[i]),
        ]
    } else {
        Vec::new()
    };
    SolidTask {
        solid,
        pose,
        seen,
        answer: answer(&body, view, &guides),
        squares: guides,
        top: top.map(|face| face.corners().map(at)),
        point: top.map(|face| at(face.centre())),
        halved,
    }
}

/// The box's face across `bit` (its corners holding it when `far`, else not), from the corner
/// with fewest bits along the other two in order.
fn face(world: &[Vector; 8], bit: usize, far: bool) -> Square {
    let [i, j] = match bit {
        1 => [2, 4],
        2 => [1, 4],
        _ => [1, 2],
    };
    let first = if far { bit } else { 0 };
    Square {
        corner: world[first],
        u: sub(world[first | i], world[first]),
        v: sub(world[first | j], world[first]),
    }
}

/// A cylinder's ends, the one whose face the eye sees first (its outward side towards the eye).
fn nearer_first([a, b]: [Square; 2]) -> [Square; 2] {
    let axis = sub(b.centre(), a.centre());
    // `b`'s outward side runs along the axis; it faces the eye where the eye lies that way.
    let b_seen = (0..3).map(|k| axis[k] * -b.centre()[k]).sum::<f64>() > 0.0;
    if b_seen { [b, a] } else { [a, b] }
}
