//! Shapes and scatter (S45): the shape stage's ranges, the stamps that may reach a pixel within
//! `MAX_STAMPS`, stamps refusing paper, the stroke's seed and the jitter it gives (the same
//! seed and index the same values, another seed others), and both round-tripping through parts.

use std::sync::Arc;

use openpaint_core::{
    Accumulation, Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError, Curve, Deposit,
    Dynamics, Grain, MAX_SCATTER, MAX_SPACING, MAX_STAMPS, MIN_ASPECT, MIN_SPACING, Paper,
    PaperTexture, Rotation, Shape, ShapeParts, ShapeTexture, Stamp, StrokeSample, StrokeStyle, Tip,
    jitter,
};

fn invalid() -> CoreError {
    CoreError::InvalidValue {
        what: "brush shape",
    }
}

fn shape(stamp: (Stamp, f32), spacing: f32, scatter: f32, jitters: [f32; 2]) -> Shape {
    let [rotation_jitter, size_jitter] = jitters;
    let rotation = (Rotation::Direction, 0.0, rotation_jitter);
    Shape::stamped(stamp, spacing, scatter, rotation, size_jitter).expect("shape")
}

#[test]
fn the_shape_stage_checks_every_range() {
    let make = |stamp, spacing, scatter, (rotation, angle, jitter), size| {
        Shape::stamped(stamp, spacing, scatter, (rotation, angle, jitter), size)
    };
    let turn = (Rotation::Fixed, 0.0, 0.0);
    assert!(make((Stamp::Disc, 1.0), MIN_SPACING, 0.0, turn, 0.0).is_ok());
    assert!(
        make(
            (Stamp::Square, MIN_ASPECT),
            MAX_SPACING,
            MAX_SCATTER,
            turn,
            1.0
        )
        .is_ok()
    );
    let texture = Stamp::Texture(ShapeTexture::RoughSquare);
    let turned = (Rotation::Azimuth, -180.0, 180.0);
    assert!(make((texture, 0.5), 0.08, 0.0, turned, 0.0).is_ok());
    let refused = [
        make((Stamp::Disc, 0.5), 0.1, 0.0, turn, 0.0),
        make((Stamp::Ellipse, 0.01), 0.1, 0.0, turn, 0.0),
        make((Stamp::Ellipse, 1.5), 0.1, 0.0, turn, 0.0),
        make((Stamp::Disc, 1.0), 0.0, 0.0, turn, 0.0),
        make((Stamp::Disc, 1.0), 0.04, 0.0, turn, 0.0),
        make((Stamp::Disc, 1.0), 4.5, 0.0, turn, 0.0),
        make((Stamp::Disc, 1.0), 0.1, -0.1, turn, 0.0),
        make((Stamp::Disc, 1.0), 0.1, 4.5, turn, 0.0),
        make(
            (Stamp::Disc, 1.0),
            0.1,
            0.0,
            (Rotation::Fixed, 181.0, 0.0),
            0.0,
        ),
        make(
            (Stamp::Disc, 1.0),
            0.1,
            0.0,
            (Rotation::Fixed, 0.0, -1.0),
            0.0,
        ),
        make(
            (Stamp::Disc, 1.0),
            0.1,
            0.0,
            (Rotation::Fixed, 0.0, 200.0),
            0.0,
        ),
        make((Stamp::Disc, 1.0), 0.1, 0.0, turn, 1.5),
        make((Stamp::Disc, 1.0), f32::NAN, 0.0, turn, 0.0),
    ];
    for (k, result) in refused.into_iter().enumerate() {
        assert_eq!(result, Err(invalid()), "case {k}");
    }
    assert!(!Shape::CONTINUOUS.is_stamped());
}

#[test]
fn a_stamps_reach_and_the_stamps_a_pixel_takes_fit_max_stamps() {
    let disc = shape((Stamp::Disc, 1.0), MIN_SPACING, 0.0, [0.0, 0.0]);
    assert_eq!(disc.reach(), 1.0);
    // At the spacing floor a disc's window holds 1 / 0.05 + 2 = 22 stamps.
    assert!(disc.fits(1.0) && disc.fits(2.3));
    assert!(!disc.fits(2.4), "a size gain of 2.4 at the floor overflows");
    let square = shape((Stamp::Square, 0.25), 0.1, 0.0, [0.0, 0.0]);
    assert!(
        (square.reach() - 0.25f32.hypot(1.0)).abs() < 1e-6,
        "its corner"
    );
    let speckle = shape((Stamp::Disc, 1.0), 0.5, 3.0, [0.0, 0.5]);
    let reach = 1.5 + 3.0 * std::f32::consts::SQRT_2;
    assert!(
        (speckle.reach() - reach).abs() < 1e-5,
        "{}",
        speckle.reach()
    );
    assert!(speckle.fits(1.0));
    let crowded = shape((Stamp::Disc, 1.0), MIN_SPACING, 3.0, [0.0, 0.0]);
    assert!(
        !crowded.fits(1.0),
        "{} stamps",
        crowded.reach() / MIN_SPACING
    );
    assert_eq!(MAX_STAMPS, 48);
}

fn flat() -> Curve {
    Curve::new(&[[0.0, 1.0]]).expect("flat")
}

fn dynamics(size: &[[f32; 2]]) -> Dynamics {
    Dynamics::new(Curve::new(size).expect("curve"), flat()).expect("dynamics")
}

#[test]
fn a_recipe_takes_a_shape_within_its_window_and_never_on_paper() {
    let plain = BrushRecipe::plain();
    assert_eq!(*plain.shape(), Shape::CONTINUOUS);
    let disc = shape((Stamp::Disc, 1.0), MIN_SPACING, 0.0, [0.0, 0.0]);
    assert!(plain.clone().with_shape(disc).is_ok());
    // Size over tilt widening up to 3 times: the window at the floor would overflow.
    let wide = dynamics(&[[0.0, 1.0]])
        .with_tilt(Curve::gain(&[[0.0, 1.0], [80.0, 3.0]]).expect("gain"))
        .expect("tilt");
    let widening = BrushRecipe::new(0.0, wide, dynamics(&[[0.0, 1.0]]), Grain::NONE);
    assert_eq!(widening.expect("recipe").with_shape(disc), Err(invalid()));
    let contact = Curve::new(&[[0.0, 0.5]]).expect("contact");
    let paper = Paper::new(PaperTexture::Sketch, contact, 4.5, 0.3, 0.3).expect("paper");
    let graphite = Accumulation::new(Tip::Flat, 3.0, 1.0, Some(paper)).expect("graphite");
    let papered = plain
        .clone()
        .with_deposit(Deposit::Accumulate(graphite.clone()))
        .expect("pencil");
    assert_eq!(papered.with_shape(disc), Err(invalid()));
    let stamped = plain.with_shape(disc).expect("stamped");
    assert!(stamped.with_deposit(Deposit::Accumulate(graphite)).is_err());
}

#[test]
fn the_same_seed_and_index_give_the_same_jitter_and_another_seed_other_jitter() {
    for index in [0, 1, 2, 77, 4096, u32::MAX] {
        let units = jitter(9, index);
        assert_eq!(units, jitter(9, index), "deterministic");
        assert!(units.iter().all(|u| (0.0..1.0).contains(u)), "{units:?}");
        assert_ne!(units, jitter(10, index), "another seed");
    }
    assert_ne!(jitter(9, 0), jitter(9, 1), "another stamp");
    // Spread evenly: the mean of many is about a half, none repeats.
    let many: Vec<[f32; 4]> = (0..4000).map(|index| jitter(1234, index)).collect();
    for k in 0..4 {
        let mean = many.iter().map(|units| units[k]).sum::<f32>() / 4000.0;
        assert!((mean - 0.5).abs() < 0.02, "component {k}: {mean}");
    }
    let mut firsts: Vec<u32> = many.iter().map(|units| units[0].to_bits()).collect();
    firsts.sort_unstable();
    firsts.dedup();
    assert!(firsts.len() > 3990, "{} distinct", firsts.len());
}

#[test]
fn the_shape_and_the_seed_round_trip_through_parts() {
    let texture = Stamp::Texture(ShapeTexture::Chalk);
    let rotation = (Rotation::Azimuth, -30.0, 15.0);
    let chalk = Shape::stamped((texture, 0.75), 0.1, 0.5, rotation, 0.25).expect("chalk");
    let recipe = BrushRecipe::new(
        0.0,
        dynamics(&[[0.0, 0.2], [1.0, 1.0]]),
        dynamics(&[[0.0, 1.0]]),
        Grain::NONE,
    )
    .and_then(|recipe| recipe.with_shape(chalk))
    .expect("recipe");
    let parts = recipe.to_parts();
    assert_eq!(parts.shape.stamp, texture);
    assert_eq!(BrushRecipe::from_parts(&parts), Ok(recipe.clone()));
    assert_eq!(
        BrushRecipe::plain().to_parts().shape,
        ShapeParts::CONTINUOUS
    );
    let mut odd = BrushRecipe::plain().to_parts();
    odd.shape.scatter = 1.0;
    assert_eq!(
        BrushRecipe::from_parts(&odd),
        Err(invalid()),
        "a continuous shape scatters nothing"
    );
    let style = StrokeStyle::new(Arc::new(recipe), 30.0, 1.0, Color::BLACK)
        .expect("style")
        .with_seed(0xDEAD_BEEF);
    let artwork = Artwork::new(CanvasSize::new(100, 100).expect("size"));
    let point = CanvasPoint::new(10.0, 10.0).expect("point");
    let first = StrokeSample::new(point, 0.5, 0.0, 0.0, 0).expect("sample");
    let drawn = artwork
        .begin_stroke(style, first)
        .expect("begin")
        .finish()
        .artwork;
    let parts = drawn.to_parts();
    assert_eq!(parts.layers[0].strokes[0].seed, 0xDEAD_BEEF);
    let reopened = Artwork::from_storage(parts.clone()).expect("reopened");
    assert_eq!(reopened.to_parts(), parts);
    let stroke = &reopened.layers()[0].strokes().iter().next().cloned();
    assert_eq!(stroke.as_ref().map(|s| s.style().seed()), Some(0xDEAD_BEEF));
}
