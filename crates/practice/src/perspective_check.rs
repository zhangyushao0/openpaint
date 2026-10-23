//! Compare for boxes in perspective (S84): geometry made visible, never a score. Every exercise
//! here has an exact answer or an exact rule, so the check extends the copy's own edges (found by
//! `edges.rs`) the way Drawabox's rough-perspective check does: each edge drawn on towards where
//! its direction vanishes, so a wrong convergence shows as lines missing the point.
//!
//! A box to complete (here): each edge found is taken for the direction whose vanishing point its
//! line passes nearest (in angle), and extended from its end nearer that point until it meets the
//! horizon or comes level with the point (a three-point box's verticals: the third point's
//! height). A two-point vertical has no point: the true vertical is drawn through it instead.
//! Free boxes, with no answer but the rules: `perspective_free.rs`.
//!
//! An edge "heads for" a point when its line passes within `ON_ANGLE` of the line from its middle
//! to the point: a miss of 2 degrees is about 3 pixels over a 100-pixel edge, unseen on the edge,
//! but some 20 pixels at a vanishing point a frame away, where the extension shows it.

use crate::edges::Edge;
use crate::path::{Point, distance};
use crate::perspective::{Direction, PerspectiveBox, angle_between};

/// An edge heads for a point within this angle (degrees); a two-point vertical leans past it.
pub const ON_ANGLE: f64 = 2.0;
/// An edge is taken for a box's direction only within this angle of it (degrees).
const TAKEN: f64 = 20.0;
/// A true vertical runs this share of its edge beyond either end.
const PLUMB: f64 = 0.15;

/// One edge of the copy as checked: the direction it was taken for, its extension towards that
/// direction's vanishing point (from its end nearer the point), or the true vertical through it,
/// and whether it heads for its point (a vertical: stays vertical).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Checked {
    pub edge: Edge,
    pub direction: Direction,
    pub extension: Option<[Point; 2]>,
    pub plumb: Option<[Point; 2]>,
    pub on: bool,
}

/// A completed box as checked against the exact one.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxCheck {
    pub edges: Vec<Checked>,
}

impl BoxCheck {
    /// The edges `found` in the copy checked against `exact`'s vanishing points; an edge near no
    /// direction of the box (`TAKEN`) is left out.
    #[must_use]
    pub fn of(exact: &PerspectiveBox, found: &[Edge]) -> Self {
        let middle = [0.5, 0.5];
        let reach = (exact.vanishing.iter().flatten())
            .map(|p| distance(*p, middle) + 0.25)
            .fold(1.0, f64::max);
        let edges = (found.iter())
            .filter_map(|edge| against(exact, edge, reach))
            .collect();
        Self { edges }
    }
}

/// `edge` taken for the direction of `exact` its line lies nearest, extended within `reach` of the
/// frame's middle.
fn against(exact: &PerspectiveBox, edge: &Edge, reach: f64) -> Option<Checked> {
    let middle = edge.middle();
    let off = |d: Direction| match exact.vanishing[d.index()] {
        Some(p) => line_angle(edge.direction(), sub(p, middle)),
        None => line_angle(edge.direction(), [0.0, 1.0]),
    };
    let (direction, angle) = (Direction::ALL.iter())
        .map(|d| (*d, off(*d)))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    if angle > TAKEN {
        return None;
    }
    let on = angle <= ON_ANGLE;
    let Some(point) = exact.vanishing[direction.index()] else {
        return Some(Checked {
            edge: *edge,
            direction,
            extension: None,
            plumb: Some(plumb(edge)),
            on,
        });
    };
    // The horizontal directions stop on the horizon or at their point's side; the third at its
    // height.
    let side = (direction != Direction::Vertical).then_some(point[0]);
    let level = if direction == Direction::Vertical {
        point[1]
    } else {
        exact.horizon
    };
    Some(Checked {
        edge: *edge,
        direction,
        extension: extended(edge, point, (level, side), reach),
        plumb: None,
        on,
    })
}

/// `edge` drawn on from its end nearer `towards` until it meets the height `level` or, given, the
/// side `side`; never past `reach` from the frame's middle. None when it already got there.
pub(crate) fn extended(
    edge: &Edge,
    towards: Point,
    (level, side): (f64, Option<f64>),
    reach: f64,
) -> Option<[Point; 2]> {
    let mut d = edge.direction();
    let middle = edge.middle();
    if dot(d, sub(towards, middle)) < 0.0 {
        d = [-d[0], -d[1]];
    }
    let [a, b] = edge.ends;
    let from = if dot(d, sub(b, a)) >= 0.0 { b } else { a };
    let mut stops = Vec::new();
    if d[1].abs() > 1e-9 {
        stops.push((level - from[1]) / d[1]);
    }
    if let Some(side) = side
        && d[0].abs() > 1e-9
    {
        stops.push((side - from[0]) / d[0]);
    }
    let t = stops
        .into_iter()
        .filter(|t| *t > 0.0)
        .fold(f64::INFINITY, f64::min)
        .min(within(from, d, reach));
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
