//! Polylines in a cell's units (its side is 1): length, even resampling, nearest points, centre
//! and spread.

/// A point, `[x, y]`.
pub type Point = [f64; 2];

#[must_use]
pub fn distance(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The length along `points`.
#[must_use]
pub fn length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| distance(pair[0], pair[1]))
        .sum()
}

/// `points` at even steps of `spacing` along their path, the first and last kept (the last step
/// shorter); one point for a path of no length.
#[must_use]
pub fn resample(points: &[Point], spacing: f64) -> Vec<Point> {
    let Some(&first) = points.first() else {
        return Vec::new();
    };
    let total = length(points);
    if total <= 0.0 || spacing <= 0.0 {
        return vec![first];
    }
    let mut out = vec![first];
    let mut next = spacing;
    let mut along = 0.0;
    for pair in points.windows(2) {
        let step = distance(pair[0], pair[1]);
        while step > 0.0 && next <= along + step && next < total {
            let t = (next - along) / step;
            out.push(lerp(pair[0], pair[1], t));
            next += spacing;
        }
        along += step;
    }
    if let Some(&last) = points.last() {
        out.push(last);
    }
    out
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// The point of the segment from `a` to `b` nearest `p`.
fn nearest_on_segment(a: Point, b: Point, p: Point) -> Point {
    let d = [b[0] - a[0], b[1] - a[1]];
    let squared = d[0] * d[0] + d[1] * d[1];
    if squared <= 0.0 {
        return a;
    }
    let t = ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / squared;
    lerp(a, b, t.clamp(0.0, 1.0))
}

/// The point of `lines` (polylines; a lone point counts) nearest `p`, and its distance; `None`
/// when `lines` holds no point.
#[must_use]
pub fn nearest(lines: &[&[Point]], p: Point) -> Option<(Point, f64)> {
    let mut best: Option<(Point, f64)> = None;
    let mut consider = |q: Point| {
        let d = distance(p, q);
        if best.is_none_or(|(_, b)| d < b) {
            best = Some((q, d));
        }
    };
    for line in lines {
        match line {
            [] => {}
            [only] => consider(*only),
            _ => {
                for pair in line.windows(2) {
                    consider(nearest_on_segment(pair[0], pair[1], p));
                }
            }
        }
    }
    best
}

/// The centre of `lines` taken along their length (every bit of line weighing as much as its
/// length) and their root mean square distance from it, likewise; `None` for no length. Exact
/// for polylines, so a moved and sized copy of a line has the moved and sized centre and spread.
#[must_use]
pub fn centre_and_spread(lines: &[&[Point]]) -> Option<(Point, f64)> {
    let segments = || {
        lines
            .iter()
            .flat_map(|line| line.windows(2))
            .map(|pair| (pair[0], pair[1], distance(pair[0], pair[1])))
    };
    let total: f64 = segments().map(|(_, _, l)| l).sum();
    if total <= 0.0 {
        return None;
    }
    let mut centre = [0.0, 0.0];
    for (a, b, l) in segments() {
        centre[0] += l * (a[0] + b[0]) / 2.0;
        centre[1] += l * (a[1] + b[1]) / 2.0;
    }
    let centre = [centre[0] / total, centre[1] / total];
    // Over a segment of length l with middle m: l (|m - c|^2 + l^2 / 12).
    let squares: f64 = segments()
        .map(|(a, b, l)| {
            let m = lerp(a, b, 0.5);
            l * ((m[0] - centre[0]).powi(2) + (m[1] - centre[1]).powi(2) + l * l / 12.0)
        })
        .sum();
    Some((centre, (squares / total).sqrt()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_steps_evenly_and_keeps_both_ends() {
        let line = [[0.0, 0.0], [1.0, 0.0], [1.0, 0.5]];
        let even = resample(&line, 0.25);
        assert_eq!(even.len(), 7, "{even:?}");
        assert_eq!(even[0], [0.0, 0.0]);
        assert_eq!(even[4], [1.0, 0.0]);
        assert_eq!(*even.last().expect("last"), [1.0, 0.5]);
        assert_eq!(resample(&[[0.3, 0.3], [0.3, 0.3]], 0.1), vec![[0.3, 0.3]]);
    }

    #[test]
    fn nearest_points_lie_on_segments_and_lone_points() {
        let line: &[Point] = &[[0.0, 0.0], [1.0, 0.0]];
        let dot: &[Point] = &[[0.5, 2.0]];
        let (at, d) = nearest(&[line, dot], [0.25, 0.5]).expect("nearest");
        assert_eq!((at, d), ([0.25, 0.0], 0.5));
        assert_eq!(nearest(&[], [0.0, 0.0]), None);
        assert_eq!(nearest(&[line, dot], [0.5, 1.75]), Some(([0.5, 2.0], 0.25)));
    }

    #[test]
    fn centre_and_spread_move_and_size_with_the_line() {
        let line: &[Point] = &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        let moved: Vec<Point> = line
            .iter()
            .map(|p| [2.0 * p[0] + 1.0, 2.0 * p[1] - 3.0])
            .collect();
        let (centre, spread) = centre_and_spread(&[line]).expect("a line");
        let (moved_centre, moved_spread) = centre_and_spread(&[&moved]).expect("a line");
        assert!(distance(moved_centre, [2.0 * centre[0] + 1.0, 2.0 * centre[1] - 3.0]) < 1e-12);
        assert!((moved_spread - 2.0 * spread).abs() < 1e-12);
        assert_eq!(centre_and_spread(&[&[[0.5, 0.5]]]), None, "no length");
    }
}
