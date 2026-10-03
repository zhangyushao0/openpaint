//! Brush recipes: curves, dynamics and grain accept their ranges and refuse what lies outside;
//! the plain round brush gives back pressure exactly; stored recipes round-trip and are shared.

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError, Curve, Dynamics, Grain,
    MAX_CURVE_POINTS, MAX_GRAIN_SCALE, MAX_SPEED, StrokeSample, StrokeStyle,
};

fn invalid(what: &'static str) -> CoreError {
    CoreError::InvalidValue { what }
}

fn curve(points: &[[f32; 2]]) -> Curve {
    Curve::new(points).unwrap()
}

#[test]
fn curves_join_their_points_with_straight_lines_and_stay_flat_outside() {
    let ramp = curve(&[[0.2, 0.1], [0.6, 0.9], [1.0, 0.5]]);
    assert_eq!(ramp.at(0.0), 0.1);
    assert_eq!(ramp.at(0.2), 0.1);
    assert!((ramp.at(0.4) - 0.5).abs() < 1e-6);
    assert_eq!(ramp.at(0.6), 0.9);
    assert!((ramp.at(0.8) - 0.7).abs() < 1e-6);
    assert_eq!(ramp.at(5.0), 0.5);
    assert!(!ramp.is_flat());
    assert!(curve(&[[0.0, 0.4], [3.0, 0.4]]).is_flat());
}

#[test]
fn the_plain_round_brush_gives_back_pressure_exactly() {
    let plain = BrushRecipe::plain();
    for step in 0..=1000u16 {
        let pressure = f32::from(step) / 1000.0;
        let size = plain.size().at(pressure, 3.0);
        assert_eq!(size.to_bits(), pressure.to_bits(), "at {pressure}");
        assert_eq!(plain.opacity().at(pressure, 3.0), 1.0);
    }
    assert_eq!(plain.softness(), 0.0);
    assert_eq!(plain.grain(), Grain::NONE);
    assert!(!plain.uses_speed());
}

#[test]
fn curves_refuse_bad_points() {
    let too_many = vec![[0.0, 0.5]; MAX_CURVE_POINTS + 1];
    let cases: [&[[f32; 2]]; 6] = [
        &[],
        &too_many,
        &[[0.5, 0.1], [0.5, 0.2]],
        &[[0.0, 1.5]],
        &[[-0.1, 0.5]],
        &[[f32::NAN, 0.5]],
    ];
    for points in cases {
        assert_eq!(
            Curve::new(points),
            Err(invalid("brush curve")),
            "{points:?}"
        );
    }
}

#[test]
fn dynamics_grain_and_softness_have_ranges() {
    let flat = || curve(&[[0.0, 1.0]]);
    let speed = curve(&[[0.0, 1.0], [MAX_SPEED, 0.5]]);
    assert!(Dynamics::new(flat(), speed).is_ok());
    let fast = curve(&[[0.0, 1.0], [MAX_SPEED + 1.0, 0.5]]);
    assert_eq!(Dynamics::new(flat(), fast), Err(invalid("brush dynamics")));
    let deep = curve(&[[0.0, 1.0], [1.5, 0.5]]);
    assert_eq!(Dynamics::new(deep, flat()), Err(invalid("brush dynamics")));
    assert!(Grain::new(1.0, MAX_GRAIN_SCALE).is_ok());
    for (strength, scale) in [(1.1, 1), (0.5, 0), (0.5, MAX_GRAIN_SCALE + 1)] {
        assert_eq!(Grain::new(strength, scale), Err(invalid("brush grain")));
    }
    let dynamics = || Dynamics::new(flat(), flat()).unwrap();
    let soft = BrushRecipe::new(1.2, dynamics(), dynamics(), Grain::NONE);
    assert_eq!(soft, Err(invalid("brush softness")));
}

#[test]
fn a_recipe_with_speed_says_so_and_round_trips_through_parts() {
    let speed = curve(&[[0.0, 1.0], [4.0, 0.6]]);
    let size = Dynamics::new(curve(&[[0.0, 0.3], [1.0, 1.0]]), speed).unwrap();
    let opacity = Dynamics::new(curve(&[[0.0, 1.0]]), curve(&[[0.0, 1.0]])).unwrap();
    let recipe = BrushRecipe::new(0.5, size, opacity, Grain::new(0.4, 3).unwrap()).unwrap();
    assert!(recipe.uses_speed());
    assert_eq!(
        BrushRecipe::from_parts(&recipe.to_parts()),
        Ok(recipe.clone())
    );
    let mut broken = recipe.to_parts();
    broken.grain_scale = 0;
    assert_eq!(
        BrushRecipe::from_parts(&broken),
        Err(invalid("brush grain"))
    );
}

#[test]
fn stored_strokes_keep_their_recipes_and_share_equal_ones() {
    let point = |x| CanvasPoint::new(x, 4.0).unwrap();
    let sample = |x| StrokeSample::new(point(x), 0.5, 0.0, 0.0, 0).unwrap();
    let grainy = BrushRecipe::new(
        0.0,
        BrushRecipe::plain().size().clone(),
        BrushRecipe::plain().opacity().clone(),
        Grain::new(0.5, 2).unwrap(),
    )
    .unwrap();
    let mut artwork = Artwork::new(CanvasSize::new(32, 8).unwrap());
    for recipe in [BrushRecipe::plain(), grainy.clone(), BrushRecipe::plain()] {
        let style = StrokeStyle::new(Arc::new(recipe), 4.0, 1.0, Color::BLACK).unwrap();
        artwork = artwork
            .begin_stroke(style, sample(2.0))
            .unwrap()
            .finish()
            .artwork;
    }
    let stored = Artwork::from_storage(artwork.to_parts()).unwrap();
    assert_eq!(stored, artwork);
    let strokes: Vec<_> = stored.layers()[0].strokes().iter().cloned().collect();
    assert_eq!(**strokes[1].style().recipe(), grainy);
    let (first, third) = (strokes[0].style().recipe(), strokes[2].style().recipe());
    assert!(Arc::ptr_eq(first, third), "equal recipes are one");
    let mut parts = artwork.to_parts();
    parts.layers[0].strokes[1].recipe.softness = 2.0;
    assert_eq!(Artwork::from_storage(parts), Err(invalid("brush softness")));
}
