//! Where a straight edge heads (S84, S97): an edge "heads for" a point when its line passes within
//! `ON_ANGLE` of the line from its middle to the point (a miss of 2 degrees is about 3 pixels over a
//! 100-pixel edge, unseen on the edge, but some 20 pixels at a vanishing point a frame away, where
//! the extension shows it); one that misses is drawn on, the way Drawabox's rough-perspective check
//! does, past where it comes nearest the point, and a leaning vertical gets the true vertical
//! through it. What is off on a straight part read where one stroke is asked (`off.rs`). Frame
//! units.

use crate::geometry::edges::Edge;
use crate::geometry::perspective::angle_between;
use crate::maths::path::{Point, distance};

/// An edge heads for a point within this angle (degrees); a two-point vertical leans past it.
pub const ON_ANGLE: f64 = 2.0;
/// A true vertical runs this share of its edge beyond either end.
const PLUMB: f64 = 0.15;
/// How far an extension runs past where its line comes nearest its point (frame units): past the
/// point's ring at the sheet's view.
pub const PAST: f64 = 0.06;
/// How far beyond a point (frame units) from the frame's middle an extension may run.
const BEYOND: f64 = 0.25;

/// How far from the frame's middle an extension towards any of `points` may run: a little beyond
/// the farthest, a frame at least.
pub(crate) fn reach(points: &[Point]) -> f64 {
    (points.iter())
        .map(|p| distance(*p, [0.5, 0.5]) + BEYOND)
        .fold(1.0, f64::max)
}

/// `edge` drawn on from its end nearer `point` to `PAST` beyond where its line passes nearest the
/// point; never past `reach` from the frame's middle. None when it already got there.
pub(crate) fn toward(edge: &Edge, point: Point, reach: f64) -> Option<[Point; 2]> {
    let mut d = edge.direction();
    if dot(d, sub(point, edge.middle())) < 0.0 {
        d = [-d[0], -d[1]];
    }
    let [a, b] = edge.ends;
    let from = if dot(d, sub(b, a)) >= 0.0 { b } else { a };
    let t = (dot(sub(point, from), d) + PAST).min(within(from, d, reach));
    (t.is_finite() && t > 1e-6).then(|| [from, [from[0] + t * d[0], from[1] + t * d[1]]])
}

/// How far along `d` from `from` the frame's middle stays within `reach`.
fn within(from: Point, d: Point, reach: f64) -> f64 {
    let rel = [from[0] - 0.5, from[1] - 0.5];
    let b = dot(rel, d);
    let c = dot(rel, rel) - reach * reach;
    let disc = b * b - c;
    if disc < 0.0 {
        0.0
    } else {
        (-b + disc.sqrt()).max(0.0)
    }
}

/// The true vertical through `edge`'s middle, a little beyond its ends.
pub(crate) fn plumb(edge: &Edge) -> [Point; 2] {
    let [a, b] = edge.ends;
    let (top, bottom) = (a[1].min(b[1]), a[1].max(b[1]));
    let more = PLUMB * edge.length();
    let x = edge.middle()[0];
    [[x, top - more], [x, bottom + more]]
}

/// The angle between two lines of directions `a` and `b`, degrees (0 to 90).
pub(crate) fn line_angle(a: Point, b: Point) -> f64 {
    let angle = angle_between(a, b);
    angle.min(180.0 - angle)
}

pub(crate) fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
