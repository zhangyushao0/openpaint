//! QuickShape (Procreate's): a stroke drawn and held still at its end snaps to the shape it
//! resembles: a straight line, a circle's arc, an ellipse (a circle when nearly round), or a
//! polyline with corners (open, or closed as a triangle, a rectangle and the like). This module
//! holds the shapes, the samples a snapped stroke stores along them and their handles; the
//! recognition is `quickshape_fit.rs`.
//!
//! A snapped stroke stays a stroke: samples along the exact shape, dense where the curve needs
//! them and at corners, each with the pressure, tilt, azimuth and time the drawn stroke had at
//! the same share of its length (`along`), so its pressure taper carries over.

use std::f32::consts::TAU;

use openpaint_core::{CanvasPoint, StrokeSample};

use crate::error::AppError;

pub(crate) type Point = [f32; 2];

/// Longest step between two samples along a shape, in canvas pixels.
const STEP: f32 = 16.0;
/// Largest turn between two samples along a curve: the curve through them keeps to the shape
/// within a small fraction of a pixel.
const TURN_STEP: f32 = 5.0 * TAU / 360.0;
/// Samples this far from a corner on both sides keep it sharp: the curve the renderer draws
/// through the samples rounds a corner only between its nearest samples.
const CORNER: [f32; 2] = [0.5, 2.0];

/// A shape a held stroke snaps to, in canvas pixels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Shape {
    /// From the first point to the second.
    Line([Point; 2]),
    /// The circle's arc from the first point through the second to the third.
    Arc([Point; 3]),
    /// The ellipse around `centre` with the two perpendicular semi-axes `axes`, drawn from the
    /// angle `start` (from the first axis towards the second) all the way round, turning with
    /// `sense` (1 from the first axis towards the second, -1 back).
    Ellipse {
        centre: Point,
        axes: [Point; 2],
        start: f32,
        sense: f32,
    },
    /// Corners in drawing order, back to the first when `closed`.
    Polyline { corners: Vec<Point>, closed: bool },
}

impl Shape {
    /// The points a stroke along the shape passes, from its start to its end.
    pub(crate) fn outline(&self) -> Vec<Point> {
        match self {
            Self::Line([from, to]) => sides(&[*from, *to]),
            Self::Arc(points) => match circle_through(*points) {
                Some((centre, radius, start, sweep)) => arc(centre, radius, start, sweep),
                None => sides(points),
            },
            Self::Ellipse {
                centre,
                axes,
                start,
                sense,
            } => ellipse(*centre, *axes, *start, *sense),
            Self::Polyline { corners, closed } => {
                let mut path = corners.clone();
                if *closed {
                    path.extend(corners.first());
                }
                sides(&path)
            }
        }
    }

    /// Where the shape's handles are: a line's ends; an arc's ends and middle; an ellipse's
    /// four axis ends; a polyline's corners.
    pub(crate) fn handles(&self) -> Vec<Point> {
        match self {
            Self::Line(ends) => ends.to_vec(),
            Self::Arc(points) => points.to_vec(),
            Self::Ellipse { centre, axes, .. } => {
                let [u, v] = *axes;
                let [x, y] = *centre;
                vec![
                    [x + u[0], y + u[1]],
                    [x + v[0], y + v[1]],
                    [x - u[0], y - u[1]],
                    [x - v[0], y - v[1]],
                ]
            }
            Self::Polyline { corners, .. } => corners.clone(),
        }
    }

    /// The shape with handle `index` dragged to `to`. An ellipse's axis end turns that axis to
    /// point at `to` and sets its length; the other axis keeps its length, upright to it.
    pub(crate) fn moved(&self, index: usize, to: Point) -> Self {
        let mut shape = self.clone();
        match &mut shape {
            Self::Line(points) => set(&mut points[..], index, to),
            Self::Arc(points) => set(&mut points[..], index, to),
            Self::Polyline { corners, .. } => set(corners, index, to),
            Self::Ellipse { centre, axes, .. } => {
                let sign = if index >= 2 { -1.0 } else { 1.0 };
                let axis = [(to[0] - centre[0]) * sign, (to[1] - centre[1]) * sign];
                let length = norm(axis);
                if length < 1.0 || index > 3 {
                    return shape;
                }
                let (j, other) = (index % 2, 1 - index % 2);
                let keep = norm(axes[other]);
                let unit = [axis[0] / length, axis[1] / length];
                // The second axis lies a quarter turn on from the first.
                axes[other] = if j == 0 {
                    [-unit[1] * keep, unit[0] * keep]
                } else {
                    [unit[1] * keep, -unit[0] * keep]
                };
                axes[j] = axis;
            }
        }
        shape
    }

    /// The shape turned and scaled about its anchor so that `held` (where the pen held still
    /// when it snapped) lands on `pen`: still pressing, the hand turns and sizes the shape
    /// (Procreate). The anchor is an open shape's start and a closed one's middle.
    pub(crate) fn dragged(&self, held: Point, pen: Point) -> Self {
        let anchor = self.anchor();
        let (from, to) = (sub(held, anchor), sub(pen, anchor));
        let length = norm(from);
        if length < 1.0 {
            return self.clone();
        }
        // The similarity z -> anchor + k (z - anchor), k = to / from as complex numbers.
        let k = [
            (to[0] * from[0] + to[1] * from[1]) / (length * length),
            (to[1] * from[0] - to[0] * from[1]) / (length * length),
        ];
        let map = |p: Point| {
            let d = sub(p, anchor);
            [
                anchor[0] + k[0] * d[0] - k[1] * d[1],
                anchor[1] + k[1] * d[0] + k[0] * d[1],
            ]
        };
        let linear = |v: Point| [k[0] * v[0] - k[1] * v[1], k[1] * v[0] + k[0] * v[1]];
        match self {
            Self::Line(points) => Self::Line(points.map(map)),
            Self::Arc(points) => Self::Arc(points.map(map)),
            Self::Polyline { corners, closed } => Self::Polyline {
                corners: corners.iter().map(|corner| map(*corner)).collect(),
                closed: *closed,
            },
            Self::Ellipse {
                centre,
                axes,
                start,
                sense,
            } => Self::Ellipse {
                centre: *centre,
                axes: axes.map(linear),
                start: *start,
                sense: *sense,
            },
        }
    }

    fn anchor(&self) -> Point {
        match self {
            Self::Line([from, _]) | Self::Arc([from, _, _]) => *from,
            Self::Ellipse { centre, .. } => *centre,
            Self::Polyline { corners, closed } => {
                if *closed {
                    let n = corners.len().max(1) as f32;
                    let sum = corners
                        .iter()
                        .fold([0.0, 0.0], |s, c| [s[0] + c[0], s[1] + c[1]]);
                    [sum[0] / n, sum[1] / n]
                } else {
                    corners.first().copied().unwrap_or([0.0, 0.0])
                }
            }
        }
    }
}

fn set(points: &mut [Point], index: usize, to: Point) {
    if let Some(point) = points.get_mut(index) {
        *point = to;
    }
}

/// Samples along `outline` carrying what the drawn stroke `drawn` had at the same share of its
/// length: pressure, tilt and time interpolated, the azimuth of the nearer sample.
///
/// # Errors
/// `Core` when a point leaves the coordinates a canvas point may have.
pub(crate) fn along(
    outline: &[Point],
    drawn: &[StrokeSample],
) -> Result<Vec<StrokeSample>, AppError> {
    let drawn_points: Vec<Point> = drawn
        .iter()
        .map(|sample| [sample.position().x(), sample.position().y()])
        .collect();
    let drawn_lengths = lengths(&drawn_points);
    let total = drawn_lengths.last().copied().unwrap_or(0.0);
    let shape_lengths = lengths(outline);
    let shape_total = shape_lengths
        .last()
        .copied()
        .unwrap_or(0.0)
        .max(f32::MIN_POSITIVE);
    outline
        .iter()
        .zip(&shape_lengths)
        .map(|(point, length)| {
            let target = length / shape_total * total;
            let next = drawn_lengths
                .partition_point(|at| *at < target)
                .min(drawn.len() - 1);
            let before = next.saturating_sub(1);
            let span = drawn_lengths[next] - drawn_lengths[before];
            let t = if span > 0.0 {
                ((target - drawn_lengths[before]) / span).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let (a, b) = (&drawn[before], &drawn[next]);
            let lerp = |x: f32, y: f32| x + (y - x) * t;
            let time = lerp(a.time_ms() as f32, b.time_ms() as f32).round() as u32;
            let azimuth = if t < 0.5 { a.azimuth() } else { b.azimuth() };
            Ok(StrokeSample::new(
                CanvasPoint::new(point[0], point[1])?,
                lerp(a.pressure(), b.pressure()),
                lerp(a.tilt(), b.tilt()),
                azimuth,
                time.clamp(a.time_ms(), b.time_ms()),
            )?)
        })
        .collect()
}

/// The length along `points` up to each point.
pub(crate) fn lengths(points: &[Point]) -> Vec<f32> {
    let mut total = 0.0;
    let mut out = Vec::with_capacity(points.len());
    let mut last = points.first().copied().unwrap_or([0.0, 0.0]);
    for point in points {
        total += norm(sub(*point, last));
        out.push(total);
        last = *point;
    }
    out
}

/// The straight sides through `path`: at most `STEP` apart, and close on both sides of every
/// inner corner (`CORNER`).
fn sides(path: &[Point]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for (index, pair) in path.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let length = norm(sub(b, a));
        if length <= f32::EPSILON {
            continue;
        }
        let mut at: Vec<f32> = vec![0.0];
        let corner_after = index + 2 < path.len();
        if index > 0 {
            at.extend(CORNER.iter().filter(|d| **d * 2.0 < length).copied());
        }
        let steps = (length / STEP).ceil().max(1.0);
        at.extend((1..steps as u32).map(|k| k as f32 * length / steps));
        if corner_after {
            at.extend(
                CORNER
                    .iter()
                    .rev()
                    .filter(|d| **d * 2.0 < length)
                    .map(|d| length - d),
            );
        }
        at.sort_by(f32::total_cmp);
        at.dedup_by(|x, y| (*x - *y).abs() < 0.25);
        out.extend(at.iter().map(|d| {
            let t = d / length;
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
        }));
    }
    out.extend(path.last());
    out.dedup();
    out
}

/// How many pieces a curve of `radius` turning `sweep` radians is sampled in.
fn steps(radius: f32, sweep: f32) -> u32 {
    let turn = TURN_STEP.min(STEP / radius.max(1.0));
    (sweep.abs() / turn).ceil().max(2.0) as u32
}

fn arc(centre: Point, radius: f32, start: f32, sweep: f32) -> Vec<Point> {
    let n = steps(radius, sweep);
    (0..=n)
        .map(|k| {
            let angle = start + sweep * k as f32 / n as f32;
            [
                centre[0] + radius * angle.cos(),
                centre[1] + radius * angle.sin(),
            ]
        })
        .collect()
}

fn ellipse(centre: Point, [u, v]: [Point; 2], start: f32, sense: f32) -> Vec<Point> {
    let n = steps(norm(u).max(norm(v)), TAU);
    (0..=n)
        .map(|k| {
            let t = start + sense * TAU * k as f32 / n as f32;
            let (sin, cos) = if k == n { start.sin_cos() } else { t.sin_cos() };
            [
                centre[0] + u[0] * cos + v[0] * sin,
                centre[1] + u[1] * cos + v[1] * sin,
            ]
        })
        .collect()
}

/// The circle through `[start, through, end]` and the arc on it from `start` through
/// `through` to `end`: centre, radius, start angle and signed sweep. `None` when the points lie
/// (almost) on one line.
pub(crate) fn circle_through([a, b, c]: [Point; 3]) -> Option<(Point, f32, f32, f32)> {
    let (ab, ac) = (sub(b, a), sub(c, a));
    let d = 2.0 * (ab[0] * ac[1] - ab[1] * ac[0]);
    let (sab, sac) = (dot(ab, ab), dot(ac, ac));
    if d.abs() < 1e-3 * sab.max(sac) {
        return None;
    }
    let ux = (ac[1] * sab - ab[1] * sac) / d;
    let uy = (ab[0] * sac - ac[0] * sab) / d;
    let centre = [a[0] + ux, a[1] + uy];
    let radius = ux.hypot(uy);
    let angle = |p: Point| (p[1] - centre[1]).atan2(p[0] - centre[0]);
    let start = angle(a);
    let turn = |p: Point| (angle(p) - start).rem_euclid(TAU);
    let (to_end, to_through) = (turn(c), turn(b));
    let sweep = if to_through <= to_end {
        to_end
    } else {
        to_end - TAU
    };
    Some((centre, radius, start, sweep))
}

pub(crate) fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

pub(crate) fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

pub(crate) fn norm(a: Point) -> f32 {
    a[0].hypot(a[1])
}
