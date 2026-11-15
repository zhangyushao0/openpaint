use std::sync::Arc;

use super::*;
use crate::curve::tests::{FAST_TURNS, samples as fixture};
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
    let segment = segments(&style(BrushRecipe::plain()), &samples)
        .next()
        .expect("one segment")
        .operands;
    assert_eq!((segment.a, segment.b), ([10.0, 20.0], [30.0, 20.0]));
    assert_eq!(segment.radii, [0.5 * 10.0 * 1.0, 0.5 * 10.0 * 0.2]);
    assert_eq!((segment.flows, segment.edge), ([1.0, 1.0], 1.0));
    assert_eq!((segment.contacts, segment.tip), ([0.0; 2], Tip::Cover));
    assert!(segment.flat * 8.0 <= curve::TOLERANCE, "a straight span");
    let drawn = segment.radii[1];
    assert_eq!(
        tip_radius(&style(BrushRecipe::plain()), &samples),
        Some(drawn)
    );
    assert_eq!(tip_radius(&style(BrushRecipe::plain()), &[]), None);
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
fn the_settled_spans_of_a_stroke_being_drawn_are_the_finished_strokes() {
    let style = style(speedy());
    let samples: Vec<StrokeSample> = FAST_TURNS.iter().flat_map(|rows| fixture(rows)).collect();
    // One stroke through the four windows, its times rising.
    let samples: Vec<StrokeSample> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| StrokeSample::new(s.position(), s.pressure(), 0.0, 0.0, 3 * i as u32))
        .collect::<Result<_, _>>()
        .expect("samples");
    let whole: Vec<Op> = segments(&style, &samples).map(|s| s.op(1)).collect();
    for end in 1..=samples.len() {
        let path = Path::of(&samples[..end]);
        let settled: Vec<Op> = spans(&style, &samples[..end], &path, 0..path.settled())
            .map(|s| s.op(1))
            .collect();
        assert_eq!(settled, whole[..settled.len()], "after {end} samples");
        assert!(path.spans() - path.settled() <= 2);
    }
    let thin = segments(&style, &samples)
        .nth(whole.len() / 2)
        .expect("segment")
        .operands;
    assert!(thin.radii[0] < 5.0, "faster is thinner: {:?}", thin.radii);
    assert!((thin.edge - 1.0 / (0.5 * thin.radii[0].max(thin.radii[1]))).abs() < 1e-6);
}

#[test]
fn dynamics_change_evenly_along_the_path_and_meet_at_knots() {
    let style = style(BrushRecipe::plain());
    let samples = fixture(&FAST_TURNS[1]);
    let all: Vec<Segment> = segments(&style, &samples).collect();
    for pair in all.windows(2) {
        assert_eq!(pair[0].operands.b, pair[1].operands.a);
        assert_eq!(pair[0].radii()[1], pair[1].radii()[0]);
        assert_eq!(pair[0].operands.flows[1], pair[1].operands.flows[0]);
    }
    for segment in &all {
        for scale in [1.0, 8.0] {
            let pieces = segment.pieces(scale);
            assert_eq!(pieces[0].a, segment.operands.a);
            assert_eq!(pieces[pieces.len() - 1].b, segment.operands.b);
            for pair in pieces.windows(2) {
                assert_eq!(pair[0].b, pair[1].a);
                assert_eq!(pair[0].radii[1], pair[1].radii[0]);
            }
            let [ra, rb] = segment.radii();
            for piece in &pieces {
                let within = |r: f32| r >= ra.min(rb) - 1e-5 && r <= ra.max(rb) + 1e-5;
                assert!(within(piece.radii[0]) && within(piece.radii[1]));
                // Every piece lies in the span's bounds.
                let Bounds([x0, y0, x1, y1]) = segment.bounds();
                for p in [piece.a, piece.b] {
                    assert!(p[0] >= x0 && p[0] <= x1 && p[1] >= y0 && p[1] <= y1);
                }
            }
        }
        assert!(segment.pieces(8.0).len() >= segment.pieces(1.0).len());
    }
}

/// One width at any pressure (a monoline), with `taper`.
fn tapered(start: f32, end: f32, tip: f32) -> StrokeStyle {
    let flat = || Curve::new(&[[0.0, 1.0]]).expect("curve");
    let dynamics = || Dynamics::new(flat(), flat()).expect("dynamics");
    let recipe = BrushRecipe::new(0.0, dynamics(), dynamics(), Grain::NONE).expect("recipe");
    let taper = openpaint_core::Taper::new(start, end, tip).expect("taper");
    style(recipe.with_taper(taper))
}

/// A straight line along x, a sample every `step` px, `count` samples, each a knot: the
/// pressure (which sets no width here) falls a little from sample to sample.
fn line(count: u16, step: f32) -> Vec<StrokeSample> {
    (0..count)
        .map(|i| {
            let pressure = 1.0 - 0.001 * f32::from(i);
            sample(5.0 + step * f32::from(i), pressure, 4 * u32::from(i))
        })
        .collect()
}

#[test]
fn the_taper_narrows_the_knots_near_both_ends_of_a_finished_stroke_only() {
    // Size 10: two diameters (20 px) at the start, four (40 px) at the end, tip a tenth; the
    // line is 80 px long.
    let style = tapered(2.0, 4.0, 0.1);
    let samples = line(41, 2.0);
    let finished: Vec<Segment> = segments(&style, &samples).collect();
    let radius = |k: usize| {
        let span = &finished[k.min(finished.len() - 1)];
        if k < finished.len() {
            span.radii()[0]
        } else {
            span.radii()[1]
        }
    };
    let full = 5.0;
    assert!((radius(0) - 0.1 * full).abs() < 1e-6, "the start's tip");
    assert!((radius(40) - 0.1 * full).abs() < 1e-6, "the end's tip");
    // 10 px in: halfway along the start's 20 px; 20 px before the end: halfway along its 40.
    assert!((radius(5) - 0.55 * full).abs() < 1e-5);
    assert!((radius(30) - 0.55 * full).abs() < 1e-5);
    for k in 10..=20 {
        assert_eq!(radius(k), full, "knot {k}: the middle untouched");
    }
    // Drawn, the end is blunt: the in-progress spans are the finished ones but the end's.
    let path = Path::of(&samples);
    let drawn: Vec<Segment> = spans(&style, &samples, &path, 0..path.spans()).collect();
    let first_changed = path.ending_within(40.0);
    assert_eq!(
        first_changed, 20,
        "the span ending at the first knot within 40 px of the end"
    );
    for (j, (now, before)) in finished.iter().zip(&drawn).enumerate() {
        if j < first_changed {
            assert_eq!(now.op(1), before.op(1), "span {j}");
        } else {
            assert!(
                now.radii()[1] < before.radii()[1],
                "span {j} narrows at the lift"
            );
        }
    }
    assert_eq!(drawn[drawn.len() - 1].radii()[1], full, "blunt");
    // What the lift redraws covers every narrowed span as drawn and as finished.
    let lifted = lift_region(&style, &samples).expect("an end taper");
    for span in &drawn[first_changed..] {
        let pixels = span.bounds().pixels();
        assert_eq!(lifted.union(&pixels), lifted, "{pixels:?}");
    }
    assert_eq!(
        lift_region(&tapered(2.0, 0.0, 0.1), &samples),
        None,
        "no end taper"
    );
    let smudge = tapered(2.0, 3.0, 0.1).with_tool(StrokeTool::Smudge);
    assert_eq!(
        lift_region(&smudge, &samples),
        None,
        "a smudge's end is never tapered"
    );
}

#[test]
fn a_dot_is_never_tapered_and_the_outline_ignores_the_taper() {
    let style = tapered(2.0, 3.0, 0.0);
    let dot = [sample(20.0, 1.0, 0)];
    let span = segments(&style, &dot).next().expect("a dot").operands;
    assert_eq!(span.radii, [5.0, 5.0]);
    let samples = line(41, 2.0);
    assert_eq!(
        tip_radius(&style, &samples),
        Some(5.0),
        "the brush's outline"
    );
}

#[test]
fn tilt_scales_size_and_opacity_exactly_at_the_knots() {
    let curve = |points: &[[f32; 2]]| Curve::new(points).expect("curve");
    let wider = Curve::gain(&[[0.0, 1.0], [40.0, 1.0], [80.0, 3.0]]).expect("gain");
    let size = Dynamics::new(curve(&[[0.0, 0.3], [1.0, 1.0]]), curve(&[[0.0, 1.0]]))
        .and_then(|size| size.with_tilt(wider))
        .expect("size");
    let lighter = curve(&[[0.0, 1.0], [40.0, 1.0], [80.0, 0.4]]);
    let opacity = Dynamics::new(curve(&[[0.0, 1.0]]), curve(&[[0.0, 1.0]]))
        .and_then(|opacity| opacity.with_tilt(lighter))
        .expect("opacity");
    let recipe = BrushRecipe::new(0.0, size, opacity, Grain::NONE).expect("recipe");
    let style = style(recipe);
    // Off one line, so the middle sample is a knot.
    let tilted = |[x, y]: [f32; 2], degrees: f32, time_ms: u32| {
        let position = CanvasPoint::new(x, y).expect("point");
        let tilt = degrees.to_radians();
        StrokeSample::new(position, 0.5, tilt, 0.0, time_ms).expect("sample")
    };
    let samples = [
        tilted([10.0, 20.0], 0.0, 0),
        tilted([30.0, 26.0], 30.0, 4),
        tilted([50.0, 20.0], 60.0, 8),
    ];
    let all: Vec<Segment> = segments(&style, &samples).collect();
    let recipe = style.recipe();
    for (k, sample) in samples.iter().enumerate() {
        let (span, end) = if k < all.len() {
            (&all[k], 0)
        } else {
            (&all[k - 1], 1)
        };
        let factor = recipe.size().at(0.5, 0.0, sample.tilt());
        assert_eq!(span.radii()[end], 0.5 * 10.0 * factor, "knot {k}");
        let flow = recipe.opacity().at(0.5, 0.0, sample.tilt());
        assert_eq!(span.operands.flows[end], flow, "knot {k}");
    }
    // Upright and at 30 degrees as without tilt; at 60 twice as wide and 0.7 as strong.
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].radii()[0], all[0].radii()[1]);
    assert!((all[0].radii()[0] - 0.5 * 10.0 * 0.65).abs() < 1e-5);
    assert!((all[1].radii()[1] - 2.0 * 0.5 * 10.0 * 0.65).abs() < 1e-5);
    assert!((all[1].operands.flows[1] - 0.7).abs() < 1e-5);
}

#[test]
fn region_covers_the_widest_radius_plus_antialiasing() {
    let style = style(BrushRecipe::plain());
    let samples = [sample(10.0, 1.0, 0), sample(30.0, 0.2, 4)];
    let region = stroke_region(&style, &samples, 0).expect("region");
    assert_eq!(region.min, [4, 14]);
    assert_eq!(region.max, [37, 27]);
    assert_eq!(stroke_region(&style, &[], 0), None);
    assert!(stroke_region(&style, &samples[..1], 0).is_some(), "a dot");
    // Samples from the fifth on change the spans from the knot two before the fourth on.
    let turns = fixture(&FAST_TURNS[0]);
    let path = Path::of(&turns);
    let from = path.changed_from(5);
    assert!(
        from < path.spans() && from <= 2,
        "{from} of {}",
        path.spans()
    );
    assert_eq!(path.changed_from(0), 0);
}
