//! Points in space (S90, V大's week 3 点定位): a complete box in two-point perspective, one of its
//! two front faces shaded, and one task by V大's five tricks: the face's centre (its diagonals
//! cross there), the face halved towards its vanishing point (the line through the centre), the
//! face in thirds of its depth (the near part longer), the next face of equal depth beyond it (the
//! far edge's middle and the diagonal through it), a given point's mirror across the face's
//! upright middle (the rectangle between the point and the middle). Every answer is a point of the
//! face in space carried through the eye that sees the box (`perspective_eye.rs`), never one found
//! on the picture: the tricks are exact in perspective, so their lines on the picture land on the
//! answer (`tests/space.rs`), and they are what Compare draws as the construction.
//!
//! A face's own coordinates `[u, v]`: `u` along its depth from its near upright edge (0) to its
//! far one (1), `v` up from its bottom edge (0) to its top (1). Frame units otherwise.

use crate::geometry::ellipse::{apply, homography, inverse};
use crate::geometry::perspective::{Direction, PerspectiveBox, level, place};
use crate::geometry::perspective_eye::{box_corners, directions, fitted, framing};
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;
use crate::sessions::plan::Family;

/// Boxes laid before the generator keeps the last one whatever it is (every seed tested keeps the
/// rules well within).
const TRIES: usize = 400;
/// The box's longer side in the frame; the next face's box smaller, so the face beyond fits.
const SIDE: [f64; 2] = [0.5, 0.62];
const STEP_SIDE: [f64; 2] = [0.34, 0.44];
/// The box's middle from the eye: nearer than S84's boxes (2.6 to 5.5), so its faces recede
/// plainly and a trick's answer lies well off the drawing's middle.
const DISTANCE: [f64; 2] = [2.0, 3.6];
/// A face's near and far upright edges and its depth along its bottom edge at least, so its
/// constructions can be drawn and seen; its far edge at most this share of its near one, so it
/// recedes plainly.
const NEAR_EDGE: f64 = 0.2;
const FAR_EDGE: f64 = 0.1;
const DEPTH: f64 = 0.22;
const RECEDES: f64 = 0.8;
/// The next face's depth along the bottom edge at least.
const STEP_DEPTH: f64 = 0.08;
/// How far inside the frame every answer keeps.
const INSIDE: f64 = 0.03;

/// V大's five tricks for points in space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trick {
    /// The face's centre: its diagonals cross there.
    Centre,
    /// The face halved, upper and lower: the line through its centre to its vanishing point.
    Halves,
    /// The face in three equal depths: two uprights.
    Thirds,
    /// The next face of the same depth beyond it: its far upright.
    Step,
    /// A given point's mirror across the face's upright middle.
    Mirror,
}

impl Trick {
    pub const ALL: [Self; 5] = [
        Self::Centre,
        Self::Halves,
        Self::Thirds,
        Self::Step,
        Self::Mirror,
    ];

    #[must_use]
    pub const fn family(self) -> Family {
        match self {
            Self::Centre => Family::Centre,
            Self::Halves => Family::Halves,
            Self::Thirds => Family::Thirds,
            Self::Step => Family::Step,
            Self::Mirror => Family::Mirror,
        }
    }
}

/// A trick's exact answer, in frame units.
#[derive(Debug, Clone, PartialEq)]
pub enum SpaceAnswer {
    Point(Point),
    Lines(Vec<[Point; 2]>),
}

/// The face in space and how the eye's image of it lies in the frame: its near bottom corner and
/// its edges along its depth and up, in the world (y up, z away from the eye, which looks level).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Face {
    corner: [f64; 3],
    depth: [f64; 3],
    up: [f64; 3],
    scale: f64,
    offset: Point,
}

impl Face {
    /// The face's point `[u, v]` as the eye sees it, in the frame.
    fn seen(&self, [u, v]: Point) -> Point {
        let world: [f64; 3] =
            std::array::from_fn(|k| self.corner[k] + u * self.depth[k] + v * self.up[k]);
        let image = projected(world);
        [
            image[0] * self.scale + self.offset[0],
            image[1] * self.scale + self.offset[1],
        ]
    }
}

/// Where the eye looking level (`Eye` at no pitch) sees the world point or direction `p`. Every
/// point it is asked of lies well in front of it: a box's middle at least 2.0 away, its corners
/// at most 1.3 from its middle, a face's next one as far again, its depth turned 25 to 65 degrees
/// from the eye's line.
pub(crate) fn projected([x, y, z]: [f64; 3]) -> Point {
    let z = z.max(1e-9);
    [x / z, -y / z]
}

/// A point in space to fix on a box, generated the same for the same trick and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceTask {
    pub trick: Trick,
    /// The box, its edges the eye sees drawn in ink.
    pub seen: PerspectiveBox,
    /// The shaded face's corners: near bottom, far bottom, far top, near top.
    pub corners: [Point; 4],
    /// Where its top and bottom edges meet: its depth's vanishing point.
    pub vanishing: Point,
    /// A mirror's given point, in the face's coordinates.
    pub given: Option<Point>,
    face: Face,
}

impl SpaceTask {
    /// The task `trick` seeded `seed`: boxes laid from the seed's sequence until one keeps every
    /// rule (`PerspectiveBox`'s and the face's).
    #[must_use]
    pub fn generate(trick: Trick, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut task = laid(trick, &mut random);
        for _ in 1..TRIES {
            if task.keeps() {
                break;
            }
            task = laid(trick, &mut random);
        }
        task
    }

    /// The face's point `[u, v]` (its own coordinates) as the eye sees it, in the frame: a point
    /// of the box in space projected.
    #[must_use]
    pub fn at(&self, p: Point) -> Point {
        self.face.seen(p)
    }

    /// The face's own coordinates of the frame point `p`: the picture carried back onto the face
    /// (the homography of its corners, exact in perspective).
    #[must_use]
    pub fn on_face(&self, p: Point) -> Point {
        apply(&inverse(&homography(self.corners)), p)
    }

    /// The given point (a mirror's) in the frame.
    #[must_use]
    pub fn given_at(&self) -> Option<Point> {
        self.given.map(|g| self.at(g))
    }

    /// The exact answer: the point or the lines the trick finds, each a point of the face in
    /// space projected.
    #[must_use]
    pub fn answer(&self) -> SpaceAnswer {
        let upright = |u: f64| [self.at([u, 0.0]), self.at([u, 1.0])];
        match self.trick {
            Trick::Centre => SpaceAnswer::Point(self.at([0.5, 0.5])),
            Trick::Halves => SpaceAnswer::Lines(vec![[self.at([0.0, 0.5]), self.at([1.0, 0.5])]]),
            Trick::Thirds => SpaceAnswer::Lines(vec![upright(1.0 / 3.0), upright(2.0 / 3.0)]),
            Trick::Step => SpaceAnswer::Lines(vec![
                [self.at([1.0, 0.0]), self.at([2.0, 0.0])],
                upright(2.0),
                [self.at([2.0, 1.0]), self.at([1.0, 1.0])],
            ]),
            Trick::Mirror => {
                let [u, v] = self.given.unwrap_or([0.25, 0.5]);
                SpaceAnswer::Point(self.at([1.0 - u, v]))
            }
        }
    }

    /// Where the answer lies in the face's own coordinates: a point's, or each upright's `u`
    /// (the halving line's `v`).
    #[must_use]
    pub fn target(&self) -> Vec<f64> {
        match self.trick {
            Trick::Centre => vec![0.5, 0.5],
            Trick::Halves => vec![0.5],
            Trick::Thirds => vec![1.0 / 3.0, 2.0 / 3.0],
            Trick::Step => vec![2.0],
            Trick::Mirror => {
                let [u, v] = self.given.unwrap_or([0.25, 0.5]);
                vec![1.0 - u, v]
            }
        }
    }

    /// The trick's construction as lines on the picture: what Compare draws in pencil. Each line
    /// joins two points of the face (or one and its vanishing point), as the eye sees them.
    #[must_use]
    pub fn construction(&self) -> Vec<[Point; 2]> {
        let at = |u: f64, v: f64| self.at([u, v]);
        let diagonals = [[at(0.0, 0.0), at(1.0, 1.0)], [at(1.0, 0.0), at(0.0, 1.0)]];
        let to_point = |v: f64| [at(0.0, v), self.vanishing];
        match self.trick {
            Trick::Centre => diagonals.to_vec(),
            Trick::Halves => [diagonals[0], diagonals[1], to_point(0.5)].to_vec(),
            Trick::Thirds => vec![to_point(1.0 / 3.0), to_point(2.0 / 3.0), diagonals[0]],
            Trick::Step => vec![
                to_point(0.5),
                [at(0.0, 1.0), at(2.0, 0.0)],
                [at(1.0, 0.0), self.vanishing],
                [at(1.0, 1.0), self.vanishing],
            ],
            Trick::Mirror => {
                let [u, v] = self.given.unwrap_or([0.25, 0.5]);
                let crossed = (u + 0.5) / 2.0;
                vec![
                    diagonals[0],
                    diagonals[1],
                    [at(0.5, 0.0), at(0.5, 1.0)],
                    [at(0.0, v), at(1.0, v)],
                    [at(u, 0.0), at(u, v)],
                    [at(u, 0.0), at(0.5, v)],
                    [at(0.5, 0.0), at(u, v)],
                    [at(crossed, v / 2.0), at(0.5, v / 2.0)],
                    [at(u, v), at(1.0 - u, 0.0)],
                    [at(1.0 - u, 0.0), at(1.0 - u, v)],
                ]
            }
        }
    }

    /// Whether it keeps every rule: the box's, the face wide and deep enough to construct on, and
    /// every answer inside the frame (the next face too).
    fn keeps(&self) -> bool {
        let at = |u: f64, v: f64| self.at([u, v]);
        let inside = |p: Point| p.iter().all(|v| (INSIDE..=1.0 - INSIDE).contains(v));
        let answers = match self.answer() {
            SpaceAnswer::Point(p) => vec![p],
            SpaceAnswer::Lines(lines) => lines.into_iter().flatten().collect(),
        };
        let step = self.trick != Trick::Step || distance(at(1.0, 0.0), at(2.0, 0.0)) >= STEP_DEPTH;
        let near = distance(at(0.0, 0.0), at(0.0, 1.0));
        let far = distance(at(1.0, 0.0), at(1.0, 1.0));
        self.seen.keeps(false)
            && near >= NEAR_EDGE
            && (FAR_EDGE..=RECEDES * near).contains(&far)
            && distance(at(0.0, 0.0), at(1.0, 0.0)) >= DEPTH
            && answers.into_iter().all(inside)
            && step
    }
}

/// One task laid from `random`: a two-point box seen by the eye looking level (S84's boxes, the
/// eye at the world's origin), one of its two front faces and, for a mirror, a point on it.
fn laid(trick: Trick, random: &mut Random) -> SpaceTask {
    let yaw = random.range(25.0, 65.0).to_radians();
    let size = [0; 3].map(|_| random.range(0.6, 1.5));
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    let across = random.range(-0.2, 0.2) * depth;
    let up = level(place(random.unit(), None), size[1], random);
    let world = box_corners([across, up, depth], yaw, size);
    let image = world.map(projected);
    // The two horizontal directions vanish on the horizon (no third: the verticals stay upright).
    let [left, right, _] = directions(yaw).map(projected);
    let vanishing = [Some(left), Some(right), None];
    let [low, high] = if trick == Trick::Step {
        STEP_SIDE
    } else {
        SIDE
    };
    let side = random.range(low, high);
    let shift = [random.range(-0.1, 0.1), random.range(-0.08, 0.08)];
    let (scale, offset) = framing(&image, 0.0, (side, shift), None);
    let seen = fitted(image, (0.0, vanishing), (side, shift), None);
    // Corner 1 lies a width towards the right vanishing point, corner 4 a depth towards the left.
    let (far, direction) = if random.unit() < 0.5 {
        (1, Direction::Right)
    } else {
        (4, Direction::Left)
    };
    let given = (trick == Trick::Mirror).then(|| {
        let u = random.range(0.14, 0.34);
        let u = if random.unit() < 0.5 { u } else { 1.0 - u };
        [u, random.range(0.25, 0.75)]
    });
    let edge = |to: usize| std::array::from_fn(|k| world[to][k] - world[0][k]);
    let vanishing = [left, right][direction.index()];
    SpaceTask {
        trick,
        corners: [0, far, far | 2, 2].map(|i| seen.corners[i]),
        vanishing: [
            vanishing[0] * scale + offset[0],
            vanishing[1] * scale + offset[1],
        ],
        seen,
        given,
        face: Face {
            corner: world[0],
            depth: edge(far),
            up: edge(2),
            scale,
            offset,
        },
    }
}
