//! Compare for free boxes (S84, `perspective_check.rs`): boxes drawn freely on a page with a
//! horizon, with no answer but the rules. The edges found are gathered into boxes by touching one
//! another; a box's verticals (near the page's vertical) should stay vertical, its other edges
//! are run on to the horizon and split into its two directions where their crossings, seen from
//! the box, lie widely apart; each direction meets the horizon where the edges that agree pass
//! nearest, its vanishing point, and its edges either head there or miss it (S97: only one that
//! misses drawn on, past the point). A front corner (where an edge of each direction leaves
//! towards its meeting point) under 90 degrees is marked: V大 calls it distortion, the vanishing
//! points too close. On a three-point page (S142) the uprights head for the page's third point
//! instead (`third.rs`).

use crate::geometry::edges::Edge;
use crate::geometry::heading::{Checked, ON_ANGLE, checked, line_angle, sub};
use crate::geometry::perspective::{Direction, FAR, angle_between};
use crate::kinds::free_boxes::third::{Uprights, checked_upright, upright};
use crate::maths::path::{Point, apart, distance};

/// On a free page an edge within this angle of the vertical is a vertical (degrees).
const VERTICAL: f64 = 12.0;
/// Edges this close touch: one box's.
const TOUCH: f64 = 0.03;
/// A box's two horizontal directions lie apart where their crossings, seen from the box, leave
/// a gap of at least this (radians, about 25 degrees).
const DIRECTIONS_APART: f64 = 0.44;
/// A front corner narrower than this (degrees) is distortion.
pub const RIGHT_ANGLE: f64 = 90.0;

/// Free boxes on a page, as checked against its horizon, and on a three-point page its third
/// point (given, or found from the uprights drawn).
#[derive(Debug, Clone, PartialEq)]
pub struct FreeCheck {
    pub boxes: Vec<FreeBox>,
    pub third: Option<Point>,
}

/// One box drawn freely: its edges checked, where each of its directions meets the horizon, the
/// directions whose edges miss that point, and its front corners under 90 degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeBox {
    pub edges: Vec<Checked>,
    pub meets: Vec<(Direction, Point)>,
    pub apart: Vec<Direction>,
    pub narrow: Vec<Corner>,
}

/// A front corner: where it lies, the unit directions of its two edges from it, its angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corner {
    pub at: Point,
    pub legs: [Point; 2],
    pub angle: f64,
}

impl FreeCheck {
    /// The edges `found` on a page whose horizon lies at `horizon`, gathered into boxes.
    #[must_use]
    pub fn of(horizon: f64, found: &[Edge]) -> Self {
        Self::on_page(horizon, Uprights::Plumb, found)
    }

    /// The edges `found` on a page whose horizon lies at `horizon` and whose uprights head as
    /// `uprights` says, gathered into boxes.
    #[must_use]
    pub fn on_page(horizon: f64, uprights: Uprights, found: &[Edge]) -> Self {
        let third = uprights.third(horizon, found);
        let boxes = touching(found)
            .into_iter()
            .map(|edges| free_box((horizon, third), &edges))
            .filter(|checked| !checked.edges.is_empty())
            .collect();
        Self { boxes, third }
    }

    /// How many edges miss where their direction meets (or lean, a vertical).
    #[must_use]
    pub fn missed(&self) -> usize {
        (self.boxes.iter().flat_map(|free| &free.edges))
            .filter(|edge| !edge.on)
            .count()
    }

    /// How many front corners are under 90 degrees.
    #[must_use]
    pub fn narrow(&self) -> usize {
        self.boxes.iter().map(|free| free.narrow.len()).sum()
    }
}

/// `found` gathered into groups of edges touching one another, left to right.
fn touching(found: &[Edge]) -> Vec<Vec<Edge>> {
    let mut group: Vec<usize> = (0..found.len()).collect();
    fn root(group: &mut [usize], mut i: usize) -> usize {
        while group[i] != i {
            group[i] = group[group[i]];
            i = group[i];
        }
        i
    }
    for i in 0..found.len() {
        for j in i + 1..found.len() {
            if apart(&found[i].ends, &found[j].ends) <= TOUCH {
                let (a, b) = (root(&mut group, i), root(&mut group, j));
                group[a.max(b)] = a.min(b);
            }
        }
    }
    let mut boxes: Vec<(usize, Vec<Edge>)> = Vec::new();
    for (i, edge) in found.iter().enumerate() {
        let at = root(&mut group, i);
        match boxes.iter_mut().find(|(r, _)| *r == at) {
            Some((_, edges)) => edges.push(*edge),
            None => boxes.push((at, vec![*edge])),
        }
    }
    let left = |edges: &Vec<Edge>| {
        (edges.iter()).fold(f64::INFINITY, |l, e| l.min(e.ends[0][0]).min(e.ends[1][0]))
    };
    boxes.sort_by(|a, b| left(&a.1).total_cmp(&left(&b.1)));
    boxes.into_iter().map(|(_, edges)| edges).collect()
}

/// One box's `edges` checked against the `horizon` and the page's `third` point, if any.
fn free_box((horizon, third): (f64, Option<Point>), edges: &[Edge]) -> FreeBox {
    let centre = mean(&edges.iter().map(Edge::middle).collect::<Vec<_>>());
    let mut read: Vec<Checked> = Vec::new();
    let mut receding: Vec<(Edge, f64)> = Vec::new();
    for edge in edges {
        match third {
            Some(point) if upright(edge, point) => read.push(checked_upright(edge, point)),
            None if line_angle(edge.direction(), [0.0, 1.0]) <= VERTICAL => {
                read.push(checked(edge, Direction::Vertical, None, FAR));
            }
            _ => receding.extend(crossing(edge, horizon).map(|x| (*edge, x))),
        }
    }
    let groups = directions(&receding, centre, horizon);
    let mut meets = Vec::new();
    let mut apart = Vec::new();
    for (direction, group) in &groups {
        let meet = meeting(group, horizon);
        meets.push((*direction, meet));
        let mut missed = false;
        for (edge, _) in group {
            let edge = checked(edge, *direction, Some(meet), FAR);
            missed |= !edge.on;
            read.push(edge);
        }
        if missed && group.len() > 1 {
            apart.push(*direction);
        }
    }
    let narrow = front_corners(&groups, &meets);
    FreeBox {
        edges: read,
        meets,
        apart,
        narrow,
    }
}

/// Where `edge`'s line meets the `horizon` (its x), within reach of the page; none for an edge
/// that runs along it.
fn crossing(edge: &Edge, horizon: f64) -> Option<f64> {
    let d = edge.direction();
    if d[1].abs() < 1e-9 {
        return None;
    }
    let m = edge.middle();
    let x = m[0] + (horizon - m[1]) * d[0] / d[1];
    ((x - 0.5).abs() <= FAR).then_some(x)
}

/// A box's receding edges (each with its crossing) split into its two directions: apart where,
/// seen from the box's `centre`, their crossings leave the widest gap, if wide enough; one
/// direction else, named by the side its crossings lie on.
fn directions(
    receding: &[(Edge, f64)],
    centre: Point,
    horizon: f64,
) -> Vec<(Direction, Vec<(Edge, f64)>)> {
    if receding.is_empty() {
        return Vec::new();
    }
    let height = (horizon - centre[1]).abs() + 0.05;
    let seen = |x: f64| ((x - centre[0]) / height).atan();
    let mut sorted = receding.to_vec();
    sorted.sort_by(|a, b| seen(a.1).total_cmp(&seen(b.1)));
    let gap = (sorted.windows(2).enumerate())
        .map(|(at, pair)| (at + 1, seen(pair[1].1) - seen(pair[0].1)))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    match gap {
        Some((at, wide)) if wide >= DIRECTIONS_APART => {
            let right = sorted.split_off(at);
            vec![(Direction::Left, sorted), (Direction::Right, right)]
        }
        _ => {
            let middle = sorted[sorted.len() / 2].1;
            let direction = if middle < centre[0] {
                Direction::Left
            } else {
                Direction::Right
            };
            vec![(direction, sorted)]
        }
    }
}

/// Where a direction's edges (each with its crossing) meet on the `horizon`: the point their lines
/// pass nearest, in angle squared, each edge weighed by its length; while an edge misses it by
/// more than `ON_ANGLE` and more than two remain, the worst is left out and the point found again,
/// so where most edges agree one drawn off does not move it.
fn meeting(group: &[(Edge, f64)], horizon: f64) -> Point {
    let mut agreeing: Vec<&(Edge, f64)> = group.iter().collect();
    loop {
        let meet = [nearest_to_all(&agreeing, horizon), horizon];
        let off = |(edge, _): &&(Edge, f64)| line_angle(edge.direction(), sub(meet, edge.middle()));
        let worst =
            (0..agreeing.len()).max_by(|a, b| off(&agreeing[*a]).total_cmp(&off(&agreeing[*b])));
        match worst {
            Some(at) if agreeing.len() > 2 && off(&agreeing[at]) > ON_ANGLE => {
                agreeing.remove(at);
            }
            _ => return meet,
        }
    }
}

/// The point of the horizon between `edges`' crossings their lines pass nearest, in angle squared
/// weighed by length (a golden-section search: each round keeps the part holding the least).
fn nearest_to_all(edges: &[&(Edge, f64)], horizon: f64) -> f64 {
    let misses = |x: f64| -> f64 {
        (edges.iter())
            .map(|(edge, _)| {
                let angle = line_angle(edge.direction(), sub([x, horizon], edge.middle()));
                edge.length() * angle * angle
            })
            .sum()
    };
    let crossings = edges.iter().map(|(_, x)| *x);
    let mut low = crossings.clone().fold(f64::INFINITY, f64::min);
    let mut high = crossings.fold(f64::NEG_INFINITY, f64::max);
    let golden = (5f64.sqrt() - 1.0) / 2.0;
    for _ in 0..80 {
        let (a, b) = (high - golden * (high - low), low + golden * (high - low));
        if misses(a) <= misses(b) {
            high = b;
        } else {
            low = a;
        }
    }
    (low + high) / 2.0
}

/// The front corners under 90 degrees: where an edge of each direction meets the other at its
/// end farther from its meeting point (both leave the corner towards the horizon's points).
fn front_corners(
    groups: &[(Direction, Vec<(Edge, f64)>)],
    meets: &[(Direction, Point)],
) -> Vec<Corner> {
    let ([(_, left), (_, right)], [(_, to_left), (_, to_right)]) = (groups, meets) else {
        return Vec::new();
    };
    let mut corners = Vec::new();
    for (l, _) in left {
        for (r, _) in right {
            let Some(at) = lines_cross(l, r) else {
                continue;
            };
            let (Some(leg_l), Some(leg_r)) = (leg(l, at, *to_left), leg(r, at, *to_right)) else {
                continue;
            };
            let angle = angle_between(leg_l, leg_r);
            if angle < RIGHT_ANGLE {
                corners.push(Corner {
                    at,
                    legs: [leg_l, leg_r].map(unit),
                    angle,
                });
            }
        }
    }
    corners
}

/// Where the lines of `a` and `b` cross, if they are not parallel.
fn lines_cross(a: &Edge, b: &Edge) -> Option<Point> {
    let (p, r) = (a.ends[0], sub(a.ends[1], a.ends[0]));
    let (q, s) = (b.ends[0], sub(b.ends[1], b.ends[0]));
    let across = r[0] * s[1] - r[1] * s[0];
    if across.abs() < 1e-12 {
        return None;
    }
    let t = ((q[0] - p[0]) * s[1] - (q[1] - p[1]) * s[0]) / across;
    Some([p[0] + t * r[0], p[1] + t * r[1]])
}

/// `edge` leaving the corner `at` towards its meeting point `meet`: from its end at the corner
/// (within `TOUCH`) to its other end, which lies nearer `meet`; none otherwise.
fn leg(edge: &Edge, at: Point, meet: Point) -> Option<Point> {
    let [a, b] = edge.ends;
    let (near, far) = if distance(a, at) <= distance(b, at) {
        (a, b)
    } else {
        (b, a)
    };
    (distance(near, at) <= TOUCH && distance(far, meet) < distance(near, meet))
        .then(|| sub(far, at))
}

fn mean(points: &[Point]) -> Point {
    let n = points.len().max(1) as f64;
    [0, 1].map(|k| points.iter().map(|p| p[k]).sum::<f64>() / n)
}

fn unit(v: Point) -> Point {
    let long = v[0].hypot(v[1]).max(f64::MIN_POSITIVE);
    [v[0] / long, v[1] / long]
}
