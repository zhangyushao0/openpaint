//! Taper and tilt (S44): the taper factor along a stroke's path rises from the tip's share to
//! exactly 1 and falls back symmetrically at the end, never above 1 and continuous on a stroke
//! shorter than both lengths; tilt curves are over degrees, may widen the size but never raise
//! the opacity, and multiply pressure and speed; both round-trip through parts.

use openpaint_core::{
    BrushRecipe, CoreError, Curve, Dynamics, Grain, MAX_GAIN, MAX_TAPER, MAX_TILT, Taper,
};

fn invalid(what: &'static str) -> CoreError {
    CoreError::InvalidValue { what }
}

fn curve(points: &[[f32; 2]]) -> Curve {
    Curve::new(points).unwrap()
}

#[test]
fn the_taper_rises_from_the_tip_to_exactly_one_and_falls_back_at_the_end() {
    // Two diameters at the start, three at the end, a fifth of the radius at the tips; a
    // stroke of size 10, so 20 and 30 pixels.
    let taper = Taper::new(2.0, 3.0, 0.2).unwrap();
    let size = 10.0;
    assert!(
        (taper.factor(0.0, None, size) - 0.2).abs() < 1e-6,
        "the tip"
    );
    assert!(
        (taper.factor(10.0, None, size) - 0.6).abs() < 1e-6,
        "halfway: smoothstep 1/2"
    );
    for along in [20.0, 20.5, 400.0] {
        assert_eq!(
            taper.factor(along, None, size),
            1.0,
            "past the start: exactly 1"
        );
    }
    // Drawn: no end taper; ended: the end mirrors the start over its own length.
    assert_eq!(taper.factor(399.0, None, size), 1.0);
    assert!((taper.factor(400.0, Some(0.0), size) - 0.2).abs() < 1e-6);
    assert!((taper.factor(385.0, Some(15.0), size) - 0.6).abs() < 1e-6);
    assert_eq!(
        taper.factor(200.0, Some(200.0), size),
        1.0,
        "the middle untouched"
    );
    let symmetric = Taper::new(2.0, 2.0, 0.2).unwrap();
    for step in 0..=40u8 {
        let d = f32::from(step);
        let start = symmetric.factor(d, Some(400.0 - d), size);
        let end = symmetric.factor(400.0 - d, Some(d), size);
        assert_eq!(start.to_bits(), end.to_bits(), "{d} px from either end");
    }
    // Rising, smooth: the steps along the start never jump.
    let mut before = taper.factor(0.0, None, size);
    for tenth in 1..=250u16 {
        let now = taper.factor(f32::from(tenth) / 10.0, None, size);
        assert!(now >= before && now - before < 0.01, "at {tenth}");
        before = now;
    }
    assert_eq!(Taper::NONE.factor(0.0, Some(0.0), size), 1.0, "no taper");
    assert!(Taper::NONE.is_none() && !taper.is_none());
}

#[test]
fn a_stroke_shorter_than_both_lengths_never_exceeds_one_and_stays_continuous() {
    let taper = Taper::new(3.0, 3.0, 0.0).unwrap();
    // 30 + 30 pixels of taper on a stroke 24 pixels long.
    let length = 24.0;
    let at = |along: f32| taper.factor(along, Some(length - along), 10.0);
    let mut before = at(0.0);
    assert_eq!(before, 0.0, "a point");
    let mut highest: f32 = 0.0;
    for hundredth in 1..=2400u16 {
        let along = f32::from(hundredth) / 100.0;
        let now = at(along);
        assert!(now <= 1.0);
        assert!((now - before).abs() < 0.01, "continuous at {along}");
        highest = highest.max(now);
        before = now;
    }
    // Smoothstep of 12/30 at the middle.
    assert!(
        (highest - 0.352).abs() < 1e-3,
        "peaks in the middle: {highest}"
    );
    assert!((at(12.0) - highest).abs() < 1e-6);
}

#[test]
fn a_taper_has_ranges() {
    assert!(Taper::new(MAX_TAPER, 0.0, 1.0).is_ok());
    for (start, end, tip) in [
        (-1.0, 0.0, 0.0),
        (0.0, MAX_TAPER + 1.0, 0.0),
        (1.0, 1.0, 1.5),
        (f32::NAN, 1.0, 0.0),
    ] {
        assert_eq!(Taper::new(start, end, tip), Err(invalid("brush taper")));
    }
}

#[test]
fn tilt_curves_are_over_degrees_and_multiply_pressure_and_speed() {
    let speed = curve(&[[0.0, 1.0], [4.0, 0.5]]);
    let wider = Curve::gain(&[[0.0, 1.0], [40.0, 1.0], [80.0, 3.0]]).unwrap();
    let untilted = Dynamics::new(curve(&[[0.0, 0.2], [1.0, 1.0]]), speed).unwrap();
    let size = untilted.clone().with_tilt(wider).unwrap();
    let degrees = |d: f32| d.to_radians();
    // Upright and up to 40 degrees: pressure times speed alone.
    let alone = untilted.at(0.5, 2.0, degrees(75.0));
    assert!((alone - 0.6 * 0.75).abs() < 1e-6);
    assert_eq!(size.at(0.5, 2.0, 0.0), alone);
    assert!((size.at(0.5, 2.0, degrees(30.0)) - alone).abs() < 1e-6);
    // 60 degrees: halfway to 3.
    assert!((size.at(0.5, 2.0, degrees(60.0)) - alone * 2.0).abs() < 1e-5);
    assert!((size.at(0.5, 2.0, std::f32::consts::FRAC_PI_2) - alone * 3.0).abs() < 1e-5);
    // Flat: bit for bit the product of pressure and speed, as every recipe before S44.
    let plain = BrushRecipe::plain();
    for step in 0..=100u8 {
        let p = f32::from(step) / 100.0;
        let tilted = plain.size().at(p, 0.0, degrees(70.0));
        assert_eq!(tilted.to_bits(), p.to_bits());
    }
}

#[test]
fn tilt_curves_have_ranges() {
    let flat = || curve(&[[0.0, 1.0]]);
    let dynamics = || Dynamics::new(flat(), flat()).unwrap();
    let far = Curve::gain(&[[0.0, 1.0], [MAX_TILT + 1.0, 2.0]]).unwrap();
    assert_eq!(dynamics().with_tilt(far), Err(invalid("brush dynamics")));
    let top = Curve::gain(&[[0.0, 1.0], [MAX_TILT, MAX_GAIN]]);
    assert!(dynamics().with_tilt(top.unwrap()).is_ok());
    let beyond = Curve::gain(&[[0.0, MAX_GAIN + 0.5]]);
    assert_eq!(beyond, Err(invalid("brush curve")));
    assert_eq!(
        Curve::new(&[[0.0, 1.5]]),
        Err(invalid("brush curve")),
        "not a gain"
    );
    // The size may grow with tilt; the opacity never rises above the stroke's.
    let raised = dynamics()
        .with_tilt(Curve::gain(&[[0.0, 1.5]]).unwrap())
        .unwrap();
    assert!(BrushRecipe::new(0.0, raised.clone(), dynamics(), Grain::NONE).is_ok());
    let refused = BrushRecipe::new(0.0, dynamics(), raised, Grain::NONE);
    assert_eq!(refused, Err(invalid("brush dynamics")));
}

#[test]
fn taper_and_tilt_round_trip_through_parts() {
    let wider = Curve::gain(&[[0.0, 1.0], [45.0, 1.0], [80.0, 2.5]]).unwrap();
    let lighter = curve(&[[0.0, 1.0], [45.0, 1.0], [80.0, 0.4]]);
    let plain = BrushRecipe::plain();
    let recipe = BrushRecipe::new(
        0.0,
        plain.size().clone().with_tilt(wider).unwrap(),
        plain.opacity().clone().with_tilt(lighter).unwrap(),
        Grain::NONE,
    )
    .unwrap()
    .with_taper(Taper::new(1.5, 4.0, 0.1).unwrap());
    let parts = recipe.to_parts();
    assert_eq!(parts.taper, [1.5, 4.0, 0.1]);
    assert_eq!(parts.size_tilt, [[0.0, 1.0], [45.0, 1.0], [80.0, 2.5]]);
    assert_eq!(BrushRecipe::from_parts(&parts), Ok(recipe));
    let plain_parts = plain.to_parts();
    assert_eq!(
        (
            plain_parts.size_tilt,
            plain_parts.opacity_tilt,
            plain_parts.taper
        ),
        (vec![[0.0, 1.0]], vec![[0.0, 1.0]], [0.0; 3]),
        "no tilt, no taper"
    );
    let mut broken = parts.clone();
    broken.taper[2] = 2.0;
    assert_eq!(
        BrushRecipe::from_parts(&broken),
        Err(invalid("brush taper"))
    );
    let mut raised = parts;
    raised.opacity_tilt = vec![[0.0, 1.0], [80.0, 1.5]];
    assert_eq!(
        BrushRecipe::from_parts(&raised),
        Err(invalid("brush dynamics"))
    );
}
