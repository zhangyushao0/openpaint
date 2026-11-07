//! Boxes in perspective (V大's week-2 透视盒子, after Drawabox's boxes too): a box seen by a
//! pinhole eye, generated the same for the same seed. Near is bigger, far is smaller; the edges
//! of one direction meet at its vanishing point; the edges parallel to the ground meet on the
//! horizon, the eye's level. Two-point: the eye looks level, so the verticals stay vertical and
//! the two horizontal directions vanish on the horizon. Three-point: the eye looks up or down, so
//! the verticals meet too, at a third point above or below.
//!
//! Every box keeps V大's rules: the front angle (the box's nearest corner, between its two
//! horizontal edges, as drawn) wider than 90 degrees, about 120; vanishing points not too close
//! (`NEAR`), at sane distances off the frame at most (`FAR`); a third vanishing point far but near
//! enough that the verticals visibly lean (`THIRD`); every proportion and angle varied. A box to
//! complete is given by its front corner and the three edges from it (Drawabox's "Y"), the corner
//! where three faces show (of a box across the horizon, the one whose edges are steeper); no face
//! is a sliver; a free-boxes page shows one box on its horizon as an example.
//!
//! Frame units throughout (the frame's side is 1, y down).

use crate::path::{Point, distance};
use crate::perspective_eye::{Eye, box_corners, directions, fitted};
use crate::random::Random;

/// How far a vanishing point may lie from the frame's middle (frame units).
pub const FAR: f64 = 3.0;
/// How near the box's front corner a left or right vanishing point may come.
const NEAR: f64 = 0.55;
/// A three-point box's third vanishing point, from the frame's middle: at least and at most.
const THIRD: [f64; 2] = [1.3, 4.0];
/// The front angle's range, degrees: wider than 90, about 120 (V大).
const FRONT: [f64; 2] = [100.0, 150.0];
/// The given edges' least length, and any edge's; a face's least width (its area over its longest
/// side), so no face is a sliver whose edges run together.
const GIVEN: f64 = 0.12;
const SHORTEST: f64 = 0.04;
const FACE: f64 = 0.05;
/// The six faces, each its corners in order round.
const FACES: [[usize; 4]; 6] = [
    [0, 1, 5, 4],
    [2, 3, 7, 6],
    [0, 1, 3, 2],
    [4, 5, 7, 6],
    [0, 4, 6, 2],
    [1, 5, 7, 3],
];
/// How far inside the frame the box keeps, and the horizon.
const INSIDE: f64 = 0.06;
/// Boxes laid before the generator keeps the last one whatever it is (every seed tested keeps the
/// rules well within).
const TRIES: usize = 400;

/// Whether a box's verticals stay parallel (two-point) or meet at a third point (three-point).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoxView {
    TwoPoint,
    ThreePoint,
}

impl BoxView {
    pub const ALL: [Self; 2] = [Self::TwoPoint, Self::ThreePoint];
}

/// Where a free-boxes page's horizon lies (V大: a low horizon sees the boxes from below, a high
/// one from above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Horizon {
    Low,
    Middle,
    High,
}

impl Horizon {
    pub const ALL: [Self; 3] = [Self::Low, Self::Middle, Self::High];

    /// Its height in the frame, down from the top.
    #[must_use]
    pub const fn height(self) -> f64 {
        match self {
            Self::Low => 0.72,
            Self::Middle => 0.5,
            Self::High => 0.28,
        }
    }
}

/// The direction of a box's edge: towards the left or the right vanishing point, or up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Vertical,
}

impl Direction {
    pub const ALL: [Self; 3] = [Self::Left, Self::Right, Self::Vertical];

    /// Its place in `ALL` (and in a box's vanishing points).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Vertical => 2,
        }
    }
}

/// A box in perspective: its eight corners, the horizon, each direction's vanishing point and
/// the front corner its exercise gives.
///
/// Corner `i` holds bit 0 when it lies a width towards the right vanishing point, bit 1 a height
/// up, bit 2 a depth towards the left one: corners 0 and 2 are the front edge's, nearest the eye.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveBox {
    pub corners: [Point; 8],
    /// The horizon's height in the frame, down from the top.
    pub horizon: f64,
    /// Each direction's vanishing point (`Direction::index`); none where its edges stay parallel
    /// (a two-point box's verticals).
    pub vanishing: [Option<Point>; 3],
    /// The corner given with its three edges: the front edge's end nearer the horizon, where
    /// three faces show; of a box across the horizon, the end farther from it.
    pub front: usize,
}

/// A box's twelve edges: two corners, the first nearer the eye (or lower), and the direction.
pub const EDGES: [(usize, usize, Direction); 12] = [
    (0, 1, Direction::Right),
    (2, 3, Direction::Right),
    (4, 5, Direction::Right),
    (6, 7, Direction::Right),
    (0, 2, Direction::Vertical),
    (1, 3, Direction::Vertical),
    (4, 6, Direction::Vertical),
    (5, 7, Direction::Vertical),
    (0, 4, Direction::Left),
    (1, 5, Direction::Left),
    (2, 6, Direction::Left),
    (3, 7, Direction::Left),
];

/// Where a box lies against the eye's level: below it (its top seen), across it, above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    Below,
    Across,
    Above,
}

impl PerspectiveBox {
    /// The box `view` and `seed` make: laid from the seed's sequence until it keeps every rule.
    #[must_use]
    pub fn generate(view: BoxView, seed: u64) -> Self {
        Self::kept(view, seed, None)
    }

    /// A two-point box on a page whose horizon lies at `horizon`, smaller (one of a few), seen
    /// from below under a low horizon and from above under a high one: a free-boxes page's
    /// example.
    #[must_use]
    pub fn on_horizon(horizon: Horizon, seed: u64) -> Self {
        Self::kept(BoxView::TwoPoint, seed, Some(horizon))
    }

    fn kept(view: BoxView, seed: u64, page: Option<Horizon>) -> Self {
        let mut random = Random::new(seed);
        let mut last = None;
        for _ in 0..TRIES {
            if let Some(laid) = Self::laid(view, &mut random, page) {
                if laid.keeps(page.is_none()) {
                    return laid;
                }
                last = Some(laid);
            }
        }
        last.unwrap_or_else(Self::plain)
    }

    /// A plain two-point box, the generator's last resort (never met by a seed tested).
    fn plain() -> Self {
        let corners = [
            [0.5, 0.75],
            [0.75, 0.62],
            [0.5, 0.45],
            [0.75, 0.36],
            [0.27, 0.63],
            [0.5, 0.53],
            [0.27, 0.37],
            [0.5, 0.29],
        ];
        Self {
            corners,
            horizon: 0.2,
            vanishing: [Some([-0.7, 0.2]), Some([1.7, 0.2]), None],
            front: 2,
        }
    }

    /// One box laid from `random`, fitted in the frame (none when the eye cannot see it whole).
    fn laid(view: BoxView, random: &mut Random, page: Option<Horizon>) -> Option<Self> {
        let pitch = match view {
            BoxView::TwoPoint => 0.0,
            BoxView::ThreePoint => random.sign() * random.range(12.0, 26.0).to_radians(),
        };
        let yaw = random.range(25.0, 65.0).to_radians();
        let size = [0; 3].map(|_| random.range(0.6, 1.5));
        let depth = random.range(2.6, 5.5);
        let across = random.range(-0.2, 0.2) * depth;
        let up = match view {
            BoxView::TwoPoint => level(place(random.unit(), page), size[1], random),
            BoxView::ThreePoint => random.range(-0.15, 0.15) * depth,
        };
        let eye = Eye { pitch };
        let world = eye.to_world([across, up, depth]);
        let corners = box_corners(world, yaw, size);
        let below = corners.iter().all(|c| c[1] < 0.0);
        let above = corners.iter().all(|c| c[1] > 0.0);
        let seen = match view {
            BoxView::TwoPoint => true,
            BoxView::ThreePoint => (pitch < 0.0 && below) || (pitch > 0.0 && above),
        };
        let image: Vec<Point> = corners.iter().filter_map(|c| eye.image(*c)).collect();
        let image: [Point; 8] = image.try_into().ok().filter(|_| seen)?;
        let directions = directions(yaw);
        let vanishing = directions.map(|d| eye.vanishing(d));
        let side = match page {
            Some(_) => random.range(0.3, 0.4),
            None => random.range(0.48, 0.6),
        };
        let shift = [random.range(-0.04, 0.04), random.range(-0.04, 0.04)];
        Some(fitted(image, (pitch.tan(), vanishing), (side, shift), page))
    }

    /// Whether it keeps every rule: inside the frame, its vanishing points at sane distances,
    /// its front angle wide, its edges long enough (the given ones too, when `given`).
    pub(crate) fn keeps(&self, given: bool) -> bool {
        let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
        let middle = [0.5, 0.5];
        let front = self.corners[self.front];
        let sides = [Direction::Left, Direction::Right].iter().all(|d| {
            self.vanishing[d.index()]
                .is_some_and(|p| distance(p, middle) <= FAR && distance(p, front) >= NEAR)
        });
        let third = match self.vanishing[Direction::Vertical.index()] {
            Some(p) => (THIRD[0]..=THIRD[1]).contains(&distance(p, middle)),
            None => true,
        };
        let angle = self.front_angle();
        let edges = EDGES.iter().all(|(a, b, _)| {
            let long = distance(self.corners[*a], self.corners[*b]);
            let given_edge = *a == self.front || *b == self.front;
            long >= if given && given_edge { GIVEN } else { SHORTEST }
        });
        inside(self.horizon)
            && self.corners.iter().all(|c| inside(c[0]) && inside(c[1]))
            && sides
            && third
            && (FRONT[0]..=FRONT[1]).contains(&angle)
            && edges
            && FACES.iter().all(|face| self.width(face) >= FACE)
    }

    /// How wide `face` shows: its area over its longest side.
    fn width(&self, face: &[usize; 4]) -> f64 {
        let p = face.map(|i| self.corners[i]);
        let area = (0..4)
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % 4]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            .abs()
            / 2.0;
        let longest = (0..4)
            .map(|i| distance(p[i], p[(i + 1) % 4]))
            .fold(0.0, f64::max);
        area / longest.max(f64::MIN_POSITIVE)
    }

    /// The front angle, degrees: the narrower of those the two horizontal edges make at either
    /// end of the front edge (the end farther from the horizon's).
    #[must_use]
    pub fn front_angle(&self) -> f64 {
        [(0, 1, 4), (2, 3, 6)]
            .iter()
            .map(|&(at, right, left)| {
                let c = self.corners;
                angle_between(sub(c[right], c[at]), sub(c[left], c[at]))
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// Its edges as two points each, with their directions.
    #[must_use]
    pub fn edges(&self) -> [(Point, Point, Direction); 12] {
        EDGES.map(|(a, b, d)| (self.corners[a], self.corners[b], d))
    }

    /// Whether each of `EDGES` is hidden from the eye: those of the far bottom corner unless the
    /// box lies above the horizon (its bottom seen), those of the far top corner unless it lies
    /// below (its top seen).
    #[must_use]
    pub fn hidden(&self) -> [bool; 12] {
        let above = self.corners.iter().all(|c| c[1] < self.horizon);
        let below = self.corners.iter().all(|c| c[1] > self.horizon);
        EDGES.map(|(a, b, _)| {
            let touches = |corner: usize| a == corner || b == corner;
            (touches(5) && !above) || (touches(7) && !below)
        })
    }

    /// The three edges its exercise gives: those from the front corner.
    #[must_use]
    pub fn given(&self) -> Vec<(Point, Point, Direction)> {
        EDGES
            .iter()
            .filter(|(a, b, _)| *a == self.front || *b == self.front)
            .map(|&(a, b, d)| {
                let other = if a == self.front { b } else { a };
                (self.corners[self.front], self.corners[other], d)
            })
            .collect()
    }
}

/// Where a box goes against the eye's level, from `roll` (0 to 1) and the page's horizon.
pub(crate) fn place(roll: f64, page: Option<Horizon>) -> Place {
    let [below, across] = match page {
        None => [0.45, 0.75],
        Some(Horizon::High) => [0.7, 1.0],
        Some(Horizon::Low) => [0.0, 0.3],
        Some(Horizon::Middle) => [0.4, 0.6],
    };
    if roll < below {
        Place::Below
    } else if roll < across {
        Place::Across
    } else {
        Place::Above
    }
}

/// A box's middle's height against the eye, for a box `height` high placed so.
pub(crate) fn level(place: Place, height: f64, random: &mut Random) -> f64 {
    match place {
        Place::Below => -(height / 2.0 + random.range(0.1, 1.0)),
        Place::Across => random.range(-0.3, 0.3) * height,
        Place::Above => height / 2.0 + random.range(0.1, 0.8),
    }
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

/// The angle between the vectors `a` and `b`, degrees (0 to 180).
pub(crate) fn angle_between(a: Point, b: Point) -> f64 {
    let dot = a[0] * b[0] + a[1] * b[1];
    let cross = a[0] * b[1] - a[1] * b[0];
    cross.abs().atan2(dot).to_degrees()
}
