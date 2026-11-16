//! A plane in perspective (S156, 鬼影面): four corners round a convex quadrilateral, which is a
//! square seen in perspective whatever its shape. Its lines to draw: the four edges, the two
//! diagonals, and through the diagonals' crossing (its centre) a line toward where each pair of
//! opposite edges meet, from one of the other edges to the other: V大's trick 2, the plane halved
//! in perspective. Where a pair of edges runs parallel, its line through the centre runs parallel
//! to them.
//!
//! Laid round a centre near the frame's middle, a corner each quarter turn give or take `TURN`, and
//! kept to its rules (`keeps`): convex, every edge `EDGE` long, every corner's angle within
//! `ANGLES`, each vanishing point at least `FAR` from its centre (edges meeting close by make no
//! plane the eye believes).

use crate::maths::path::{Point, distance, meet};
use crate::maths::random::Random;

/// The centre's place on both axes, its corners' distance from it and how far a corner may turn
/// from its quarter (radians).
const CENTRE: [f64; 2] = [0.42, 0.58];
const REACH: [f64; 2] = [0.27, 0.42];
const TURN: f64 = 0.38;
/// The shortest edge, the corners' angles (degrees) and how far a vanishing point lies from the
/// centre at least (frame units).
const EDGE: f64 = 0.25;
const ANGLES: [f64; 2] = [55.0, 130.0];
const FAR: f64 = 1.0;

/// A plane's corners in order round it: `a`, `b`, `c`, `d`; `ab` and `dc` opposite, `ad` and
/// `bc` opposite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub corners: [Point; 4],
}

impl Plane {
    /// A plane laid from `random`, its rules not yet asked.
    pub(crate) fn laid(random: &mut Random) -> Self {
        let centre = [CENTRE, CENTRE].map(|[low, high]| random.range(low, high));
        let start = random.range(0.0, std::f64::consts::TAU);
        let corners = [0.0, 1.0, 2.0, 3.0].map(|k: f64| {
            let turn = start + k * std::f64::consts::FRAC_PI_2 + random.range(-TURN, TURN);
            let reach = random.range(REACH[0], REACH[1]);
            [
                centre[0] + reach * turn.cos(),
                centre[1] + reach * turn.sin(),
            ]
        });
        Self { corners }
    }

    /// Its centre: where its diagonals cross.
    #[must_use]
    pub fn centre(&self) -> Point {
        let [a, b, c, d] = self.corners;
        meet([a, c], [b, d]).unwrap_or([(a[0] + c[0]) / 2.0, (a[1] + c[1]) / 2.0])
    }

    /// Where each pair of opposite edges meet: `ab` with `dc`, then `ad` with `bc`; none for a pair
    /// that runs parallel.
    #[must_use]
    pub fn vanishing(&self) -> [Option<Point>; 2] {
        let [a, b, c, d] = self.corners;
        [meet([a, b], [d, c]), meet([a, d], [b, c])]
    }

    /// The two lines through its centre: toward where `ab` and `dc` meet, from `ad` to `bc`; toward
    /// where `ad` and `bc` meet, from `ab` to `dc` (each along its pair where they run parallel).
    #[must_use]
    pub fn middles(&self) -> [[Point; 2]; 2] {
        let [a, b, c, d] = self.corners;
        let o = self.centre();
        let [first, second] = self.vanishing();
        let through = |vanishing: Option<Point>, [from, to]: [Point; 2]| {
            let [dx, dy] = vanishing.map_or([to[0] - from[0], to[1] - from[1]], |v| {
                [v[0] - o[0], v[1] - o[1]]
            });
            [o, [o[0] + dx, o[1] + dy]]
        };
        let ends = |line: [Point; 2], [one, other]: [[Point; 2]; 2]| {
            [one, other].map(|edge| meet(line, edge).unwrap_or(o))
        };
        [
            ends(through(first, [a, b]), [[a, d], [b, c]]),
            ends(through(second, [a, d]), [[a, b], [d, c]]),
        ]
    }

    /// Its edges in order round it.
    #[must_use]
    pub fn edges(&self) -> [[Point; 2]; 4] {
        let [a, b, c, d] = self.corners;
        [[a, b], [b, c], [c, d], [d, a]]
    }

    /// Its lines to draw, each once: the four edges, the two diagonals, the two middles.
    #[must_use]
    pub fn lines(&self) -> Vec<[Point; 2]> {
        let [a, b, c, d] = self.corners;
        let mut lines = self.edges().to_vec();
        lines.extend([[a, c], [b, d]]);
        lines.extend(self.middles());
        lines
    }

    /// Whether it keeps its rules: convex, every edge `EDGE` long, every corner's angle within
    /// `ANGLES`, each vanishing point at least `FAR` from its centre.
    pub(crate) fn keeps(&self) -> bool {
        let at = |k: usize| self.corners[k % 4];
        let turns: Vec<f64> = (0..4)
            .map(|k| {
                let ([ax, ay], [bx, by], [cx, cy]) = (at(k), at(k + 1), at(k + 2));
                (bx - ax) * (cy - by) - (by - ay) * (cx - bx)
            })
            .collect();
        let convex = turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0);
        let edges = (0..4).all(|k| distance(at(k), at(k + 1)) >= EDGE);
        let angles = (0..4).all(|k| {
            let (before, here, after) = (at(k + 3), at(k), at(k + 1));
            let [u, v] = [before, after].map(|p| [p[0] - here[0], p[1] - here[1]]);
            let cos = (u[0] * v[0] + u[1] * v[1]) / (u[0].hypot(u[1]) * v[0].hypot(v[1]));
            (ANGLES[0]..=ANGLES[1]).contains(&cos.clamp(-1.0, 1.0).acos().to_degrees())
        });
        let centre = self.centre();
        let far = (self.vanishing().iter().flatten()).all(|v| distance(*v, centre) >= FAR);
        convex && edges && angles && far
    }
}
