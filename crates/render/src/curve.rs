//! The curve a stroke follows (S34): a centripetal Catmull-Rom spline through its knots, every
//! sample but the ones Android makes up once a frame on the straight line between or past real
//! ones (`knots`).
//! Each span between two knots is a cubic Bézier; the evaluator flattens it into straight
//! pieces, as many as keep it within `TOLERANCE` output pixels of its curve at the scale it is
//! evaluated at (`pieces`), so the canvas pays little and an 8x view stays smooth.
//!
//! Centripetal parameterisation (the square root of the distance between knots) never loops or
//! cusps inside a span, so reversals and samples almost on top of each other stay tight; the
//! ends take a mirrored knot, so a two-sample stroke is its straight segment exactly.

use openpaint_core::StrokeSample;

/// How far a flattened span may lie from its curve, in output pixels (draweave's budget).
pub(crate) const TOLERANCE: f32 = 1.0 / 16.0;
/// Most straight pieces a span is flattened into.
pub(crate) const MAX_PIECES: u32 = 128;
/// A sample with the pressure of the one before, within this many canvas pixels of the line
/// Android made it on (the chord between its neighbours, or the line through the two knots
/// before it), that line at least `CHORD` long, is a resample, not a knot: `stroke-stats` found
/// Android's within 0.02 px.
const ON_CHORD: f32 = 0.02;
const CHORD: f32 = 2.0;
/// Below this parameter length (a square root of canvas pixels) two knots count as one point.
const NEAR: f32 = 1e-3;

/// The indices of the knots among `samples`, in order: the first and the last sample, and every
/// sample between that is not a resample: on the chord to the next sample (Android
/// interpolating) or on the line of the two knots before it, past them and sooner than the
/// samples came before (Android predicting when no later sample has come yet). A resample needs a knot
/// before it, so between two knots lies at most one. Deterministic left to right: a sample's
/// place depends only on the samples up to the one after it.
pub(crate) fn knots(samples: &[StrokeSample]) -> Vec<usize> {
    let mut knots: Vec<usize> = Vec::with_capacity(samples.len());
    for i in 0..samples.len() {
        let skipped = i > 0
            && i + 1 < samples.len()
            && knots.last() == Some(&(i - 1))
            && made_up(samples, &knots, i);
        if !skipped {
            knots.push(i);
        }
    }
    knots
}

/// Whether sample `i`, after the knot `i - 1`, is one Android made up: predicted from the last
/// two knots, or on the chord to the next sample unless that next one is itself predicted from
/// `i - 1` and `i` (with one pressure all along, three samples on a line could be either).
fn made_up(samples: &[StrokeSample], knots: &[usize], i: usize) -> bool {
    let (before, now, next) = (&samples[i - 1], &samples[i], &samples[i + 1]);
    let gap = |a: &StrokeSample, b: &StrokeSample| b.time_ms().saturating_sub(a.time_ms());
    if let Some(earlier) = knots.len().checked_sub(2).map(|k| &samples[knots[k]])
        && predicted(
            [earlier, before, now],
            gap(&samples[i.saturating_sub(2)], before),
        )
    {
        return true;
    }
    resampled([before, now, next]) && !predicted([before, now, next], gap(before, now))
}

/// Whether `c` has `b`'s pressure, comes sooner after `b` than `b` after the sample before it
/// (`gap` ms) and lies on the line from `a` through `b`, beyond `b`, within `ON_CHORD`, `a` and
/// `b` at least `CHORD` apart: where Android puts a frame's sample it predicts from the last
/// two, 0 to 3 ms ahead (real samples come 3 to 4 ms apart, injected ones 16).
fn predicted([a, b, c]: [&StrokeSample; 3], gap: u32) -> bool {
    let [a_, b_, c_] = [a, b, c].map(point);
    let (line, step) = (sub(b_, a_), sub(c_, b_));
    let length = len(line);
    let sooner = c.time_ms().saturating_sub(b.time_ms()) < gap;
    length >= CHORD
        && sooner
        && c.pressure() == b.pressure()
        && cross(step, line).abs() / length < ON_CHORD
        && dot(step, line) > 0.0
}

/// Whether `b` lies on the chord from `a` to `c` with `a`'s pressure (see `ON_CHORD`).
fn resampled([a, b, c]: [&StrokeSample; 3]) -> bool {
    let [a_, b_, c_] = [a, b, c].map(point);
    let chord = sub(c_, a_);
    let length = len(chord);
    if length < CHORD || b.pressure() != a.pressure() {
        return false;
    }
    let offset = sub(b_, a_);
    let along = dot(offset, chord) / (length * length);
    cross(offset, chord).abs() / length < ON_CHORD && (0.0..=1.0).contains(&along)
}

pub(crate) fn point(sample: &StrokeSample) -> [f32; 2] {
    let position = sample.position();
    [position.x(), position.y()]
}

/// One span as a cubic Bézier: from `0[0]` to `0[3]`, controls `0[1]` and `0[2]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cubic(pub(crate) [[f32; 2]; 4]);

impl Cubic {
    /// The span between knots `j` and `j + 1` of `points` (the knots' positions): the
    /// centripetal Catmull-Rom curve through the knot before, both and the knot after, a
    /// mirrored knot standing in past either end. A single knot is a point.
    pub(crate) fn span(points: &[[f32; 2]], j: usize) -> Self {
        let last = points.len().saturating_sub(1);
        let (p1, p2) = (points[j], points[(j + 1).min(last)]);
        let p0 = match j.checked_sub(1) {
            Some(before) => points[before],
            None => sub(scaled(p1, 2.0), p2),
        };
        let p3 = match points.get(j + 2) {
            Some(after) => *after,
            None => sub(scaled(p2, 2.0), p1),
        };
        Self::centripetal([p0, p1, p2, p3])
    }

    /// The centripetal Catmull-Rom span from `p[1]` to `p[2]` (Barry and Goldman's tangents,
    /// knot intervals the square roots of the distances), as a Bézier.
    pub(crate) fn centripetal([p0, p1, p2, p3]: [[f32; 2]; 4]) -> Self {
        let (a, b, c) = (
            len(sub(p1, p0)).sqrt(),
            len(sub(p2, p1)).sqrt(),
            len(sub(p3, p2)).sqrt(),
        );
        if b < NEAR {
            return Self([p1, p1, p2, p2]);
        }
        // Tangents times the span's own interval `b`.
        let start = if a < NEAR {
            [0.0; 2]
        } else {
            let along = sub(
                scaled(sub(p1, p0), 1.0 / a),
                scaled(sub(p2, p0), 1.0 / (a + b)),
            );
            scaled(add(along, scaled(sub(p2, p1), 1.0 / b)), b)
        };
        let end = if c < NEAR {
            [0.0; 2]
        } else {
            let along = sub(
                scaled(sub(p2, p1), 1.0 / b),
                scaled(sub(p3, p1), 1.0 / (b + c)),
            );
            scaled(add(along, scaled(sub(p3, p2), 1.0 / c)), b)
        };
        Self([
            p1,
            add(p1, scaled(start, 1.0 / 3.0)),
            sub(p2, scaled(end, 1.0 / 3.0)),
            p2,
        ])
    }

    /// The point at parameter `t` (Bernstein form, as the evaluator computes it).
    pub(crate) fn at(&self, t: f32) -> [f32; 2] {
        let [p0, p1, p2, p3] = self.0;
        let s = 1.0 - t;
        let weights = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
        [0, 1].map(|axis| {
            weights[0] * p0[axis]
                + weights[1] * p1[axis]
                + weights[2] * p2[axis]
                + weights[3] * p3[axis]
        })
    }

    /// At most how far the curve lies from its chord, in canvas pixels: the curve lies in the
    /// hull of its four points, and with both controls beside the chord its distance from the
    /// chord's line is at most 3/4 of theirs (`3t(1 - t)` times the larger).
    pub(crate) fn flatness(&self) -> f32 {
        let [a, c1, c2, b] = self.0;
        let chord = sub(b, a);
        let length = len(chord);
        if length < NEAR * NEAR {
            return len(sub(c1, a)).max(len(sub(c2, a)));
        }
        let along = |c: [f32; 2]| dot(sub(c, a), chord) / (length * length);
        let beside = |c: [f32; 2]| cross(sub(c, a), chord).abs() / length;
        if [c1, c2].iter().all(|c| (0.0..=1.0).contains(&along(*c))) {
            return 0.75 * beside(c1).max(beside(c2));
        }
        let off = |c: [f32; 2]| len(sub(c, add(a, scaled(chord, along(c).clamp(0.0, 1.0)))));
        off(c1).max(off(c2))
    }

    /// Three quarters of the larger second difference of the four points: `n` pieces of equal
    /// parameter lie within `bend / n^2` of the curve (the second derivative is at most six
    /// times that difference; a chord over `dt` errs by at most `|B''| dt^2 / 8`).
    pub(crate) fn bend(&self) -> f32 {
        let [p0, p1, p2, p3] = self.0;
        let first = add(sub(p0, scaled(p1, 2.0)), p2);
        let second = add(sub(p1, scaled(p2, 2.0)), p3);
        0.75 * len(first).max(len(second))
    }
}

/// How many straight pieces of equal parameter a span with `flatness` and `bend` (canvas pixels)
/// is evaluated as at `scale` output pixels per canvas pixel: one while its chord stays within
/// `TOLERANCE` output pixels, else enough that each piece does (at most `MAX_PIECES`). The
/// evaluator computes the same (`segment.wgsl`).
pub(crate) fn pieces(flatness: f32, bend: f32, scale: f32) -> u32 {
    if flatness * scale <= TOLERANCE {
        return 1;
    }
    let count = (bend * scale / TOLERANCE).sqrt().ceil();
    (count as u32).clamp(1, MAX_PIECES)
}

pub(crate) fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

pub(crate) fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

pub(crate) fn scaled(a: [f32; 2], k: f32) -> [f32; 2] {
    [a[0] * k, a[1] * k]
}

pub(crate) fn len(a: [f32; 2]) -> f32 {
    a[0].hypot(a[1])
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

#[cfg(test)]
#[path = "curve_tests.rs"]
pub(crate) mod tests;
