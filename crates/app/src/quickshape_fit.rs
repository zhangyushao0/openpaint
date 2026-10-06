//! Which shape a held stroke resembles (QuickShape, S42). The drawn path is taken at evenly
//! spaced points (so neither a fast pen's sparse samples nor the samples piling up while the
//! pen holds still weigh more than their length), then:
//!
//! - open (its end far from its start): a line when no point strays from the chord between its
//!   ends by more than `LINE` of the shape's size and the path is barely longer than the chord;
//!   else the better fitting of a circle's arc (least squares, turning `ARC_SWEEP`) and a
//!   polyline of 2 to `MAX_SIDES` straight sides meeting at corners;
//! - closed (its end comes back near its start, enclosing an area): the better fitting of an
//!   ellipse (from the enclosed area's moments; a circle when its axes are within `ROUND`) and a
//!   polygon of 3 to `MAX_SIDES` corners.
//!
//! A fit counts when the points lie within `FIT` of it on average (root mean square) and
//! `FIT_MAX` at worst, as shares of the shape's size (its bounding box's diagonal); nothing
//! fitting leaves the stroke as drawn. Corners are where the path turns at least `CORNER_TURN`
//! between straight stretches; each side is the best line through its middle part, and the
//! corners are where neighbouring sides meet.

use std::f32::consts::{PI, TAU};

use crate::quickshape::{Point, Shape, dot, lengths, norm, sub};

/// Points the path is taken at.
const EVEN: usize = 96;
/// A stroke shorter than this many `still`s (the hold's radius) snaps to nothing.
const MIN_LENGTH: f32 = 6.0;
/// The end within this share of the size from the start closes the shape.
const CLOSE: f32 = 0.2;
/// A closed shape encloses at least this share of its size squared.
const MIN_AREA: f32 = 0.03;
const LINE: f32 = 0.05;
/// A line's path is at most this much longer than its chord.
const LINE_PATH: f32 = 1.15;
const FIT: f32 = 0.025;
const FIT_MAX: f32 = 0.08;
/// An arc turns at least and at most this much, in radians.
const ARC_SWEEP: [f32; 2] = [25.0 * PI / 180.0, 330.0 * PI / 180.0];
/// An ellipse whose short axis is at least this share of its long one is a circle.
const ROUND: f32 = 0.9;
/// Simplification tolerance for finding corners, as a share of the size.
const SIMPLIFY: f32 = 0.035;
const CORNER_TURN: f32 = 30.0 * PI / 180.0;
const MAX_SIDES: usize = 6;
/// No side shorter than this share of the size.
const MIN_SIDE: f32 = 0.08;

/// The shape the path `points` (canvas pixels, as drawn) resembles, if any; `still` is the
/// hold's radius in canvas pixels, the scale of the hand at this zoom.
pub(crate) fn recognise(points: &[Point], still: f32) -> Option<Shape> {
    // The samples of the pen resting at the end count as its last point.
    let last = *points.last()?;
    let resting = points
        .iter()
        .rev()
        .take_while(|p| norm(sub(**p, last)) <= still / 2.0)
        .count();
    let mut path = points[..points.len() - resting].to_vec();
    path.push(last);
    let total = *lengths(&path).last()?;
    if total < MIN_LENGTH * still || path.len() < 2 {
        return None;
    }
    let even = evenly(&path, EVEN);
    let size = diagonal(&even);
    if let Some(ring) = closed_ring(&even, size) {
        let ellipse = fit_ellipse(&ring, size);
        let polygon = fit_polyline(&ring, true, size);
        return better(ellipse, polygon);
    }
    let (first, last) = (even[0], even[even.len() - 1]);
    let chord = norm(sub(last, first));
    let straight = even
        .iter()
        .all(|p| distance_to_line(*p, first, last) <= LINE * size);
    if chord > 0.0 && straight && total <= LINE_PATH * chord {
        return Some(Shape::Line([points[0], last]));
    }
    better(fit_arc(&even, size), fit_polyline(&even, false, size))
}

/// The one of two fits lying closer to the points (`(shape, rms)`).
fn better(a: Option<(Shape, f32)>, b: Option<(Shape, f32)>) -> Option<Shape> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if a.1 <= b.1 { a.0 } else { b.0 }),
        (a, b) => a.or(b).map(|fit| fit.0),
    }
}

/// `points` at `n` places evenly spaced along them, the ends included.
pub(crate) fn evenly(points: &[Point], n: usize) -> Vec<Point> {
    let at = lengths(points);
    let total = at.last().copied().unwrap_or(0.0);
    (0..n)
        .map(|k| {
            let target = total * k as f32 / (n - 1) as f32;
            let next = at.partition_point(|x| *x < target).min(points.len() - 1);
            let before = next.saturating_sub(1);
            let span = at[next] - at[before];
            let t = if span > 0.0 {
                (target - at[before]) / span
            } else {
                1.0
            };
            let (a, b) = (points[before], points[next]);
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
        })
        .collect()
}

fn diagonal(points: &[Point]) -> f32 {
    let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
    for p in points {
        for axis in 0..2 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    norm(sub(max, min)).max(f32::MIN_POSITIVE)
}

/// The path as a ring when it comes back near its start (cut where it comes nearest, so a loop
/// drawn past its start counts once) and encloses an area; taken evenly again.
fn closed_ring(even: &[Point], size: f32) -> Option<Vec<Point>> {
    let from = even.len() * 3 / 5;
    let (end, gap) = (from..even.len())
        .map(|k| (k, norm(sub(even[k], even[0]))))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    if gap > CLOSE * size {
        return None;
    }
    let mut ring = even[..=end].to_vec();
    ring.push(even[0]);
    let ring = evenly(&ring, EVEN);
    let area = moments(&ring)[0];
    (area.abs() >= MIN_AREA * size * size).then_some(ring)
}

/// The enclosed area's signed area, centroid and second central moments of the closed polygon
/// `ring` (its last point back on its first): `[area, cx, cy, xx, xy, yy]`, the moments per
/// unit area (Green's theorem).
fn moments(ring: &[Point]) -> [f32; 6] {
    let n = ring.len() as f64;
    let mean = ring.iter().fold([0.0f64; 2], |s, p| {
        [s[0] + f64::from(p[0]) / n, s[1] + f64::from(p[1]) / n]
    });
    let (mut a, mut cx, mut cy, mut xx, mut xy, mut yy) = (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    for pair in ring.windows(2) {
        let [x0, y0] = [
            f64::from(pair[0][0]) - mean[0],
            f64::from(pair[0][1]) - mean[1],
        ];
        let [x1, y1] = [
            f64::from(pair[1][0]) - mean[0],
            f64::from(pair[1][1]) - mean[1],
        ];
        let cross = x0 * y1 - x1 * y0;
        a += cross / 2.0;
        cx += (x0 + x1) * cross / 6.0;
        cy += (y0 + y1) * cross / 6.0;
        xx += (x0 * x0 + x0 * x1 + x1 * x1) * cross / 12.0;
        yy += (y0 * y0 + y0 * y1 + y1 * y1) * cross / 12.0;
        xy += (x0 * y1 + 2.0 * x0 * y0 + 2.0 * x1 * y1 + x1 * y0) * cross / 24.0;
    }
    if a.abs() < 1e-9 {
        return [0.0; 6];
    }
    let (cx, cy) = (cx / a, cy / a);
    [
        a as f32,
        (cx + mean[0]) as f32,
        (cy + mean[1]) as f32,
        (xx / a - cx * cx) as f32,
        (xy / a - cx * cy) as f32,
        (yy / a - cy * cy) as f32,
    ]
}

/// The ellipse with the ring's area moments (a filled ellipse's are a quarter of each semi-axis
/// squared along it), drawn from where the ring starts, the way it turns; and how far the ring
/// lies from it (root mean square), if it lies close enough.
fn fit_ellipse(ring: &[Point], size: f32) -> Option<(Shape, f32)> {
    let [area, cx, cy, xx, xy, yy] = moments(ring);
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    let (half, mid) = (((xx - yy) / 2.0).hypot(xy), (xx + yy) / 2.0);
    let (major, minor) = (
        2.0 * (mid + half).max(0.0).sqrt(),
        2.0 * (mid - half).max(0.0).sqrt(),
    );
    if minor < 1.0 {
        return None;
    }
    let (major, minor) = if minor >= ROUND * major {
        let round = (major + minor) / 2.0;
        (round, round)
    } else {
        (major, minor)
    };
    let (sin, cos) = angle.sin_cos();
    let centre = [cx, cy];
    let local = |p: Point| {
        let d = sub(p, centre);
        [d[0] * cos + d[1] * sin, -d[0] * sin + d[1] * cos]
    };
    let off = |p: Point| {
        let [u, v] = local(p);
        let r = u.hypot(v);
        let scaled = (u / major).hypot(v / minor);
        if scaled > 0.0 {
            (r - r / scaled).abs()
        } else {
            minor
        }
    };
    let rms = accept(ring.iter().map(|p| off(*p)), size)?;
    let [u, v] = local(ring[0]);
    let shape = Shape::Ellipse {
        centre,
        axes: [[major * cos, major * sin], [-minor * sin, minor * cos]],
        start: (v / minor).atan2(u / major),
        sense: area.signum(),
    };
    Some((shape, rms))
}

/// The root mean square of `offsets` when it and the largest stay within `FIT` and `FIT_MAX`
/// of `size`.
fn accept(offsets: impl Iterator<Item = f32>, size: f32) -> Option<f32> {
    let (mut sum, mut count, mut worst) = (0.0f32, 0usize, 0.0f32);
    for off in offsets {
        sum += off * off;
        count += 1;
        worst = worst.max(off);
    }
    let rms = (sum / count.max(1) as f32).sqrt();
    (rms <= FIT * size && worst <= FIT_MAX * size).then_some(rms)
}

/// The circle nearest `points` (Kasa's least squares) and the arc of it they turn along, from
/// the first point to the last; `None` unless it fits and turns within `ARC_SWEEP` one way.
fn fit_arc(points: &[Point], size: f32) -> Option<(Shape, f32)> {
    let n = points.len() as f64;
    let mean = points.iter().fold([0.0f64; 2], |s, p| {
        [s[0] + f64::from(p[0]) / n, s[1] + f64::from(p[1]) / n]
    });
    let mut m = [[0.0f64; 3]; 3];
    let mut rhs = [0.0f64; 3];
    for p in points {
        let (x, y) = (f64::from(p[0]) - mean[0], f64::from(p[1]) - mean[1]);
        let row = [x, y, 1.0];
        let z = x * x + y * y;
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += row[i] * row[j];
            }
            rhs[i] -= row[i] * z;
        }
    }
    let [d, e, f] = solve(m, rhs)?;
    let (cx, cy) = (-d / 2.0, -e / 2.0);
    let radius = (cx * cx + cy * cy - f).sqrt();
    if !radius.is_finite() || radius > 4.0 * f64::from(size) {
        return None;
    }
    let centre = [(cx + mean[0]) as f32, (cy + mean[1]) as f32];
    let radius = radius as f32;
    let rms = accept(
        points
            .iter()
            .map(|p| (norm(sub(*p, centre)) - radius).abs()),
        size,
    )?;
    let angle = |p: Point| (p[1] - centre[1]).atan2(p[0] - centre[0]);
    let (mut sweep, mut travel) = (0.0f32, 0.0f32);
    for pair in points.windows(2) {
        let step = (angle(pair[1]) - angle(pair[0]) + PI).rem_euclid(TAU) - PI;
        sweep += step;
        travel += step.abs();
    }
    let turns_one_way = travel <= 1.2 * sweep.abs();
    if !turns_one_way || !(ARC_SWEEP[0]..=ARC_SWEEP[1]).contains(&sweep.abs()) {
        return None;
    }
    let on = |a: f32| [centre[0] + radius * a.cos(), centre[1] + radius * a.sin()];
    let start = angle(points[0]);
    let shape = Shape::Arc([on(start), on(start + sweep / 2.0), on(start + sweep)]);
    Some((shape, rms))
}

/// Solves `m x = rhs` (Cramer's rule); `None` when `m` is singular.
fn solve(m: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let det = |a: [[f64; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let whole = det(m);
    if whole.abs() < 1e-12 {
        return None;
    }
    let column = |c: usize| {
        let mut a = m;
        for (row, value) in a.iter_mut().zip(rhs) {
            row[c] = value;
        }
        det(a) / whole
    };
    Some([column(0), column(1), column(2)])
}

fn distance_to_line(p: Point, a: Point, b: Point) -> f32 {
    let ab = sub(b, a);
    let length = norm(ab);
    if length <= f32::EPSILON {
        return norm(sub(p, a));
    }
    let t = (dot(sub(p, a), ab) / (length * length)).clamp(0.0, 1.0);
    norm(sub(p, [a[0] + ab[0] * t, a[1] + ab[1] * t]))
}

#[path = "quickshape_corners.rs"]
mod corners;
use corners::fit_polyline;

#[cfg(test)]
#[path = "quickshape_tests.rs"]
mod tests;
