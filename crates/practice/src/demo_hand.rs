//! How a demonstration's hand marks a line (`demo.rs`): a short tick across it, and a copy of it
//! that strays a little as a hand's does.

use crate::path::{Point, distance, length};

/// A short tick across the point `at` of a line heading `towards`, `half` long either side.
pub(crate) fn tick(at: Point, towards: Point, half: f64) -> Vec<Point> {
    let d = [towards[0] - at[0], towards[1] - at[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    let n = [-d[1] / long * half, d[0] / long * half];
    vec![[at[0] - n[0], at[1] - n[1]], [at[0] + n[0], at[1] + n[1]]]
}

/// `points` as a hand copies them: moved off sideways by up to `amount` (frame units) in one
/// smooth swell along the line, the ends near their places.
pub(crate) fn wobbled(points: &[Point], amount: f64, phase: f64) -> Vec<Point> {
    let total = length(points).max(f64::MIN_POSITIVE);
    let mut walked = 0.0;
    let mut out = Vec::with_capacity(points.len());
    for (index, p) in points.iter().enumerate() {
        if index > 0 {
            walked += distance(points[index - 1], *p);
        }
        let (before, after) = (
            points[index.saturating_sub(1)],
            points[(index + 1).min(points.len() - 1)],
        );
        let d = [after[0] - before[0], after[1] - before[1]];
        let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
        let s = walked / total;
        let off = amount * (std::f64::consts::PI * s).sin() * (phase + 2.0 * s).cos();
        out.push([p[0] - d[1] / long * off, p[1] + d[0] / long * off]);
    }
    out
}
