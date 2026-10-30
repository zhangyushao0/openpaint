//! An ellipse found in a hand's lines, and Compare's words for it (S84). The fit is the direct
//! least-squares ellipse fit of Fitzgibbon, Pilu and Fisher ("Direct least square fitting of
//! ellipses", 1999) in Halíř and Flusser's numerically stable form (1998): the conic nearest the
//! points in algebraic distance under the constraint `4ac - b² = 1`, which always gives an
//! ellipse, from a partial arc too; the points read at even steps along the lines (a slow stretch
//! weighs no more than a quick one) and centred and scaled before the fit.
//!
//! The words compare the fitted ellipse with the exact one in what the eye sees: rounder or
//! flatter (its degree, `DEGREE`), tilted which way (its major axis, `TILT`, only where both are
//! clearly not round), and which side of the guide it falls short of or runs past (`REACH`).

use crate::ellipse::{Ellipse, EllipseGuide, upright};
use crate::path::{Point, resample};

/// The even step the lines are read at.
const SPACING: f64 = 0.005;
/// Degrees of difference in degree (Drawabox's: 90 a circle) before one is rounder or flatter.
pub const DEGREE: f64 = 4.0;
/// Degrees the major axes may differ before one is tilted; only while both are flatter than
/// `ROUND` (a near circle's axis says nothing).
pub const TILT: f64 = 4.0;
const ROUND: f64 = 0.9;
/// How far an ellipse may fall short of a side or run past it before it misses it: 8 pixels of a
/// 400-pixel frame.
pub const REACH: f64 = 0.02;
/// A fit thinner than this, or larger than the frame many times, is no ellipse drawn.
const THINNEST: f64 = 0.03;
const LARGEST: f64 = 2.0;

/// How a drawn ellipse's roundness compares with the exact one's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roundness {
    Rounder,
    Flatter,
}

/// Which way a drawn ellipse's major axis is turned from the exact one's, as the screen shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tilt {
    Clockwise,
    Anticlockwise,
}

/// A side of the guide, named by the way it faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// Whether a drawn ellipse falls short of a side or runs past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Short,
    Past,
}

/// An ellipse compared: the one fitted to the copy's lines (none found: nothing more), and how it
/// differs from the exact one.
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseCheck {
    pub fitted: Option<Ellipse>,
    pub roundness: Option<Roundness>,
    pub tilt: Option<Tilt>,
    pub sides: Vec<(Side, Reach)>,
}

impl EllipseCheck {
    /// The ellipse in `lines` (frame units) against `guide`'s exact one.
    #[must_use]
    pub fn of(guide: &EllipseGuide, lines: &[&[Point]]) -> Self {
        let Some(fitted) = fit_ellipse(lines) else {
            return Self {
                fitted: None,
                roundness: None,
                tilt: None,
                sides: Vec::new(),
            };
        };
        let exact = guide.exact;
        let degrees = fitted.degree() - exact.degree();
        let roundness = (degrees.abs() >= DEGREE).then_some(if degrees > 0.0 {
            Roundness::Rounder
        } else {
            Roundness::Flatter
        });
        let flat = fitted.roundness() < ROUND && exact.roundness() < ROUND;
        let turned = upright(fitted.angle - exact.angle).to_degrees();
        let tilt = (flat && turned.abs() >= TILT).then_some(if turned > 0.0 {
            Tilt::Clockwise
        } else {
            Tilt::Anticlockwise
        });
        Self {
            fitted: Some(fitted),
            roundness,
            tilt,
            sides: missed(guide, &fitted),
        }
    }
}

/// The sides of `guide` the ellipse `fitted` falls short of or runs past, in order round.
fn missed(guide: &EllipseGuide, fitted: &Ellipse) -> Vec<(Side, Reach)> {
    let c = guide.corners;
    let middle = [0, 1].map(|k| c.iter().map(|p| p[k]).sum::<f64>() / 4.0);
    (0..4)
        .filter_map(|i| {
            let (a, b) = (c[i], c[(i + 1) % 4]);
            let along = [b[0] - a[0], b[1] - a[1]];
            let long = along[0].hypot(along[1]).max(f64::MIN_POSITIVE);
            let mut normal = [along[1] / long, -along[0] / long];
            let outward = (a[0] + b[0]) / 2.0 - middle[0];
            let outward = [outward, (a[1] + b[1]) / 2.0 - middle[1]];
            if normal[0] * outward[0] + normal[1] * outward[1] < 0.0 {
                normal = [-normal[0], -normal[1]];
            }
            let gap = normal[0] * a[0] + normal[1] * a[1] - fitted.support(normal);
            let reach = if gap > REACH {
                Reach::Short
            } else if gap < -REACH {
                Reach::Past
            } else {
                return None;
            };
            let side = if normal[0].abs() >= normal[1].abs() {
                if normal[0] > 0.0 {
                    Side::Right
                } else {
                    Side::Left
                }
            } else if normal[1] > 0.0 {
                Side::Bottom
            } else {
                Side::Top
            };
            Some((side, reach))
        })
        .collect()
}

/// The ellipse that fits the points of `lines` best (Fitzgibbon's direct least squares); none
/// for too few points, or a fit too thin or too large to be one drawn.
#[must_use]
pub fn fit_ellipse(lines: &[&[Point]]) -> Option<Ellipse> {
    let points: Vec<Point> = lines.iter().flat_map(|l| resample(l, SPACING)).collect();
    if points.len() < 6 {
        return None;
    }
    let n = points.len() as f64;
    let centre = [0, 1].map(|k| points.iter().map(|p| p[k]).sum::<f64>() / n);
    let spread = (points.iter())
        .map(|p| (p[0] - centre[0]).powi(2) + (p[1] - centre[1]).powi(2))
        .sum::<f64>()
        / n;
    let scale = (spread / 2.0).sqrt();
    if scale <= 1e-9 {
        return None;
    }
    let normal: Vec<Point> = (points.iter())
        .map(|p| [(p[0] - centre[0]) / scale, (p[1] - centre[1]) / scale])
        .collect();
    let conic = direct_fit(&normal)?;
    let ellipse = Ellipse::of_conic(placed(conic, centre, scale))?;
    (ellipse.roundness() >= THINNEST && ellipse.axes[0] <= LARGEST).then_some(ellipse)
}

type Matrix = [[f64; 3]; 3];

/// The conic Halíř and Flusser's form of Fitzgibbon's fit finds for `points` (centred, scaled).
fn direct_fit(points: &[Point]) -> Option<[f64; 6]> {
    let mut s1 = [[0.0; 3]; 3];
    let mut s2 = [[0.0; 3]; 3];
    let mut s3 = [[0.0; 3]; 3];
    for &[x, y] in points {
        let quadratic = [x * x, x * y, y * y];
        let linear = [x, y, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                s1[i][j] += quadratic[i] * quadratic[j];
                s2[i][j] += quadratic[i] * linear[j];
                s3[i][j] += linear[i] * linear[j];
            }
        }
    }
    let s3_inverse = inverse(&s3)?;
    // T = -S3⁻¹ S2ᵀ; M = S1 + S2 T; the constraint's inverse taken in: rows 2/2, -1, 0/2.
    let t: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| -(0..3).map(|k| s3_inverse[i][k] * s2[j][k]).sum::<f64>())
    });
    let m: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| s1[i][j] + (0..3).map(|k| s2[i][k] * t[k][j]).sum::<f64>())
    });
    let reduced = [
        m[2].map(|v| v / 2.0),
        m[1].map(|v| -v),
        m[0].map(|v| v / 2.0),
    ];
    let first = (eigenvalues(&reduced).into_iter())
        .filter_map(|value| eigenvector(&reduced, value))
        .find(|a| 4.0 * a[0] * a[2] - a[1] * a[1] > 0.0)?;
    let rest: [f64; 3] = std::array::from_fn(|i| (0..3).map(|k| t[i][k] * first[k]).sum());
    Some([first[0], first[1], first[2], rest[0], rest[1], rest[2]])
}

/// The conic `[a, b, c, d, e, f]` of points centred on `centre` and divided by `scale`, as a conic
/// of the points themselves.
pub(crate) fn placed([a, b, c, d, e, f]: [f64; 6], [mx, my]: Point, s: f64) -> [f64; 6] {
    let s2 = s * s;
    [
        a / s2,
        b / s2,
        c / s2,
        (-2.0 * a * mx - b * my) / s2 + d / s,
        (-2.0 * c * my - b * mx) / s2 + e / s,
        (a * mx * mx + b * mx * my + c * my * my) / s2 - (d * mx + e * my) / s + f,
    ]
}

fn inverse(m: &Matrix) -> Option<Matrix> {
    let at = |i: usize, j: usize| m[i % 3][j % 3];
    let adjugate: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            at(j + 1, i + 1) * at(j + 2, i + 2) - at(j + 1, i + 2) * at(j + 2, i + 1)
        })
    });
    let det: f64 = (0..3).map(|k| m[0][k] * adjugate[k][0]).sum();
    (det.abs() > 1e-300).then(|| adjugate.map(|row| row.map(|v| v / det)))
}

/// The real eigenvalues of `m`: the real roots of its characteristic cubic.
fn eigenvalues(m: &Matrix) -> Vec<f64> {
    let trace = m[0][0] + m[1][1] + m[2][2];
    let minors = m[0][0] * m[1][1] - m[0][1] * m[1][0] + m[0][0] * m[2][2] - m[0][2] * m[2][0]
        + m[1][1] * m[2][2]
        - m[1][2] * m[2][1];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    cubic_roots(-trace, minors, -det)
}

/// The real roots of `x³ + a x² + b x + c`.
fn cubic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    let shift = a / 3.0;
    let p = b - a * a / 3.0;
    let q = 2.0 * a * a * a / 27.0 - a * b / 3.0 + c;
    let disc = (q / 2.0).powi(2) + (p / 3.0).powi(3);
    if disc > 0.0 {
        let root = disc.sqrt();
        return vec![(-q / 2.0 + root).cbrt() + (-q / 2.0 - root).cbrt() - shift];
    }
    if p.abs() < 1e-300 {
        return vec![-shift];
    }
    let r = (-p / 3.0).sqrt();
    let phi = (-q / (2.0 * r * r * r)).clamp(-1.0, 1.0).acos();
    let tau = std::f64::consts::TAU;
    (0..3)
        .map(|k| 2.0 * r * ((phi + tau * f64::from(k)) / 3.0).cos() - shift)
        .collect()
}

/// A vector `v` with `m v = value v`: the longest cross product of two rows of `m - value I`.
fn eigenvector(m: &Matrix, value: f64) -> Option<[f64; 3]> {
    let rows: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| m[i][j] - if i == j { value } else { 0.0 })
    });
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let norm = |v: &[f64; 3]| v.iter().map(|x| x * x).sum::<f64>();
    let best = [
        cross(rows[0], rows[1]),
        cross(rows[0], rows[2]),
        cross(rows[1], rows[2]),
    ]
    .into_iter()
    .max_by(|a, b| norm(a).total_cmp(&norm(b)))?;
    (norm(&best) > 0.0).then_some(best)
}
