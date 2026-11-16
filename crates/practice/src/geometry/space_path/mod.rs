//! A smooth path in space (S155), the spine a kind sweeps a form along: the sausage's tube
//! (contours) and the ribbon flowing back through space (organic arrows).
//!
//! The curve is a centripetal Catmull-Rom spline through the given points (each knot step the
//! square root of its chord): C1, through every point, no cusp and no loop within a span however
//! unevenly the points lie, each span shaped by its four nearest points alone. A natural cubic
//! spline's continuous curvature buys nothing a drawn tube or ribbon shows, while it is global and
//! overshoots between unevenly spaced points. The ends' phantom points mirror the second point and
//! the last but one, so each end leaves along its chord; points closer than `SAME_POINT` are one.
//!
//! Sampled evenly by arc length (each span's length tabled by Gauss-Legendre, a point placed by
//! Newton's steps), each sample carries a frame: the unit tangent, and a normal and binormal
//! turning about it as little as the curve lets them (rotation-minimising, by double reflection:
//! Wang, Juttler, Zheng and Liu 2008), the normal started from a given up vector and carried along
//! at least `FRAME_STEPS` steps whatever the samples; then turned about the tangent by the turns
//! placed along the length (a ribbon turning over: a half turn is `by` pi).
//!
//! The world: y up, z away from the eye (`maths/vector.rs`); `s` is a share of the length, 0 to 1.

use crate::maths::vector::{Vector, across, add, cross, dot, norm, sub, times, unit};

/// Points closer than this are one.
const SAME_POINT: f64 = 1e-12;
/// The sub-steps of each span's length table.
const STEPS: usize = 32;
/// The fewest steps the frame is carried along over the whole length.
const FRAME_STEPS: usize = 1024;
/// Newton's steps placing a point at its length.
const NEWTON: usize = 4;
/// The tangent of a path of no length: away from the eye.
const NO_WAY: Vector = [0.0, 0.0, 1.0];

/// A span's cubic in its own parameter `u`, 0 to 1: the coefficients of 1, u, u² and u³.
type Cubic = [Vector; 4];

/// A turn of the frame about the tangent: `by` radians (right-handed about the tangent) gained
/// smoothly over `span` of the length centred at `at`, both shares of the length. A half turn is
/// `by` pi, or minus pi the other way round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Turn {
    pub(crate) at: f64,
    pub(crate) span: f64,
    pub(crate) by: f64,
}

impl Turn {
    /// The angle gained by `s`: none before the span, all of `by` after it, smoothly between.
    fn gained(&self, s: f64) -> f64 {
        let x = if self.span > 0.0 {
            ((s - self.at) / self.span + 0.5).clamp(0.0, 1.0)
        } else if s >= self.at {
            1.0
        } else {
            0.0
        };
        self.by * x * x * (3.0 - 2.0 * x)
    }
}

/// A point of the path with its frame: `tangent`, `normal` and `binormal` one long and square to
/// each other, `binormal` the tangent crossed with the normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sample {
    /// Its share of the length.
    pub(crate) s: f64,
    pub(crate) at: Vector,
    pub(crate) tangent: Vector,
    pub(crate) normal: Vector,
    pub(crate) binormal: Vector,
}

/// A smooth path through points in space.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpacePath {
    /// One cubic a span between two neighbouring points.
    spans: Vec<Cubic>,
    /// The length from the start at the end of each span's sub-steps, the first 0.
    along: Vec<f64>,
    /// The first point: all a path of no length is.
    start: Vector,
}

impl SpacePath {
    /// The path through `points` in order (three to five for the kinds; two give the straight way
    /// between them, one a path of no length); none for no point.
    pub(crate) fn through(points: &[Vector]) -> Option<Self> {
        let mut kept: Vec<Vector> = Vec::with_capacity(points.len());
        for &point in points {
            if kept
                .last()
                .is_none_or(|&last| norm(sub(point, last)) > SAME_POINT)
            {
                kept.push(point);
            }
        }
        let start = *kept.first()?;
        let spans: Vec<Cubic> = (1..kept.len()).map(|end| span(&kept, end)).collect();
        let mut along = vec![0.0];
        let mut total = 0.0;
        for cubic in &spans {
            for step in 0..STEPS {
                total += arc(cubic, fraction(step), fraction(step + 1));
                along.push(total);
            }
        }
        Some(Self {
            spans,
            along,
            start,
        })
    }

    /// The length.
    pub(crate) fn length(&self) -> f64 {
        self.along.last().copied().unwrap_or(0.0)
    }

    /// The point at `s` (held to 0 to 1).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "S155: its tests measure the path by it; no kind needs it yet"
        )
    )]
    pub(crate) fn at(&self, s: f64) -> Vector {
        self.place(s)
            .map_or(self.start, |(cubic, u)| point(cubic, u))
    }

    /// `n` samples at even steps of the length, the first at the start and the last at the end,
    /// each with its frame: the normal started from `up` made square to the first tangent (a way
    /// across the tangent where `up` lies along it), carried along rotation-minimising, then
    /// turned about the tangent by `turns`.
    pub(crate) fn samples(&self, n: usize, up: Vector, turns: &[Turn]) -> Vec<Sample> {
        if n == 0 {
            return Vec::new();
        }
        let gaps = n - 1;
        let between = FRAME_STEPS.div_ceil(gaps.max(1));
        let fine = gaps * between;
        let mut out = Vec::with_capacity(n);
        let mut carried: Option<(Vector, Vector, Vector)> = None;
        for step in 0..=fine {
            let s = if fine == 0 {
                0.0
            } else {
                step as f64 / fine as f64
            };
            let (at, tangent) = self.pointed(s);
            let normal = carried.map_or_else(
                || started(tangent, up),
                |before| reflected(before, (at, tangent)),
            );
            if step % between == 0 {
                let binormal = cross(tangent, normal);
                let sample = Sample {
                    s,
                    at,
                    tangent,
                    normal,
                    binormal,
                };
                out.push(turned(sample, turns));
            }
            carried = Some((at, tangent, normal));
        }
        out
    }

    /// The point and the unit tangent at `s`.
    fn pointed(&self, s: f64) -> (Vector, Vector) {
        self.place(s).map_or((self.start, NO_WAY), |(cubic, u)| {
            (point(cubic, u), unit(velocity(cubic, u)))
        })
    }

    /// The span and its own parameter at `s`; none on a path of no length.
    fn place(&self, s: f64) -> Option<(&Cubic, f64)> {
        let last = self.along.len().checked_sub(2)?;
        let target = s.clamp(0.0, 1.0) * self.length();
        let k = self
            .along
            .partition_point(|&length| length <= target)
            .saturating_sub(1)
            .min(last);
        let cubic = &self.spans[k / STEPS];
        let (from, to) = (fraction(k % STEPS), fraction(k % STEPS + 1));
        let step = self.along[k + 1] - self.along[k];
        let mut u = if step > 0.0 {
            from + (to - from) * (target - self.along[k]) / step
        } else {
            from
        };
        for _ in 0..NEWTON {
            let speed = norm(velocity(cubic, u));
            if speed <= 0.0 {
                break;
            }
            let gap = self.along[k] + arc(cubic, from, u) - target;
            u = (u - gap / speed).clamp(from, to);
        }
        Some((cubic, u))
    }
}

/// The parameter at the end of a span's `step`th sub-step.
fn fraction(step: usize) -> f64 {
    step as f64 / STEPS as f64
}

/// The cubic of the span from `points[end - 1]` to `points[end]`, its neighbours mirrored past
/// the ends.
fn span(points: &[Vector], end: usize) -> Cubic {
    let last = points.len() - 1;
    let mirrored = |about: Vector, point: Vector| sub(times(about, 2.0), point);
    let before = if end >= 2 {
        points[end - 2]
    } else {
        mirrored(points[0], points[1])
    };
    let after = if end < last {
        points[end + 1]
    } else {
        mirrored(points[last], points[last - 1])
    };
    cubic([before, points[end - 1], points[end], after])
}

/// The span from `b` to `c` of the centripetal Catmull-Rom curve through `a`, `b`, `c` and `d`:
/// its ends and their tangents, the knot's slopes scaled to the span's own parameter (Hermite),
/// as powers of that parameter.
fn cubic([a, b, c, d]: [Vector; 4]) -> Cubic {
    let knot = |p: Vector, q: Vector| norm(sub(q, p)).sqrt();
    let (before, within, after) = (knot(a, b), knot(b, c), knot(c, d));
    let start = times(slope([a, b, c], [before, within]), within);
    let end = times(slope([b, c, d], [within, after]), within);
    let chord = sub(c, b);
    let square = sub(sub(times(chord, 3.0), times(start, 2.0)), end);
    let cube = add(add(times(chord, -2.0), start), end);
    [b, start, square, cube]
}

/// The curve's derivative by its knots at `q`, between `p` and `r`, the knot steps before and
/// after it given (Barry and Goldman's pyramid, differentiated).
fn slope([p, q, r]: [Vector; 3], [before, after]: [f64; 2]) -> Vector {
    let into = times(sub(q, p), 1.0 / before);
    let across = times(sub(r, p), 1.0 / (before + after));
    let out = times(sub(r, q), 1.0 / after);
    add(sub(into, across), out)
}

fn point([a, b, c, d]: &Cubic, u: f64) -> Vector {
    add(*a, times(add(*b, times(add(*c, times(*d, u)), u)), u))
}

fn velocity([_, b, c, d]: &Cubic, u: f64) -> Vector {
    add(*b, times(add(times(*c, 2.0), times(*d, 3.0 * u)), u))
}

/// The length of `cubic` from `from` to `to`, by Gauss-Legendre's three points.
fn arc(cubic: &Cubic, from: f64, to: f64) -> f64 {
    let (middle, half) = ((from + to) / 2.0, (to - from) / 2.0);
    let off = half * 0.6_f64.sqrt();
    let speed = |u: f64| norm(velocity(cubic, u));
    half * (5.0 * speed(middle - off) + 8.0 * speed(middle) + 5.0 * speed(middle + off)) / 9.0
}

/// The normal the frame starts from: `up` made square to `tangent` and one long, or a way across
/// the tangent where `up` lies along it or has no length.
fn started(tangent: Vector, up: Vector) -> Vector {
    let square = sub(up, times(tangent, dot(up, tangent)));
    let long = norm(square);
    if long > 1e-9 * norm(up) {
        times(square, 1.0 / long)
    } else {
        across(tangent)[0]
    }
}

/// The normal at `before`'s point carried to the next point, by double reflection: in the plane
/// square to the chord between them, then in the plane that takes the reflected tangent to `next`;
/// made square to `next` and one long again, so rounding never gathers.
fn reflected(
    (from, tangent, normal): (Vector, Vector, Vector),
    (to, next): (Vector, Vector),
) -> Vector {
    let mirror = |a: Vector, by: Vector| {
        let long = dot(by, by);
        if long > 0.0 {
            sub(a, times(by, 2.0 * dot(by, a) / long))
        } else {
            a
        }
    };
    let chord = sub(to, from);
    let (normal, tangent) = (mirror(normal, chord), mirror(tangent, chord));
    let normal = mirror(normal, sub(next, tangent));
    unit(sub(normal, times(next, dot(normal, next))))
}

/// `sample`'s frame turned about its tangent by the angle `turns` have gained by its place.
fn turned(sample: Sample, turns: &[Turn]) -> Sample {
    let angle: f64 = turns.iter().map(|turn| turn.gained(sample.s)).sum();
    let (sin, cos) = angle.sin_cos();
    let Sample {
        normal, binormal, ..
    } = sample;
    Sample {
        normal: add(times(normal, cos), times(binormal, sin)),
        binormal: sub(times(binormal, cos), times(normal, sin)),
        ..sample
    }
}

#[cfg(test)]
mod tests;
