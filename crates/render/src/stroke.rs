//! Stroke geometry: a stroke follows the centripetal Catmull-Rom curve through its knots
//! (`curve.rs`, S34), one `Segment` op per span between two knots, which the evaluator flattens
//! into tapered capsules fine enough for the scale it evaluates at (DECISIONS D07: no dabs). At
//! every knot the stroke's recipe turns pressure and pen speed into a radius (size dynamics), a
//! flow (opacity dynamics, times the recipe's flow when it accumulates) and a paper contact,
//! which change evenly along the span; the paint op carries the colour, the stroke's opacity,
//! the recipe's grain and how its density becomes coverage (D38).
//!
//! A span depends on the knots before and after it and a knot on the sample after it, so an
//! in-progress stroke's last two spans may still change (`Path::settled`); every earlier span
//! is exactly the span the finished stroke has.

use std::ops::Range;

use openpaint_core::{
    Deposit, StrokeBlend, StrokeSample, StrokeStyle, StrokeTool, Tip as RecipeTip,
};

use crate::curve::{self, Cubic, knots, point};
use crate::geom::{Bounds, PixelRect};
use crate::protocol::{Blend, GRAIN_ONE, Op, Paint, SegmentOperands, Tip};
use crate::smudge::smudge_reach;

/// Antialiasing reaches half a pixel beyond the shape; one pixel keeps the bounds conservative
/// against float error.
const REACH: f32 = 1.0;
/// The pen speed at a sample is the length of the path drawn in this many milliseconds before
/// it, divided by them: it rises smoothly from 0 as a stroke starts.
const SPEED_WINDOW_MS: u32 = 40;
/// Most segments looked back over for the speed, since samples can share a millisecond.
const SPEED_LOOKBACK: usize = 64;

/// One span of a stroke's curve between two knots, in canvas pixels.
#[derive(Debug, Clone)]
pub(crate) struct Segment {
    operands: SegmentOperands,
    /// The pen's pressure at both knots (a smudge's strength follows it).
    pressures: [f32; 2],
}

/// One straight piece of a flattened span: what the evaluator draws as a capsule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Piece {
    pub(crate) a: [f32; 2],
    pub(crate) b: [f32; 2],
    pub(crate) radii: [f32; 2],
    pub(crate) pressures: [f32; 2],
}

impl Segment {
    pub(crate) fn op(&self) -> Op {
        Op::segment(&self.operands)
    }

    /// As the in-progress stroke's end, which later samples may still change.
    pub(crate) fn tail_op(&self) -> Op {
        Op::tail(&self.operands)
    }

    /// Coverage and density are zero from half a pixel outside every capsule it is flattened
    /// into, whatever its edge or tip: the curve lies in the hull of its four points.
    pub(crate) fn bounds(&self) -> Bounds {
        let [ra, rb] = self.operands.radii;
        self.hull(ra.max(rb) + REACH)
    }

    /// The box of the curve's four points widened by `reach` on every side.
    pub(crate) fn hull(&self, reach: f32) -> Bounds {
        let SegmentOperands { a, b, controls, .. } = self.operands;
        let points = [a, controls[0], controls[1], b];
        let low = |axis: usize| points.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
        let high = |axis: usize| {
            points
                .iter()
                .map(|p| p[axis])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        Bounds([
            low(0) - reach,
            low(1) - reach,
            high(0) + reach,
            high(1) + reach,
        ])
    }

    /// The radii at both knots.
    pub(crate) fn radii(&self) -> [f32; 2] {
        self.operands.radii
    }

    /// The straight pieces the evaluator draws at `scale` output pixels per canvas pixel
    /// (`segment.wgsl`): the chord while it is close enough, else pieces of equal parameter.
    pub(crate) fn pieces(&self, scale: f32) -> Vec<Piece> {
        let s = &self.operands;
        let n = curve::pieces(s.flat, s.bend, scale);
        let cubic = Cubic([s.a, s.controls[0], s.controls[1], s.b]);
        let lerp = |[x, y]: [f32; 2], t: f32| x + (y - x) * t;
        let at = |k: u32| {
            let t = k as f32 / n as f32;
            let place = match k {
                0 => s.a,
                k if k == n => s.b,
                _ => cubic.at(t),
            };
            (place, lerp(s.radii, t), lerp(self.pressures, t))
        };
        (0..n)
            .map(|k| {
                let (a, ra, pa) = at(k);
                let (b, rb, pb) = at(k + 1);
                Piece {
                    a,
                    b,
                    radii: [ra, rb],
                    pressures: [pa, pb],
                }
            })
            .collect()
    }
}

/// A stroke's knots (`curve::knots`) and what follows from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Path {
    knots: Vec<usize>,
}

impl Path {
    pub(crate) fn of(samples: &[StrokeSample]) -> Self {
        Self {
            knots: knots(samples),
        }
    }

    /// Its spans: one between each two consecutive knots; a lone knot is one, a dot.
    pub(crate) fn spans(&self) -> usize {
        self.knots
            .len()
            .saturating_sub(1)
            .max(usize::from(!self.knots.is_empty()))
    }

    /// The spans no later sample can change while the stroke is drawn: all but the last two,
    /// which depend on the last sample (a knot until the next one shows whether it is one).
    pub(crate) fn settled(&self) -> usize {
        self.knots.len().saturating_sub(3)
    }

    /// The first span that samples from `from` on can change: a sample decides whether the
    /// one before it is a knot, and a span depends on the knot before and after it.
    pub(crate) fn changed_from(&self, from: usize) -> usize {
        let before = self.knots.iter().take_while(|k| **k + 1 < from).count();
        before.saturating_sub(2).min(self.spans())
    }
}

/// The spans `range` of a stroke of `style` through `samples`, whose knots are `path`.
pub(crate) fn spans<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
    path: &'a Path,
    range: Range<usize>,
) -> impl Iterator<Item = Segment> + 'a {
    range.map(move |j| span(style, samples, path, j))
}

/// Every span of a finished stroke; a stroke of one sample is a round dot.
pub(crate) fn segments<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
) -> impl Iterator<Item = Segment> + 'a {
    let path = Path::of(samples);
    (0..path.spans()).map(move |j| span(style, samples, &path, j))
}

/// Span `j` of the stroke.
fn span(style: &StrokeStyle, samples: &[StrokeSample], path: &Path, j: usize) -> Segment {
    let tip = match style.recipe().deposit() {
        Deposit::Cover => Tip::Cover,
        Deposit::Accumulate(accumulation) => match accumulation.tip() {
            RecipeTip::Flat => Tip::Flat,
            RecipeTip::Bell => Tip::Bell,
        },
    };
    let speed = style.recipe().uses_speed();
    let knots = &path.knots;
    let last = knots.len() - 1;
    let (i, k) = (knots[j], knots[(j + 1).min(last)]);
    // The knots the span depends on, as `Cubic::span` takes them.
    let first = j.saturating_sub(1);
    let mut local = [[0.0; 2]; 4];
    let count = (first..=(j + 2).min(last)).count();
    for (slot, knot) in local.iter_mut().zip(&knots[first..first + count]) {
        *slot = point(&samples[*knot]);
    }
    let cubic = Cubic::span(&local[..count], j - first);
    let (a, b) = (dab(style, samples, i, speed), dab(style, samples, k, speed));
    let width = style.recipe().softness() * a.radius.max(b.radius);
    let [start, c1, c2, end] = cubic.0;
    Segment {
        operands: SegmentOperands {
            a: start,
            b: end,
            radii: [a.radius, b.radius],
            flows: [a.flow, b.flow],
            edge: 1.0 / width.max(1.0),
            contacts: [a.contact, b.contact],
            tip,
            controls: [c1, c2],
            flat: cubic.flatness(),
            bend: cubic.bend(),
        },
        pressures: [samples[i].pressure(), samples[k].pressure()],
    }
}

/// The paint op's operands for a stroke of `style` combining with its layer by `blend`.
pub(crate) fn paint(style: &StrokeStyle, blend: StrokeBlend) -> Paint {
    let color = style.color();
    let grain = style.recipe().grain();
    let (density, paper) = match style.recipe().deposit() {
        Deposit::Cover => (0.0, [0.0; 4]),
        Deposit::Accumulate(accumulation) => {
            let paper = accumulation.paper().map_or([0.0; 4], |paper| {
                let level = paper.scale().log2().max(0.0);
                [paper.scale(), level, paper.depth(), paper.dust()]
            });
            (accumulation.max_density(), paper)
        }
    };
    Paint {
        color: [color.r(), color.g(), color.b()],
        opacity: style.opacity(),
        grain: (grain.strength() * GRAIN_ONE as f32).round() as u32,
        grain_scale: grain.scale(),
        blend: match blend {
            // A smudge paints nothing: it is drawn by its residual (`smudge`), never by a paint
            // op.
            StrokeBlend::Over | StrokeBlend::Smudge | StrokeBlend::SmudgeLocked => Blend::Over,
            StrokeBlend::Atop => Blend::Atop,
            StrokeBlend::Erase => Blend::Erase,
        },
        density,
        paper,
        clip: 0,
    }
}

/// The pixels a stroke of `style` through `samples` can change with the spans that samples
/// from `from` on can change (`Path::changed_from`): every span for `from` 0. For two versions
/// of a stroke sharing their samples before `from`, everything else they draw is the same.
#[must_use]
pub fn stroke_region(
    style: &StrokeStyle,
    samples: &[StrokeSample],
    from: usize,
) -> Option<PixelRect> {
    let path = Path::of(samples);
    let range = path.changed_from(from)..path.spans();
    span_regions(style, samples, &path, range).reduce(|all, next| all.union(&next))
}

/// How many spans a finished stroke of `style` through `samples` has, and how many straight
/// pieces the evaluator draws them as at `scale` output pixels per canvas pixel (statistics).
#[must_use]
pub fn curve_pieces(style: &StrokeStyle, samples: &[StrokeSample], scale: f32) -> [usize; 2] {
    let spans: Vec<Segment> = segments(style, samples).collect();
    let pieces = spans.iter().map(|span| span.pieces(scale).len()).sum();
    [spans.len(), pieces]
}

/// The radius in canvas pixels at the last of `samples`, exactly as a stroke of `style` draws
/// it there: the size dynamics at that sample's pressure and pen speed. `None` without samples.
#[must_use]
pub fn tip_radius(style: &StrokeStyle, samples: &[StrokeSample]) -> Option<f32> {
    let last = samples.len().checked_sub(1)?;
    Some(dab(style, samples, last, style.recipe().uses_speed()).radius)
}

/// The pixels each span of a finished stroke can change, one rectangle per span: tighter than
/// their union for a long curved stroke. A smudge's spans reach as far as its contacts can.
pub(crate) fn segment_regions<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
) -> impl Iterator<Item = PixelRect> + 'a {
    let path = Path::of(samples);
    let count = path.spans();
    let regions: Vec<PixelRect> = span_regions(style, samples, &path, 0..count).collect();
    regions.into_iter()
}

/// `segment_regions` of the spans `range`.
pub(crate) fn span_regions<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
    path: &'a Path,
    range: Range<usize>,
) -> Box<dyn Iterator<Item = PixelRect> + 'a> {
    if style.tool() == StrokeTool::Smudge {
        let reach = smudge_reach(style, samples, path, range);
        return Box::new(reach.map(|bounds| bounds.pixels()));
    }
    Box::new(spans(style, samples, path, range).map(|segment| segment.bounds().pixels()))
}

/// What a stroke is at one sample.
struct Dab {
    radius: f32,
    /// A covering stroke's coverage factor; an accumulating one's density per radius.
    flow: f32,
    /// The share of the paper the tip touches; 0 without paper.
    contact: f32,
}

/// The stroke at sample `i`.
fn dab(style: &StrokeStyle, samples: &[StrokeSample], i: usize, speed: bool) -> Dab {
    let pressure = samples[i].pressure();
    let speed = if speed { pen_speed(samples, i) } else { 0.0 };
    let recipe = style.recipe();
    let radius = 0.5 * style.size() * recipe.size().at(pressure, speed);
    let opacity = recipe.opacity().at(pressure, speed);
    match recipe.deposit() {
        Deposit::Cover => Dab {
            radius,
            flow: opacity,
            contact: 0.0,
        },
        Deposit::Accumulate(accumulation) => Dab {
            radius,
            flow: opacity * accumulation.flow(),
            contact: accumulation
                .paper()
                .map_or(0.0, |paper| paper.contact().at(pressure)),
        },
    }
}

/// Canvas pixels per millisecond at sample `i`: the path of the last `SPEED_WINDOW_MS` before
/// it (a segment reaching back further counts with the part inside the window), divided by
/// the window. Measured along the samples themselves, resamples included.
fn pen_speed(samples: &[StrokeSample], i: usize) -> f32 {
    let end = samples[i].time_ms();
    let mut path = 0.0;
    let mut k = i;
    while k > 0 && i - k < SPEED_LOOKBACK {
        let (before, after) = (&samples[k - 1], &samples[k]);
        let length = curve::len(curve::sub(point(after), point(before)));
        if end - before.time_ms() <= SPEED_WINDOW_MS {
            path += length;
            k -= 1;
            continue;
        }
        let span = after.time_ms() - before.time_ms();
        let inside = SPEED_WINDOW_MS - (end - after.time_ms());
        path += length * inside as f32 / span as f32;
        break;
    }
    path / SPEED_WINDOW_MS as f32
}

#[cfg(test)]
#[path = "stroke_tests.rs"]
mod tests;
