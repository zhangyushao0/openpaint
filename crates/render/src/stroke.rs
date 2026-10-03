//! Stroke geometry: a stroke is a chain of tapered capsules between consecutive samples
//! (DECISIONS D07: no dabs). At every sample the stroke's recipe turns pressure and pen speed
//! into a radius (size dynamics) and a flow (opacity dynamics); the paint op carries the colour,
//! the stroke's opacity and the recipe's grain.
//!
//! Everything here depends only on the samples up to the one it describes, so a stroke drawn a
//! few samples at a time gets exactly the segments it gets when drawn at once.

use openpaint_core::{StrokeSample, StrokeStyle};

use crate::geom::{Bounds, PixelRect};
use crate::protocol::{GRAIN_ONE, Op, Paint};

/// Antialiasing reaches half a pixel beyond the shape; one pixel keeps the bounds conservative
/// against float error.
const REACH: f32 = 1.0;
/// The pen speed at a sample is the length of the path drawn in this many milliseconds before
/// it, divided by them: it rises smoothly from 0 as a stroke starts.
const SPEED_WINDOW_MS: u32 = 40;
/// Most segments looked back over for the speed, since samples can share a millisecond.
const SPEED_LOOKBACK: usize = 64;

/// A tapered capsule between two samples, in target pixels.
pub(crate) struct Segment {
    a: [f32; 2],
    b: [f32; 2],
    radii: [f32; 2],
    flows: [f32; 2],
    /// One over the width the coverage fades in over, in pixels.
    edge: f32,
}

impl Segment {
    pub(crate) fn op(&self) -> Op {
        Op::segment(self.a, self.b, self.radii, self.flows, self.edge)
    }

    /// Coverage is zero from half a pixel outside the capsule, whatever its edge.
    pub(crate) fn bounds(&self) -> Bounds {
        let reach = self.radii[0].max(self.radii[1]) + REACH;
        Bounds([
            self.a[0].min(self.b[0]) - reach,
            self.a[1].min(self.b[1]) - reach,
            self.a[0].max(self.b[0]) + reach,
            self.a[1].max(self.b[1]) + reach,
        ])
    }
}

/// The segments ending at samples after `from`: one per consecutive pair from `(from, from +
/// 1)` on; a stroke of one sample is a round dot. Samples before `from` only feed the speed.
pub(crate) fn segments<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
    from: usize,
) -> impl Iterator<Item = Segment> + 'a {
    let speed = style.recipe().uses_speed();
    let pairs = (from + 1..samples.len()).map(|i| (i - 1, i));
    let dot = (samples.len() == 1 && from == 0).then_some((0, 0));
    pairs.chain(dot).map(move |(i, j)| {
        let (ra, fa) = dab(style, samples, i, speed);
        let (rb, fb) = dab(style, samples, j, speed);
        let width = style.recipe().softness() * ra.max(rb);
        Segment {
            a: point(&samples[i]),
            b: point(&samples[j]),
            radii: [ra, rb],
            flows: [fa, fb],
            edge: 1.0 / width.max(1.0),
        }
    })
}

/// The paint op's operands for `style`.
pub(crate) fn paint(style: &StrokeStyle) -> Paint {
    let color = style.color();
    let grain = style.recipe().grain();
    Paint {
        color: [color.r(), color.g(), color.b()],
        opacity: style.opacity(),
        grain: (grain.strength() * GRAIN_ONE as f32).round() as u32,
        grain_scale: grain.scale(),
    }
}

/// The pixels a stroke of `style` through `samples` can change with its segments after `from`
/// (see `segments`). For an in-progress stroke, pass every sample so far and the index of the
/// last one evaluated before: the result is the region to evaluate again.
#[must_use]
pub fn stroke_region(
    style: &StrokeStyle,
    samples: &[StrokeSample],
    from: usize,
) -> Option<PixelRect> {
    segment_regions(style, samples, from).reduce(|all, next| all.union(&next))
}

/// The pixels each segment can change, one rectangle per segment: tighter than their union for
/// a long curved stroke.
pub(crate) fn segment_regions<'a>(
    style: &'a StrokeStyle,
    samples: &'a [StrokeSample],
    from: usize,
) -> impl Iterator<Item = PixelRect> + 'a {
    segments(style, samples, from).map(|segment| segment.bounds().pixels())
}

/// The radius and the flow at sample `i`.
fn dab(style: &StrokeStyle, samples: &[StrokeSample], i: usize, speed: bool) -> (f32, f32) {
    let pressure = samples[i].pressure();
    let speed = if speed { pen_speed(samples, i) } else { 0.0 };
    let recipe = style.recipe();
    let radius = 0.5 * style.size() * recipe.size().at(pressure, speed);
    (radius, recipe.opacity().at(pressure, speed))
}

/// Canvas pixels per millisecond at sample `i`: the path of the last `SPEED_WINDOW_MS` before
/// it (a segment reaching back further counts with the part inside the window), divided by
/// the window.
fn pen_speed(samples: &[StrokeSample], i: usize) -> f32 {
    let end = samples[i].time_ms();
    let mut path = 0.0;
    let mut k = i;
    while k > 0 && i - k < SPEED_LOOKBACK {
        let (before, after) = (&samples[k - 1], &samples[k]);
        let length = distance(before, after);
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

fn distance(a: &StrokeSample, b: &StrokeSample) -> f32 {
    let [ax, ay] = point(a);
    let [bx, by] = point(b);
    (bx - ax).hypot(by - ay)
}

fn point(sample: &StrokeSample) -> [f32; 2] {
    let position = sample.position();
    [position.x(), position.y()]
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use openpaint_core::{BrushRecipe, CanvasPoint, Color, Curve, Dynamics, Grain};

    fn sample(x: f32, pressure: f32, time_ms: u32) -> StrokeSample {
        let position = CanvasPoint::new(x, 20.0).expect("point");
        StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
    }

    fn style(recipe: BrushRecipe) -> StrokeStyle {
        StrokeStyle::new(Arc::new(recipe), 10.0, 1.0, Color::BLACK).expect("style")
    }

    /// Thinner when faster: half the size from 2 px/ms on.
    fn speedy() -> BrushRecipe {
        let curve = |points: &[[f32; 2]]| Curve::new(points).expect("curve");
        let size = Dynamics::new(curve(&[[0.0, 1.0]]), curve(&[[0.0, 1.0], [2.0, 0.5]]));
        let opacity = Dynamics::new(curve(&[[0.0, 1.0]]), curve(&[[0.0, 1.0]]));
        BrushRecipe::new(
            0.5,
            size.expect("size"),
            opacity.expect("opacity"),
            Grain::NONE,
        )
        .expect("recipe")
    }

    #[test]
    fn the_plain_brush_keeps_the_round_brush_before_recipes() {
        let samples = [sample(10.0, 1.0, 0), sample(30.0, 0.2, 4)];
        let segment = segments(&style(BrushRecipe::plain()), &samples, 0)
            .next()
            .expect("one segment");
        assert_eq!(segment.radii, [0.5 * 10.0 * 1.0, 0.5 * 10.0 * 0.2]);
        assert_eq!((segment.flows, segment.edge), ([1.0, 1.0], 1.0));
    }

    #[test]
    fn speed_is_the_path_of_the_last_window_and_rises_from_zero() {
        // 1 px per ms, a sample every 4 ms.
        let samples: Vec<_> = (0..30u16)
            .map(|i| sample(4.0 * f32::from(i), 1.0, 4 * u32::from(i)))
            .collect();
        assert_eq!(pen_speed(&samples, 0), 0.0);
        assert!(
            (pen_speed(&samples, 5) - 0.5).abs() < 1e-6,
            "20 px in 40 ms"
        );
        for i in 10..30 {
            assert!((pen_speed(&samples, i) - 1.0).abs() < 1e-5, "at {i}");
        }
        // A sample straddling the window counts with the part inside it.
        let uneven = [
            sample(0.0, 1.0, 0),
            sample(30.0, 1.0, 30),
            sample(40.0, 1.0, 50),
        ];
        assert!((pen_speed(&uneven, 2) - 30.0 / 40.0).abs() < 1e-6);
    }

    #[test]
    fn segments_after_from_equal_the_same_segments_of_the_whole_stroke() {
        let style = style(speedy());
        let samples: Vec<_> = (0..40u16)
            .map(|i| sample(3.0 * f32::from(i), 0.8, 2 * u32::from(i)))
            .collect();
        let whole: Vec<Op> = segments(&style, &samples, 0).map(|s| s.op()).collect();
        let tail: Vec<Op> = segments(&style, &samples, 25).map(|s| s.op()).collect();
        assert_eq!(tail, whole[25..]);
        let thin = segments(&style, &samples, 30).next().expect("segment");
        assert!(thin.radii[0] < 5.0, "faster is thinner: {:?}", thin.radii);
        assert!((thin.edge - 1.0 / (0.5 * thin.radii[0].max(thin.radii[1]))).abs() < 1e-6);
    }

    #[test]
    fn region_covers_the_widest_radius_plus_antialiasing() {
        let style = style(BrushRecipe::plain());
        let samples = [sample(10.0, 1.0, 0), sample(30.0, 0.2, 4)];
        let region = stroke_region(&style, &samples, 0).expect("region");
        assert_eq!(region.min, [4, 14]);
        assert_eq!(region.max, [37, 27]);
        assert_eq!(stroke_region(&style, &[], 0), None);
        assert_eq!(stroke_region(&style, &samples, 1), None);
        assert!(stroke_region(&style, &samples[..1], 0).is_some(), "a dot");
    }
}
