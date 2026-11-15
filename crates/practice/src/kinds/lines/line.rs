//! Single lines to copy (V大's W1 single lines): a curve in a square frame, from near-straight
//! lines to arcs, S-curves, hooks and three-bend meanders, any way round, long or short; and a
//! long arc or S carrying small ripples (V大's W2 hierarchy: the big trend first, then the small
//! ones on it; S141).
//!
//! A line is drawn by its curvature along its length (a near-constant bend, one bend, two or three
//! alternating bends, a gentle stretch ending in a tight turn), integrated into a path of unit
//! length, then turned, sized and placed in the frame. A layered line's ripples are laid across
//! its trend, each about 0.035 of the frame from crest to trough (about three pen widths), so a
//! ripple copied a pen's width off shows. Everything follows from the tier and the seed
//! (`Random`), so the same seed gives the same line.

use crate::maths::path::Point;
use crate::maths::random::Random;

/// The frame's margin the line keeps clear of, in frame sides.
const MARGIN: f64 = 0.12;
/// Steps the curvature is integrated in.
pub(crate) const STEPS: usize = 480;

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
    /// Three bends, alternating: an S with one more turn (S141).
    Meander,
    /// A long arc or S carrying three to six small ripples (S141).
    Layered,
}

impl LineTier {
    pub const ALL: [Self; 6] = [
        Self::Straight,
        Self::Arc,
        Self::Wave,
        Self::Hook,
        Self::Meander,
        Self::Layered,
    ];
}

/// A line to copy, in frame units (the frame from `[0, 0]` to `[1, 1]`, y down).
#[derive(Debug, Clone, PartialEq)]
pub struct LineExercise {
    /// The line, densely: about `STEPS` points.
    pub points: Vec<Point>,
}

impl LineExercise {
    /// The line of `tier` that `seed` makes.
    #[must_use]
    pub fn generate(tier: LineTier, seed: u64) -> Self {
        Self {
            points: drawn(tier, seed).0,
        }
    }
}

/// The line of `tier` that `seed` makes and the trend it follows (the line itself but for a
/// layered line's ripples), both placed in the frame alike.
pub(super) fn drawn(tier: LineTier, seed: u64) -> (Vec<Point>, Vec<Point>) {
    let mut random = Random::new(seed);
    let curvature = Curvature::of(tier, &mut random);
    let turn = random.range(0.0, std::f64::consts::TAU);
    let trend = integrate(&curvature, turn);
    let size = match tier {
        LineTier::Straight => random.range(0.4, 0.76),
        LineTier::Arc => random.range(0.4, 0.72),
        LineTier::Wave => random.range(0.45, 0.76),
        LineTier::Hook => random.range(0.38, 0.66),
        LineTier::Meander => random.range(0.55, 0.76),
        LineTier::Layered => random.range(0.62, 0.76),
    };
    let line = if tier == LineTier::Layered {
        rippled(&trend, size, &mut random)
    } else {
        trend.clone()
    };
    let placing = Placing::of(&[&line], size, &mut random);
    let placed = |points: &[Point]| points.iter().map(|p| placing.apply(*p)).collect();
    (placed(&line), placed(&trend))
}

/// A layered line's ripples, in frame sides from crest to trough: two and a half to three pen
/// widths, so a ripple copied a pen's width off shows.
const RIPPLE: [f64; 2] = [0.03, 0.036];

/// `trend` (a path of unit length, its points evenly along it) carrying three or four ripples
/// across it, each a wave of 0.17 to 0.21 of its length (shorter ones, as high, swing 45 degrees
/// or more off the trend and read as steps or hooks), somewhere along it away from its ends,
/// sized so that placed at `size` (`Placing`) each is `RIPPLE` high.
fn rippled(trend: &[Point], size: f64, random: &mut Random) -> Vec<Point> {
    let count = if random.unit() < 0.5 { 3 } else { 4 };
    let wave = random.range(0.17, 0.21);
    let reach = wave * count as f64;
    let from = random.range(0.08, 0.92 - reach);
    let height = random.sign() * random.range(RIPPLE[0], RIPPLE[1]);
    // Half the height at the scale the line is placed at, which the ripples change a little:
    // measured on the trend, then once more on the line they make.
    let across = |line: &[Point]| height / 2.0 * extent(line) / size;
    let first = ripples(trend, (count, wave, from), across(trend));
    ripples(trend, (count, wave, from), across(&first))
}

/// `trend` moved across itself by `amplitude` times a sine over `count` waves of `wave` (shares of
/// its length) from the share `from`, faded in over its first quarter wave and out over its last,
/// so the line leaves its trend and comes back to it without a corner, every crest at full height.
fn ripples(trend: &[Point], (count, wave, from): (usize, f64, f64), amplitude: f64) -> Vec<Point> {
    let last = trend.len().saturating_sub(1).max(1);
    let waves = count as f64;
    (0..trend.len())
        .map(|index| {
            let at = (index as f64 / last as f64 - from) / wave;
            let p = trend[index];
            if !(0.0..=waves).contains(&at) {
                return p;
            }
            let faded = (4.0 * at.min(waves - at)).min(1.0);
            let fade = faded * faded * (3.0 - 2.0 * faded);
            let off = amplitude * fade * (std::f64::consts::TAU * at).sin();
            let (a, b) = (trend[index.saturating_sub(1)], trend[(index + 1).min(last)]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let norm = dx.hypot(dy).max(1e-12);
            [p[0] - dy / norm * off, p[1] + dx / norm * off]
        })
        .collect()
}

/// The larger side of `points`' box.
fn extent(points: &[Point]) -> f64 {
    let (low, high) = bounds(points.iter());
    (high[0] - low[0]).max(high[1] - low[1])
}

/// Curvature along the unit length, in radians per unit of length.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Curvature {
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
    /// `turns[k]` over the `k`th of three stretches cut at `cuts`, each a half sine.
    Meander { turns: [f64; 3], cuts: [f64; 2] },
}

impl Curvature {
    pub(crate) fn of(tier: LineTier, random: &mut Random) -> Self {
        match tier {
            LineTier::Straight => Self::Linear {
                a: random.sign() * random.range(0.0, 0.22),
                b: random.sign() * random.range(0.0, 0.3),
            },
            LineTier::Arc => Self::arc((0.7, 1.9), 1.3, random),
            LineTier::Wave => Self::wave((0.6, 1.4), random),
            LineTier::Hook => Self::Hook {
                base: random.sign() * random.range(0.0, 0.5),
                turn: random.sign() * random.range(2.0, 3.2),
                at: random.range(0.72, 0.86),
                width: random.range(0.045, 0.08),
            },
            LineTier::Meander => {
                let sign = random.sign();
                Self::Meander {
                    turns: [sign, -sign, sign].map(|side| side * random.range(0.6, 1.3)),
                    cuts: [random.range(0.27, 0.36), random.range(0.6, 0.7)],
                }
            }
            // A trend bowing about twice its ripples' height or more: two in three an arc, else
            // an S deeper than a plain one.
            LineTier::Layered => {
                if random.unit() < 2.0 / 3.0 {
                    Self::arc((1.2, 1.9), 0.8, random)
                } else {
                    Self::wave((1.4, 1.8), random)
                }
            }
        }
    }

    /// One bend of `turns` either way, gathering towards one end by up to `lean`.
    fn arc(turns: (f64, f64), lean: f64, random: &mut Random) -> Self {
        let turn = random.sign() * random.range(turns.0, turns.1);
        let lean = random.range(-lean, lean);
        Self::Linear {
            a: turn,
            b: turn * lean,
        }
    }

    /// Two opposite bends, each of `turns`, either way.
    fn wave(turns: (f64, f64), random: &mut Random) -> Self {
        let sign = random.sign();
        Self::Wave {
            turn1: sign * random.range(turns.0, turns.1),
            turn2: sign * random.range(turns.0, turns.1),
            middle: random.range(0.38, 0.62),
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
            Self::Meander { turns, cuts } => {
                let ends = [0.0, cuts[0], cuts[1], 1.0];
                let k = usize::from(s >= cuts[0]) + usize::from(s >= cuts[1]);
                let (from, width) = (ends[k], ends[k + 1] - ends[k]);
                turns[k] * PI / (2.0 * width) * (PI * (s - from) / width).sin()
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
/// origin.
pub(crate) fn integrate(curvature: &Curvature, turn: f64) -> Vec<Point> {
    let step = 1.0 / STEPS as f64;
    let mut points = Vec::with_capacity(STEPS + 1);
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
    }
    points
}

/// Lines scaled so the larger side of their box is `size` frame sides, placed anywhere in the
/// frame that keeps the margin clear: `p` goes to `corner + (p - low) scale`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Placing {
    low: Point,
    scale: f64,
    corner: Point,
}

impl Placing {
    pub(crate) fn of(lines: &[&[Point]], size: f64, random: &mut Random) -> Self {
        let (low, high) = bounds(lines.iter().flat_map(|line| line.iter()));
        let extent = (high[0] - low[0]).max(high[1] - low[1]).max(1e-9);
        let scale = size / extent;
        let box_size = [(high[0] - low[0]) * scale, (high[1] - low[1]) * scale];
        let corner = [0, 1].map(|axis| {
            let room = (1.0 - 2.0 * MARGIN - box_size[axis]).max(0.0);
            MARGIN + random.range(0.0, room)
        });
        Self { low, scale, corner }
    }

    pub(crate) fn apply(&self, p: Point) -> Point {
        [
            self.corner[0] + (p[0] - self.low[0]) * self.scale,
            self.corner[1] + (p[1] - self.low[1]) * self.scale,
        ]
    }
}

fn bounds<'a>(points: impl Iterator<Item = &'a Point>) -> (Point, Point) {
    points.fold(
        ([f64::MAX, f64::MAX], [f64::MIN, f64::MIN]),
        |(low, high), p| {
            (
                [low[0].min(p[0]), low[1].min(p[1])],
                [high[0].max(p[0]), high[1].max(p[1])],
            )
        },
    )
}
