//! Single lines to copy (V大's W1 single lines): a curve in a square frame, from near-straight
//! lines to arcs, S-curves and hooks, any way round, long or short, and the key points a copy is
//! judged by first: its two ends and the most bulging point of each bend (V大: fix the ends and
//! the most obvious turns first).
//!
//! A line is drawn by its curvature along its length (a near-constant bend, one bend, two
//! opposite bends, a gentle stretch ending in a tight turn), integrated into a path of unit
//! length, then turned, sized and placed in the frame. Everything follows from the tier and the
//! seed (`Random`), so the same seed gives the same line.

use crate::path::{Point, distance};
use crate::random::Random;

/// The frame's margin the line keeps clear of, in frame sides.
const MARGIN: f64 = 0.12;
/// Steps the curvature is integrated in.
const STEPS: usize = 480;
/// A bend whose most bulging point lies less than this far from its chord (frame sides) has
/// none worth fixing: a near-straight line has only its ends.
const LEAST_BULGE: f64 = 0.02;
/// Key points closer than this to one already taken (frame sides) are the same place.
const DISTINCT: f64 = 0.05;

/// How a line bends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineTier {
    /// Almost straight: at most a slight bow.
    Straight,
    /// One bend, evenly or more at one end.
    Arc,
    /// Two opposite bends: an S.
    Wave,
    /// A gentle stretch ending in a tight turn.
    Hook,
}

impl LineTier {
    pub const ALL: [Self; 4] = [Self::Straight, Self::Arc, Self::Wave, Self::Hook];
}

/// A line to copy, in frame units (the frame from `[0, 0]` to `[1, 1]`, y down).
#[derive(Debug, Clone, PartialEq)]
pub struct LineExercise {
    /// The line, densely: about `STEPS` points.
    pub points: Vec<Point>,
    /// Its ends, then the most bulging point of each bend that bulges.
    pub key_points: Vec<Point>,
}

impl LineExercise {
    /// The line of `tier` that `seed` makes.
    #[must_use]
    pub fn generate(tier: LineTier, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let curvature = Curvature::of(tier, &mut random);
        let turn = random.range(0.0, std::f64::consts::TAU);
        let (points, signs) = integrate(&curvature, turn);
        let size = match tier {
            LineTier::Straight => random.range(0.4, 0.76),
            LineTier::Arc => random.range(0.4, 0.72),
            LineTier::Wave => random.range(0.45, 0.76),
            LineTier::Hook => random.range(0.38, 0.66),
        };
        let points = placed(&points, size, &mut random);
        let key_points = key_points(&points, &signs);
        Self { points, key_points }
    }
}

/// Curvature along the unit length, in radians per unit of length.
#[derive(Debug, Clone, Copy)]
enum Curvature {
    /// `a + b (s - 0.5)`.
    Linear { a: f64, b: f64 },
    /// `turn1` over `0..middle` and `-turn2` over `middle..1`, each a half sine.
    Wave { turn1: f64, turn2: f64, middle: f64 },
    /// `base` everywhere plus a turn of `turn` gathered round `at` (a Gaussian of `width`).
    Hook {
        base: f64,
        turn: f64,
        at: f64,
        width: f64,
    },
}

impl Curvature {
    fn of(tier: LineTier, random: &mut Random) -> Self {
        match tier {
            LineTier::Straight => Self::Linear {
                a: random.sign() * random.range(0.0, 0.22),
                b: random.sign() * random.range(0.0, 0.3),
            },
            LineTier::Arc => {
                let turn = random.sign() * random.range(0.7, 1.9);
                // Uneven: the bend gathers towards one end.
                let lean = random.range(-1.3, 1.3);
                Self::Linear {
                    a: turn,
                    b: turn * lean,
                }
            }
            LineTier::Wave => {
                let sign = random.sign();
                Self::Wave {
                    turn1: sign * random.range(0.6, 1.4),
                    turn2: sign * random.range(0.6, 1.4),
                    middle: random.range(0.38, 0.62),
                }
            }
            LineTier::Hook => Self::Hook {
                base: random.sign() * random.range(0.0, 0.5),
                turn: random.sign() * random.range(2.0, 3.2),
                at: random.range(0.72, 0.86),
                width: random.range(0.045, 0.08),
            },
        }
    }

    fn at(self, s: f64) -> f64 {
        use std::f64::consts::PI;
        match self {
            Self::Linear { a, b } => a + b * (s - 0.5),
            Self::Wave {
                turn1,
                turn2,
                middle,
            } => {
                if s < middle {
                    turn1 * PI / (2.0 * middle) * (PI * s / middle).sin()
                } else {
                    let rest = 1.0 - middle;
                    -turn2 * PI / (2.0 * rest) * (PI * (s - middle) / rest).sin()
                }
            }
            Self::Hook {
                base,
                turn,
                at,
                width,
            } => {
                let gauss = (-(s - at).powi(2) / (2.0 * width * width)).exp();
                base + turn * gauss / (width * (2.0 * PI).sqrt())
            }
        }
    }
}

/// The path of unit length whose direction starts at `turn` and turns by `curvature`, from the
/// origin, and the sign of the curvature on each of its steps.
fn integrate(curvature: &Curvature, turn: f64) -> (Vec<Point>, Vec<f64>) {
    let step = 1.0 / STEPS as f64;
    let mut points = Vec::with_capacity(STEPS + 1);
    let mut signs = Vec::with_capacity(STEPS);
    let (mut at, mut heading) = ([0.0, 0.0], turn);
    points.push(at);
    for index in 0..STEPS {
        let s = (index as f64 + 0.5) * step;
        let bend = curvature.at(s);
        // The midpoint rule: the heading half a step on.
        let middle = heading + bend * step / 2.0;
        at = [at[0] + middle.cos() * step, at[1] + middle.sin() * step];
        heading += bend * step;
        points.push(at);
        signs.push(bend.signum());
    }
    (points, signs)
}

/// `points` scaled so the larger side of their box is `size` frame sides, placed anywhere in the
/// frame that keeps the margin clear.
fn placed(points: &[Point], size: f64, random: &mut Random) -> Vec<Point> {
    let (low, high) = bounds(points);
    let extent = (high[0] - low[0]).max(high[1] - low[1]).max(1e-9);
    let scale = size / extent;
    let box_size = [(high[0] - low[0]) * scale, (high[1] - low[1]) * scale];
    let corner = [0, 1].map(|axis| {
        let room = (1.0 - 2.0 * MARGIN - box_size[axis]).max(0.0);
        MARGIN + random.range(0.0, room)
    });
    points
        .iter()
        .map(|p| {
            [
                corner[0] + (p[0] - low[0]) * scale,
                corner[1] + (p[1] - low[1]) * scale,
            ]
        })
        .collect()
}

fn bounds(points: &[Point]) -> (Point, Point) {
    points.iter().fold(
        ([f64::MAX, f64::MAX], [f64::MIN, f64::MIN]),
        |(low, high), p| {
            (
                [low[0].min(p[0]), low[1].min(p[1])],
                [high[0].max(p[0]), high[1].max(p[1])],
            )
        },
    )
}

/// The two ends, then for each stretch bending one way (`signs` per step) its point farthest
/// from the stretch's chord, if that lies at least `LEAST_BULGE` from it and apart from the
/// points already taken.
fn key_points(points: &[Point], signs: &[f64]) -> Vec<Point> {
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return Vec::new();
    };
    let mut keys = vec![first, last];
    let mut start = 0;
    for end in 1..=signs.len() {
        if end < signs.len() && signs[end] == signs[start] {
            continue;
        }
        let stretch = &points[start..=end];
        if let Some(bulge) = bulge(stretch)
            && keys.iter().all(|key| distance(*key, bulge) >= DISTINCT)
        {
            keys.push(bulge);
        }
        start = end;
    }
    keys
}

/// The point of `stretch` farthest from the chord between its ends, if far enough.
fn bulge(stretch: &[Point]) -> Option<Point> {
    let (&a, &b) = (stretch.first()?, stretch.last()?);
    let chord = distance(a, b).max(1e-12);
    let off =
        |p: &Point| ((b[0] - a[0]) * (a[1] - p[1]) - (a[0] - p[0]) * (b[1] - a[1])).abs() / chord;
    let (point, far) =
        stretch
            .iter()
            .map(|p| (*p, off(p)))
            .fold(([0.0, 0.0], f64::MIN), |best, next| {
                if next.1 > best.1 { next } else { best }
            });
    (far >= LEAST_BULGE).then_some(point)
}
