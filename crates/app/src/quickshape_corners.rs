//! Polylines and polygons (QuickShape, S42): corners are found by simplifying the path
//! (Douglas-Peucker within `SIMPLIFY` of its size), keeping only places that turn at least
//! `CORNER_TURN`; each side is then the best line through the middle of its stretch of the path
//! (total least squares, its ends left out where the hand rounds the corners), and each corner
//! is where neighbouring sides meet. An open polyline's ends lie on its first and last side
//! where the path began and ended.

use super::{CORNER_TURN, MAX_SIDES, MIN_SIDE, SIMPLIFY, accept, distance_to_line};
use crate::quickshape::{Point, Shape, dot, norm, sub};

/// Share of each side's stretch, at either end, left out of its line.
const SIDE_ENDS: f32 = 0.15;
/// Neighbouring sides closer to parallel than this (the sine of their angle) keep the corner
/// the simplification found.
const PARALLEL: f32 = 0.2;
/// The path passes this close to every corner, as a share of the size.
const REACH: f32 = 0.03;

/// The polyline (`closed`: the polygon, `points` a ring back on its first point) through the
/// corners of `points`, and how far they lie from it, if it fits.
pub(super) fn fit_polyline(points: &[Point], closed: bool, size: f32) -> Option<(Shape, f32)> {
    let mut corners = simplified(points, SIMPLIFY * size);
    if closed {
        // The ring's start is where the pen began, often along a side.
        corners.pop();
    }
    drop_flat(points, &mut corners, closed);
    let sides = if closed {
        corners.len()
    } else {
        corners.len() - 1
    };
    let fewest = if closed { 3 } else { 2 };
    if !(fewest..=MAX_SIDES).contains(&sides) {
        return None;
    }
    let mut placed = corner_points(points, &corners, closed);
    let path: Vec<Point> = if closed {
        placed.iter().chain(placed.first()).copied().collect()
    } else {
        placed.clone()
    };
    if path
        .windows(2)
        .any(|pair| norm(sub(pair[1], pair[0])) < MIN_SIDE * size)
    {
        return None;
    }
    // A drawn corner goes into the corner; a wave's sides meet far beyond its turns.
    let inner = if closed {
        &placed[..]
    } else {
        &placed[1..placed.len() - 1]
    };
    let reached = |corner: &Point| {
        points
            .iter()
            .any(|p| norm(sub(*p, *corner)) <= REACH * size)
    };
    if !inner.iter().all(reached) {
        return None;
    }
    let off = |p: &Point| {
        path.windows(2)
            .map(|pair| distance_to_line(*p, pair[0], pair[1]))
            .fold(f32::MAX, f32::min)
    };
    let rms = accept(points.iter().map(off), size)?;
    if closed {
        // Start at the corner nearest where the pen began.
        let nearest = (0..placed.len())
            .min_by(|a, b| {
                let d = |c: usize| norm(sub(placed[c], points[0]));
                d(*a).total_cmp(&d(*b))
            })
            .unwrap_or(0);
        placed.rotate_left(nearest);
    }
    let shape = Shape::Polyline {
        corners: placed,
        closed,
    };
    Some((shape, rms))
}

/// Where the corners at indices `corners` of `points` lie: where the lines of the sides on
/// either side meet, an open path's ends on their side's line.
fn corner_points(points: &[Point], corners: &[usize], closed: bool) -> Vec<Point> {
    let sides = if closed {
        corners.len()
    } else {
        corners.len() - 1
    };
    let lines: Vec<(Point, Point)> = (0..sides)
        .map(|s| {
            let to = corners[(s + 1) % corners.len()];
            side_line(&stretch(points, corners[s], to, closed))
        })
        .collect();
    (0..corners.len())
        .map(|c| {
            let before = if c == 0 {
                closed.then(|| sides - 1)
            } else {
                Some(c - 1)
            };
            let after = (c < sides).then_some(c);
            match (before, after) {
                (Some(b), Some(a)) => meet(lines[b], lines[a]).unwrap_or(points[corners[c]]),
                (None, Some(a)) => project(points[0], lines[a]),
                (Some(b), None) => project(points[points.len() - 1], lines[b]),
                (None, None) => points[corners[c]],
            }
        })
        .collect()
}

/// The indices Douglas-Peucker keeps of `points` within `tolerance`, in order, both ends kept.
fn simplified(points: &[Point], tolerance: f32) -> Vec<usize> {
    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;
    let mut stack = vec![(0, last)];
    while let Some((a, b)) = stack.pop() {
        let far = (a + 1..b)
            .map(|k| (k, distance_to_line(points[k], points[a], points[b])))
            .max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((k, distance)) = far
            && distance > tolerance
        {
            keep[k] = true;
            stack.push((a, k));
            stack.push((k, b));
        }
    }
    (0..points.len()).filter(|k| keep[*k]).collect()
}

/// Drops corners where the path turns less than `CORNER_TURN`, the flattest first, until every
/// one left turns that much (an open path keeps its ends).
fn drop_flat(points: &[Point], corners: &mut Vec<usize>, closed: bool) {
    loop {
        let n = corners.len();
        let inner = if closed { 0..n } else { 1..n.saturating_sub(1) };
        let turn = |c: usize| {
            let (before, after) = (corners[(c + n - 1) % n], corners[(c + 1) % n]);
            let (u, v) = (
                sub(points[corners[c]], points[before]),
                sub(points[after], points[corners[c]]),
            );
            let cross = u[0] * v[1] - u[1] * v[0];
            cross.atan2(dot(u, v)).abs()
        };
        let flattest = inner
            .map(|c| (c, turn(c)))
            .filter(|(_, turn)| *turn < CORNER_TURN)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match flattest {
            Some((c, _)) if n > 2 => {
                corners.remove(c);
            }
            _ => return,
        }
    }
}

/// The path's points from corner index `from` to `to` (round the ring's end when closed and
/// `to` comes first), the ends of the stretch left out.
fn stretch(points: &[Point], from: usize, to: usize, closed: bool) -> Vec<Point> {
    let all: Vec<Point> = if to >= from {
        points[from..=to].to_vec()
    } else if closed {
        // The ring's last point repeats its first.
        points[from..points.len() - 1]
            .iter()
            .chain(&points[..=to])
            .copied()
            .collect()
    } else {
        Vec::new()
    };
    let cut = (all.len() as f32 * SIDE_ENDS) as usize;
    if all.len() > 2 * cut + 2 {
        all[cut..all.len() - cut].to_vec()
    } else {
        all
    }
}

/// The line nearest `points` (total least squares): a point on it and its direction.
fn side_line(points: &[Point]) -> (Point, Point) {
    let n = points.len().max(1) as f32;
    let mean = points
        .iter()
        .fold([0.0, 0.0], |s, p| [s[0] + p[0] / n, s[1] + p[1] / n]);
    let (mut xx, mut xy, mut yy) = (0.0f32, 0.0f32, 0.0f32);
    for p in points {
        let d = sub(*p, mean);
        xx += d[0] * d[0];
        xy += d[0] * d[1];
        yy += d[1] * d[1];
    }
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    (mean, [angle.cos(), angle.sin()])
}

/// Where two lines meet, unless they are nearly parallel.
fn meet((p, d): (Point, Point), (q, e): (Point, Point)) -> Option<Point> {
    let cross = d[0] * e[1] - d[1] * e[0];
    if cross.abs() < PARALLEL {
        return None;
    }
    let w = sub(q, p);
    let s = (w[0] * e[1] - w[1] * e[0]) / cross;
    Some([p[0] + d[0] * s, p[1] + d[1] * s])
}

fn project(point: Point, (p, d): (Point, Point)) -> Point {
    let s = dot(sub(point, p), d);
    [p[0] + d[0] * s, p[1] + d[1] * s]
}
