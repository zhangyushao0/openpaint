//! A three-point page's uprights (S142): where the page's eye looks down or up, every box's
//! verticals meet at one third point, below or above (V大: far, off the page if need be). The
//! second level gives it, two faint guides leading to it (`guides`); at the third the learner
//! chooses it, and it is found where the uprights drawn pass nearest, the worst left out while
//! more than two remain, as a direction's meeting is found on the horizon. Uprights that meet on
//! no side the eye looks, nearer than a third point may lie (`THIRD`) or past `FOUND_FAR`, meet
//! nowhere: the page's own point is taken. An edge within `UPRIGHT` of its line to the point is an upright; one that
//! misses the point by more than `ON_ANGLE` gets its true upright, the line through its middle to
//! the point, as a two-point page's leaning vertical gets the true vertical.

use crate::geometry::edges::Edge;
use crate::geometry::heading::{Checked, ON_ANGLE, line_angle, sub};
use crate::geometry::off::Off;
use crate::geometry::perspective::{Direction, THIRD};
use crate::maths::path::{Point, distance};

/// An edge within this angle (degrees) of its line to the third point is an upright; the uprights
/// a third point is found from lie within it of the page's vertical.
const UPRIGHT: f64 = 25.0;
/// The farthest from the frame's middle a third point found is taken (frame units).
const FOUND_FAR: f64 = 8.0;
/// A true upright runs this share of its edge beyond either end (as a true vertical does).
const BEYOND: f64 = 0.15;
/// The guides' place across the page, and how far from the horizon they start and from the
/// frame's edge they end.
const GUIDES: [f64; 2] = [0.1, 0.9];
const GAP: f64 = 0.06;
const INSIDE: f64 = 0.06;

/// Where a page's uprights head.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Uprights {
    /// Vertical: the eye looks level.
    Plumb,
    /// For the third point the page gives.
    Given(Point),
    /// For the point the uprights drawn pass nearest, below the horizon (`below`) or above it;
    /// the page's own (`page`) where they meet on neither side.
    Found { below: bool, page: Point },
}

impl Uprights {
    /// The page's third point for the edges `found` on it, its horizon at `horizon`; none on a
    /// two-point page.
    #[must_use]
    pub fn third(self, horizon: f64, found: &[Edge]) -> Option<Point> {
        match self {
            Self::Plumb => None,
            Self::Given(point) => Some(point),
            Self::Found { below, page } => {
                let near_plumb = |edge: &&Edge| line_angle(edge.direction(), [0.0, 1.0]) <= UPRIGHT;
                let met = meeting(found.iter().filter(near_plumb).collect()).filter(|p| {
                    let side = if below {
                        p[1] > horizon
                    } else {
                        p[1] < horizon
                    };
                    side && (THIRD[0]..=FOUND_FAR).contains(&distance(*p, [0.5, 0.5]))
                });
                Some(met.unwrap_or(page))
            }
        }
    }
}

/// Whether `edge` is an upright of a page whose third point is `third`.
pub(crate) fn upright(edge: &Edge, third: Point) -> bool {
    line_angle(edge.direction(), sub(third, edge.middle())) <= UPRIGHT
}

/// `edge`, an upright, checked against `third`: on within `ON_ANGLE`, else its true upright.
pub(crate) fn checked_upright(edge: &Edge, third: Point) -> Checked {
    let middle = edge.middle();
    let toward = sub(third, middle);
    let on = line_angle(edge.direction(), toward) <= ON_ANGLE;
    let true_upright = || {
        let long = toward[0].hypot(toward[1]).max(f64::MIN_POSITIVE);
        let half = edge.length() * (0.5 + BEYOND) / long;
        let d = [toward[0] * half, toward[1] * half];
        [
            [middle[0] - d[0], middle[1] - d[1]],
            [middle[0] + d[0], middle[1] + d[1]],
        ]
    };
    Checked {
        edge: *edge,
        direction: Direction::Vertical,
        off: (!on).then(|| Off::Plumb(true_upright())),
        on,
    }
}

/// A guided page's two faint guides toward `third`: near either side of the page, from beside the
/// horizon at `horizon` to the frame's far edge.
#[must_use]
pub fn guides(horizon: f64, third: Point) -> [[Point; 2]; 2] {
    let (from, to) = if third[1] > horizon {
        (horizon + GAP, 1.0 - INSIDE)
    } else {
        (horizon - GAP, INSIDE)
    };
    GUIDES.map(|x| {
        let t = (to - from) / (third[1] - from);
        [[x, from], [x + (third[0] - x) * t, to]]
    })
}

/// Where the lines of `edges` pass nearest; while one misses it by more than `ON_ANGLE` and more
/// than two remain, the worst is left out and the point found again. None for fewer than two, or
/// for lines that never meet.
fn meeting(mut edges: Vec<&Edge>) -> Option<Point> {
    loop {
        let meet = nearest(&edges)?;
        let off = |edge: &Edge| line_angle(edge.direction(), sub(meet, edge.middle()));
        let worst = (0..edges.len()).max_by(|a, b| off(edges[*a]).total_cmp(&off(edges[*b])));
        match worst {
            Some(at) if edges.len() > 2 && off(edges[at]) > ON_ANGLE => {
                edges.remove(at);
            }
            _ => return Some(meet),
        }
    }
}

/// The point whose distances from the lines of `edges`, squared and weighed by each edge's length,
/// add up least; none for fewer than two lines or lines all but parallel.
fn nearest(edges: &[&Edge]) -> Option<Point> {
    let [mut a, mut b, mut c, mut x, mut y] = [0.0; 5];
    for edge in edges {
        let d = edge.direction();
        let n = [-d[1], d[0]];
        let (w, m) = (edge.length(), edge.middle());
        let along = n[0] * m[0] + n[1] * m[1];
        a += w * n[0] * n[0];
        b += w * n[0] * n[1];
        c += w * n[1] * n[1];
        x += w * n[0] * along;
        y += w * n[1] * along;
    }
    let det = a * c - b * b;
    (edges.len() >= 2 && det > 1e-9 * (a + c) * (a + c))
        .then(|| [(c * x - b * y) / det, (a * y - b * x) / det])
}
