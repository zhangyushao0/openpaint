//! What the perspective demonstrations share (S92, S97): an example box whose vanishing points the
//! card has room for, the horizon across a wide example, an edge ruled a hair off or drawn off its
//! aim (so Compare's red shows it apart from the ink), the answer's red rings.

use crate::demos::demo::{Drawn, Marker, On, Script, WIDE_REACH, clearest, line};
use crate::geometry::perspective::{Direction, PerspectiveBox};
use crate::maths::path::{Point, distance};

/// Seeds of the sequence tried for a box whose vanishing points the card has room for.
const TRIES: u64 = 96;
/// How far off its aim the hand draws the edge drawn off (degrees).
const MISS: f64 = 5.0;

/// Of the boxes `make` lays from `seed`'s sequence whose two vanishing points lie within
/// `WIDE_REACH` of the frame's sides, the one widest beside the distance between its points, so
/// it reads largest on the card.
pub(crate) fn within_reach(seed: u64, make: impl Fn(u64) -> PerspectiveBox) -> PerspectiveBox {
    clearest((seed, TRIES), make, |exact| {
        let [Some(left), Some(right)] = [exact.vanishing[0], exact.vanishing[1]] else {
            return -1.0;
        };
        let reach = -WIDE_REACH..=1.0 + WIDE_REACH;
        if !(reach.contains(&left[0]) && reach.contains(&right[0])) {
            return -1.0;
        }
        let xs = exact.corners.map(|c| c[0]);
        let wide = xs.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b))
            - xs.iter().fold(f64::INFINITY, |a, b| a.min(*b));
        wide / distance(left, right).max(f64::MIN_POSITIVE)
    })
}

/// The horizon across a wide example.
pub(crate) fn horizon(height: f64) -> Drawn {
    line([-WIDE_REACH - 0.2, height], [1.2 + WIDE_REACH, height])
}

/// `exact`'s vanishing point for `direction`, if its edges meet.
pub(crate) fn point(exact: &PerspectiveBox, direction: Direction) -> Option<Point> {
    exact.vanishing[direction.index()]
}

/// An edge from `a` to `b` as a hand rules it: its ends a hair off.
pub(crate) fn ruled(a: Point, b: Point, turn: usize) -> Drawn {
    let off = if turn.is_multiple_of(2) {
        0.004
    } else {
        -0.004
    };
    line([a[0] + off, a[1] - off], [b[0] - off, b[1] + off])
}

/// An edge from `a` to `b` drawn `degrees` off its aim, turned about `a`: the hand's miss.
fn turned(a: Point, b: Point, degrees: f64) -> Point {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let v = [b[0] - a[0], b[1] - a[1]];
    [
        a[0] + cos * v[0] - sin * v[1],
        a[1] + sin * v[0] + cos * v[1],
    ]
}

/// `edges` of `exact` ruled with Draw one after another, each once (the `first`'s hair off one
/// way); the one at `off`, if any, drawn `MISS` off its aim.
pub(crate) fn rule_edges(
    script: &mut Script,
    exact: &PerspectiveBox,
    edges: &[(usize, usize, Direction)],
    (first, off): (usize, Option<usize>),
) {
    for (turn, (a, b, _)) in edges.iter().enumerate() {
        let (a, mut b) = (exact.corners[*a], exact.corners[*b]);
        if Some(turn) == off {
            b = turned(a, b, MISS);
        }
        script.draw(On::Copy, Marker::Ink, ruled(a, b, turn + first));
    }
}

/// The rings of `points` in the answer's red.
pub(crate) fn red_rings(points: impl Iterator<Item = Point>) -> Vec<(On, Marker, Drawn)> {
    points
        .map(|p| (On::Copy, Marker::Answer, Drawn::Ring(p)))
        .collect()
}
