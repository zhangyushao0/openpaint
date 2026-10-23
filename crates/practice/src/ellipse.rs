//! Ellipses (S84): Drawabox's tables of ellipses and V大's cylinder in a box. A guide is a plain
//! frame (a table's cell, upright) or a square in perspective (a box's face, a cylinder's top);
//! the ellipse that fits it touches the middle of each side, in perspective the middles as the
//! eye sees them (found through the diagonals' crossing, V大), so the exact ellipse is the image of
//! the circle inscribed in the square: its conic carried through the homography from the square
//! to the guide. Generated the same for the same seed; frame units (the frame's side 1, y down).

use crate::path::{Point, distance};
use crate::perspective_eye::{Eye, directions};
use crate::random::Random;

/// Guides laid before the generator keeps the last one whatever it is.
const TRIES: usize = 400;
/// How far inside the frame a guide keeps; its sides' least length.
const INSIDE: f64 = 0.06;
const SIDE: f64 = 0.15;
/// The exact ellipse's roundness (minor over major) at least and at most: about 15 to 76 degrees.
const ROUNDNESS: [f64; 2] = [0.26, 0.97];

/// An ellipse: its centre, its semi-axes (major first) and the major axis's direction, radians
/// in (-pi/2, pi/2], measured from x towards y (clockwise as the screen shows it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipse {
    pub centre: Point,
    pub axes: [f64; 2],
    pub angle: f64,
}

impl Ellipse {
    /// Its point at the parameter `t` (radians).
    #[must_use]
    pub fn at(&self, t: f64) -> Point {
        let (sin, cos) = self.angle.sin_cos();
        let [a, b] = self.axes;
        let (x, y) = (a * t.cos(), b * t.sin());
        [
            self.centre[0] + x * cos - y * sin,
            self.centre[1] + x * sin + y * cos,
        ]
    }

    /// Its outline as a closed line of `steps` pieces.
    #[must_use]
    pub fn outline(&self, steps: usize) -> Vec<Point> {
        let steps = steps.max(3);
        (0..=steps)
            .map(|k| self.at(k as f64 / steps as f64 * std::f64::consts::TAU))
            .collect()
    }

    /// How far it reaches along the unit `normal`: the greatest `normal · p` over its points.
    #[must_use]
    pub fn support(&self, normal: Point) -> f64 {
        let (sin, cos) = self.angle.sin_cos();
        let along = normal[0] * cos + normal[1] * sin;
        let across = -normal[0] * sin + normal[1] * cos;
        let [a, b] = self.axes;
        normal[0] * self.centre[0] + normal[1] * self.centre[1] + (a * along).hypot(b * across)
    }

    /// Its minor axis over its major: 1 a circle, near 0 a line.
    #[must_use]
    pub fn roundness(&self) -> f64 {
        self.axes[1] / self.axes[0].max(f64::MIN_POSITIVE)
    }

    /// Its degree as Drawabox names it: 90 a circle, 0 a line (the angle a circle is seen at).
    #[must_use]
    pub fn degree(&self) -> f64 {
        self.roundness().clamp(0.0, 1.0).asin().to_degrees()
    }

    /// The ellipse `a x² + b xy + c y² + d x + e y + f = 0`; none for another conic.
    #[must_use]
    pub fn of_conic([a, b, c, d, e, f]: [f64; 6]) -> Option<Self> {
        let det = 4.0 * a * c - b * b;
        if det <= 1e-15 * (a * a + b * b + c * c) {
            return None;
        }
        let centre = [(b * e - 2.0 * c * d) / det, (b * d - 2.0 * a * e) / det];
        let [x, y] = centre;
        let middle = a * x * x + b * x * y + c * y * y + d * x + e * y + f;
        let angle = 0.5 * b.atan2(a - c);
        let (sin, cos) = angle.sin_cos();
        let along = a * cos * cos + b * sin * cos + c * sin * sin;
        let across = a * sin * sin - b * sin * cos + c * cos * cos;
        let (first, second) = (-middle / along, -middle / across);
        if !(first > 0.0 && second > 0.0 && first.is_finite() && second.is_finite()) {
            return None;
        }
        let (first, second) = (first.sqrt(), second.sqrt());
        let (axes, angle) = if first >= second {
            ([first, second], angle)
        } else {
            ([second, first], angle + std::f64::consts::FRAC_PI_2)
        };
        Some(Self {
            centre,
            axes,
            angle: upright(angle),
        })
    }
}

/// `angle` (radians) as an axis's direction, in (-pi/2, pi/2].
pub(crate) fn upright(angle: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut angle = angle.rem_euclid(pi);
    if angle > pi / 2.0 {
        angle -= pi;
    }
    angle
}

/// What an ellipse is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Guide {
    /// A plain frame: a table's cell, upright, of any proportion.
    Frame,
    /// A square in perspective: a box's face or a cylinder's top.
    Plane,
}

impl Guide {
    pub const ALL: [Self; 2] = [Self::Frame, Self::Plane];
}

/// A guide and the ellipse that fits it: its four corners round it, the exact ellipse and where
/// it touches each side (side `i` runs from corner `i` to the next).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseGuide {
    pub corners: [Point; 4],
    pub exact: Ellipse,
    pub touches: [Point; 4],
}

impl EllipseGuide {
    /// The guide `guide` and `seed` make.
    #[must_use]
    pub fn generate(guide: Guide, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut last = None;
        for _ in 0..TRIES {
            let laid = match guide {
                Guide::Frame => Some(frame(&mut random)),
                Guide::Plane => plane(&mut random),
            };
            if let Some(laid) = laid {
                if laid.keeps() {
                    return laid;
                }
                last = Some(laid);
            }
        }
        last.unwrap_or_else(|| square_on([[0.25, 0.3], [0.75, 0.3], [0.75, 0.7], [0.25, 0.7]]))
    }

    fn keeps(&self) -> bool {
        let inside = |v: f64| (INSIDE..=1.0 - INSIDE).contains(&v);
        let turns: Vec<f64> = (0..4)
            .map(|i| {
                let [a, b, c] = [0, 1, 2].map(|k| self.corners[(i + k) % 4]);
                (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
            })
            .collect();
        let convex = turns.iter().all(|t| *t > 0.0) || turns.iter().all(|t| *t < 0.0);
        let sides = (0..4).all(|i| distance(self.corners[i], self.corners[(i + 1) % 4]) >= SIDE);
        convex
            && sides
            && self.corners.iter().all(|c| inside(c[0]) && inside(c[1]))
            && (ROUNDNESS[0]..=ROUNDNESS[1]).contains(&self.exact.roundness())
    }
}

/// A plain upright frame, a table's cell: wide or tall, its ellipse touching its sides' middles.
fn frame(random: &mut Random) -> EllipseGuide {
    let long = random.range(0.4, 0.8);
    let short = long * random.range(ROUNDNESS[0], 1.0);
    let [width, height] = if random.unit() < 0.4 {
        [short, long]
    } else {
        [long, short]
    };
    let middle = [
        0.5 + random.range(-0.04, 0.04),
        0.5 + random.range(-0.04, 0.04),
    ];
    let [x0, y0] = [middle[0] - width / 2.0, middle[1] - height / 2.0];
    let [x1, y1] = [x0 + width, y0 + height];
    square_on([[x0, y0], [x1, y0], [x1, y1], [x0, y1]])
}

/// A square seen by the eye: a horizontal one above or below it (a cylinder's top), or an
/// upright one (a box's side), fitted in the frame; none when the eye cannot see it whole.
fn plane(random: &mut Random) -> Option<EllipseGuide> {
    let eye = Eye {
        pitch: random.range(-12.0, 12.0).to_radians(),
    };
    let [left, right, up] = directions(random.range(20.0, 70.0).to_radians());
    let upright = random.unit() < 0.35;
    let height = if upright {
        random.range(-0.4, 0.4)
    } else {
        random.sign() * random.range(0.35, 1.3)
    };
    let middle = eye.to_world([random.range(-0.3, 0.3), height, random.range(2.2, 4.5)]);
    let (u, v) = match (upright, random.unit() < 0.5) {
        (true, true) => (right, up),
        (true, false) => (left, up),
        (false, _) => (right, left),
    };
    let corner = |su: f64, sv: f64| -> [f64; 3] {
        std::array::from_fn(|k| middle[k] + 0.5 * (su * u[k] + sv * v[k]))
    };
    let seen = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(su, sv)| eye.image(corner(su, sv)));
    let image: [Point; 4] = [seen[0]?, seen[1]?, seen[2]?, seen[3]?];
    let low = [0, 1].map(|k| image.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let high = [0, 1].map(|k| image.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    let scale = random.range(0.55, 0.72) / (high[0] - low[0]).max(high[1] - low[1]).max(1e-9);
    let shift = [random.range(-0.04, 0.04), random.range(-0.04, 0.04)];
    let place =
        |p: Point| [0, 1].map(|k| 0.5 + shift[k] + (p[k] - (low[k] + high[k]) / 2.0) * scale);
    Some(square_on(image.map(place)))
}

/// The guide whose corners are `corners` (a square's image, in its order round), its exact
/// ellipse the image of the square's inscribed circle.
fn square_on(corners: [Point; 4]) -> EllipseGuide {
    let to = homography(corners);
    let touches = [[0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]].map(|p| apply(&to, p));
    // The circle inscribed in the unit square: x² + y² - x - y + 1/4 = 0.
    let circle = [[1.0, 0.0, -0.5], [0.0, 1.0, -0.5], [-0.5, -0.5, 0.25]];
    let back = inverse(&to);
    let seen = product(&product(&transpose(&back), &circle), &back);
    let conic = [
        seen[0][0],
        seen[0][1] + seen[1][0],
        seen[1][1],
        seen[0][2] + seen[2][0],
        seen[1][2] + seen[2][1],
        seen[2][2],
    ];
    let fallback = Ellipse {
        centre: [0.5, 0.5],
        axes: [0.25, 0.2],
        angle: 0.0,
    };
    EllipseGuide {
        corners,
        exact: Ellipse::of_conic(conic).unwrap_or(fallback),
        touches,
    }
}

type Matrix = [[f64; 3]; 3];

/// The homography taking the unit square's corners (0,0), (1,0), (1,1), (0,1) to `q` (Heckbert's
/// square-to-quadrilateral).
fn homography(q: [Point; 4]) -> Matrix {
    let s = [0, 1].map(|k| q[0][k] - q[1][k] + q[2][k] - q[3][k]);
    let d1 = [0, 1].map(|k| q[1][k] - q[2][k]);
    let d2 = [0, 1].map(|k| q[3][k] - q[2][k]);
    let det = d1[0] * d2[1] - d2[0] * d1[1];
    let (g, h) = if det.abs() < 1e-15 {
        (0.0, 0.0)
    } else {
        (
            (s[0] * d2[1] - d2[0] * s[1]) / det,
            (d1[0] * s[1] - s[0] * d1[1]) / det,
        )
    };
    [
        [
            q[1][0] - q[0][0] + g * q[1][0],
            q[3][0] - q[0][0] + h * q[3][0],
            q[0][0],
        ],
        [
            q[1][1] - q[0][1] + g * q[1][1],
            q[3][1] - q[0][1] + h * q[3][1],
            q[0][1],
        ],
        [g, h, 1.0],
    ]
}

fn apply(m: &Matrix, [x, y]: Point) -> Point {
    let w = m[2][0] * x + m[2][1] * y + m[2][2];
    [
        (m[0][0] * x + m[0][1] * y + m[0][2]) / w,
        (m[1][0] * x + m[1][1] * y + m[1][2]) / w,
    ]
}

fn product(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn transpose(m: &Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| m[j][i]))
}

/// `m`'s inverse up to scale (its adjugate), which a conic needs no more of.
fn inverse(m: &Matrix) -> Matrix {
    let at = |i: usize, j: usize| m[i % 3][j % 3];
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            at(j + 1, i + 1) * at(j + 2, i + 2) - at(j + 1, i + 2) * at(j + 2, i + 1)
        })
    })
}
