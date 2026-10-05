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
    let whole: Vec<Op> = segments(&style, &samples).map(|s| s.op()).collect();
    for end in 1..=samples.len() {
        let path = Path::of(&samples[..end]);
        let settled: Vec<Op> = spans(&style, &samples[..end], &path, 0..path.settled())
            .map(|s| s.op())
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
