//! Shapes and scatter (S45): a stamped stroke of discs equals a CPU reference stamping
//! antialiased discs at the same places with the same hashed jitter; a straight stroke cut into
//! many spans lays bit for bit what it lays as one; one seed evaluates bit for bit twice and
//! another differently; the Flat brush's square turns with the stroke.

use std::sync::Arc;

use openpaint_core::{
    Accumulation, BrushRecipe, Color, Curve, Deposit, Dynamics, Grain, Rotation, Shape,
    ShapeTexture, Stamp, StrokeSample, StrokeStyle, Tip, jitter,
};
use openpaint_render::RgbaImage;

use crate::fixture::{SIZE, blank, cold, core, pixel, sample, with_stroke};

fn curve(points: &[[f32; 2]]) -> Curve {
    Curve::new(points).expect("curve")
}

fn dynamics(pressure: &[[f32; 2]]) -> Dynamics {
    Dynamics::new(curve(pressure), curve(&[[0.0, 1.0]])).expect("dynamics")
}

/// A recipe of `shape`: size and opacity over pressure as given, covering unless `flow`.
pub fn stamped(shape: Shape, size: &[[f32; 2]], opacity: &[[f32; 2]]) -> BrushRecipe {
    BrushRecipe::new(0.0, dynamics(size), dynamics(opacity), Grain::NONE)
        .and_then(|recipe| recipe.with_shape(shape))
        .expect("recipe")
}

fn shape(
    stamp: (Stamp, f32),
    spacing: f32,
    scatter: f32,
    turn: (Rotation, f32),
    size: f32,
) -> Shape {
    let (rotation, jitter) = turn;
    Shape::stamped(stamp, spacing, scatter, (rotation, 0.0, jitter), size).expect("shape")
}

/// The four stamped built-ins as `crates/app/brushes/*.toml` hold them, with their default
/// sizes.
pub fn brushes() -> Vec<(&'static str, BrushRecipe, f32)> {
    let flat = [[0.0, 1.0]];
    let chalk = shape(
        (Stamp::Texture(ShapeTexture::Chalk), 1.0),
        0.1,
        0.0,
        (Rotation::Fixed, 15.0),
        0.0,
    );
    let textured = stamped(
        chalk,
        &[[0.0, 0.35], [1.0, 1.0]],
        &[[0.0, 0.25], [1.0, 1.0]],
    );
    let grain = Grain::new(0.3, 5).expect("grain");
    let textured = BrushRecipe::new(
        0.0,
        textured.size().clone(),
        textured.opacity().clone(),
        grain,
    )
    .and_then(|recipe| recipe.with_shape(chalk))
    .expect("textured round");
    let square = shape(
        (Stamp::Square, 0.25),
        0.05,
        0.0,
        (Rotation::Direction, 0.0),
        0.0,
    );
    let rough = (Stamp::Texture(ShapeTexture::RoughSquare), 0.5);
    let opaque = shape(rough, 0.08, 0.0, (Rotation::Direction, 4.0), 0.1);
    let dots = shape((Stamp::Disc, 1.0), 0.5, 3.0, (Rotation::Fixed, 0.0), 0.5);
    let speckle = stamped(dots, &flat, &[[0.0, 0.3], [1.0, 1.0]]);
    let deposit = Accumulation::new(Tip::Flat, 2.0, 1.0, None).expect("deposit");
    let speckle = speckle
        .with_deposit(Deposit::Accumulate(deposit))
        .expect("speckle");
    vec![
        ("textured round", textured, 40.0),
        (
            "flat",
            stamped(square, &flat, &[[0.0, 0.2], [1.0, 1.0]]),
            30.0,
        ),
        (
            "opaque paint",
            stamped(opaque, &flat, &[[0.0, 0.3], [1.0, 1.0]]),
            40.0,
        ),
        ("speckle", speckle, 12.0),
    ]
}

/// A level line along y = 100.25 from x = 40 to 296, a sample every 8 px (each a knot: the
/// pressure falls a ten-thousandth per sample), or its two ends alone.
fn level(every: bool) -> Vec<StrokeSample> {
    let at = |i: u16| {
        sample(
            40.0 + 8.0 * f32::from(i),
            100.25,
            0.9 - 0.0001 * f32::from(i),
            4 * u32::from(i),
        )
    };
    if every {
        (0..=32).map(at).collect()
    } else {
        vec![at(0), at(32)]
    }
}

fn style(recipe: BrushRecipe, size: f32, seed: u32) -> StrokeStyle {
    StrokeStyle::new(Arc::new(recipe), size, 1.0, Color::BLACK)
        .expect("style")
        .with_seed(seed)
}

/// Discs at spacing 0.3 of a 20 px stroke (every 6 px), scattered a radius, sized 1 +- 0.4.
fn discs() -> BrushRecipe {
    let flat = [[0.0, 1.0]];
    let discs = shape((Stamp::Disc, 1.0), 0.3, 1.0, (Rotation::Fixed, 0.0), 0.4);
    stamped(discs, &flat, &flat)
}

/// What the CPU lays for `discs` along the level line of `seed`: black on white, each pixel
/// the largest coverage of a disc antialiased over one pixel.
fn reference(seed: u32) -> Vec<u8> {
    let (radius, step) = (10.0f32, 6.0f32);
    let stamps: Vec<[f32; 3]> = (0..=(256.0 / step) as u32)
        .map(|n| {
            let [along, across, _, size] = jitter(seed, n).map(|u| 2.0 * u - 1.0);
            let x = 40.0 + n as f32 * step + radius * along;
            [x, 100.25 + radius * across, radius * (1.0 + 0.4 * size)]
        })
        .collect();
    let mut pixels = Vec::with_capacity((SIZE[0] * SIZE[1]) as usize);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let coverage = stamps
                .iter()
                .map(|[cx, cy, r]| (0.5 - ((px - cx).hypot(py - cy) - r)).clamp(0.0, 1.0))
                .fold(0.0, f32::max);
            pixels.push((255.0 * (1.0 - coverage)).round() as u8);
        }
    }
    pixels
}

fn reds(image: &RgbaImage) -> Vec<u8> {
    image.pixels().chunks(4).map(|rgba| rgba[0]).collect()
}

#[test]
fn stamped_discs_are_within_a_level_of_the_cpus_discs_at_the_same_places() {
    let core = core();
    for seed in [1, 0xC0FF_EE00] {
        let artwork = with_stroke(&blank(), (style(discs(), 20.0, seed), level(false)));
        let image = reds(&cold(&core, &artwork));
        let expected = reference(seed);
        let largest = image
            .iter()
            .zip(&expected)
            .map(|(a, b)| a.abs_diff(*b))
            .max();
        let painted = expected.iter().filter(|value| **value < 255).count();
        eprintln!("seed {seed:x}: largest difference {largest:?} over {painted} painted pixels");
        assert!(largest <= Some(1), "seed {seed:x}: {largest:?}");
        assert!(painted > 5000, "the discs paint");
    }
}

#[test]
fn a_straight_stroke_in_many_spans_lays_what_it_lays_in_one() {
    let core = core();
    let square = shape(
        (Stamp::Square, 0.25),
        0.1,
        0.0,
        (Rotation::Direction, 0.0),
        0.0,
    );
    let flat = [[0.0, 1.0]];
    for recipe in [discs(), stamped(square, &flat, &flat)] {
        let one = cold(
            &core,
            &with_stroke(&blank(), (style(recipe.clone(), 20.0, 7), level(false))),
        );
        let many = cold(
            &core,
            &with_stroke(&blank(), (style(recipe, 20.0, 7), level(true))),
        );
        assert!(one == many, "33 knots lay bit for bit what 2 do");
    }
}

#[test]
fn one_seed_evaluates_bit_for_bit_twice_and_another_seed_differently() {
    let core = core();
    for (name, recipe, size) in brushes() {
        let drawn = |seed| {
            cold(
                &core,
                &with_stroke(&blank(), (style(recipe.clone(), size, seed), level(false))),
            )
        };
        let (first, again, other) = (drawn(11), drawn(11), drawn(12));
        assert!(first == again, "{name}: the same seed twice");
        if name != "flat" {
            assert!(first != other, "{name}: another seed");
        }
    }
}

#[test]
fn stamps_turned_by_the_pens_azimuth_are_stamps_turned_by_that_angle() {
    let core = core();
    let flat = [[0.0, 1.0]];
    let square = (Stamp::Square, 0.25);
    let angle = 50f32;
    let by = |rotation, degrees| {
        let shape = Shape::stamped(square, 0.1, 0.0, (rotation, degrees, 0.0), 0.0);
        stamped(shape.expect("shape"), &flat, &flat)
    };
    let pen = level(true)
        .into_iter()
        .map(|s| {
            let azimuth = angle.to_radians();
            StrokeSample::new(s.position(), s.pressure(), 0.0, azimuth, s.time_ms())
                .expect("sample")
        })
        .collect::<Vec<_>>();
    let azimuth = cold(
        &core,
        &with_stroke(&blank(), (style(by(Rotation::Azimuth, 0.0), 20.0, 5), pen)),
    );
    let fixed = cold(
        &core,
        &with_stroke(
            &blank(),
            (style(by(Rotation::Fixed, angle), 20.0, 5), level(true)),
        ),
    );
    let upright = cold(
        &core,
        &with_stroke(
            &blank(),
            (style(by(Rotation::Fixed, 0.0), 20.0, 5), level(true)),
        ),
    );
    assert!(
        azimuth == fixed,
        "the azimuth turns the stamps as the angle does"
    );
    assert!(azimuth != upright);
}

/// Down column `x`: where the paint's middle lies and how far it reaches (its coverage summed).
fn column(image: &RgbaImage, x: u32) -> (f32, f32) {
    let (mut sum, mut moment) = (0.0, 0.0);
    for y in 0..SIZE[1] {
        let coverage = f32::from(255 - pixel(image, [x, y])[0]) / 255.0;
        sum += coverage;
        moment += coverage * (y as f32 + 0.5);
    }
    (moment / sum, sum)
}

/// A band's direction between columns `x0` and `x1` (degrees, y down) and its width across
/// that direction (the paint down a column times the cosine of the direction).
fn band(image: &RgbaImage, [x0, x1]: [u32; 2]) -> (f32, f32) {
    let ((y0, down0), (y1, down1)) = (column(image, x0), column(image, x1));
    let angle = (y1 - y0).atan2(x1 as f32 - x0 as f32);
    (angle.to_degrees(), 0.5 * (down0 + down1) * angle.cos())
}

#[test]
fn the_flats_square_turns_with_the_stroke() {
    let core = core();
    let (_, flat, _) = brushes().swap_remove(1);
    // Level, then down and right at 60 degrees, the curve rounding the corner.
    let turn = 60f32.to_radians();
    let mut samples = Vec::new();
    for i in 0..=75u16 {
        let s = f32::from(i);
        let (x, y) = if s <= 30.0 {
            (20.0 + 4.0 * s, 40.0)
        } else {
            let t = 4.0 * (s - 30.0);
            (140.0 + t * turn.cos(), 40.0 + t * turn.sin())
        };
        // Full pressure (the Flat's opacity follows it), a knot at every sample.
        samples.push(sample(x, y, 1.0 - 0.0001 * s, 4 * u32::from(i)));
    }
    let image = cold(
        &core,
        &with_stroke(&blank(), (style(flat, 30.0, 3), samples)),
    );
    let (level_angle, level_width) = band(&image, [60, 100]);
    let (falling_angle, falling_width) = band(&image, [180, 200]);
    eprintln!(
        "level {level_angle:.2} deg, {level_width:.2} px; falling {falling_angle:.2} deg, \
         {falling_width:.2} px"
    );
    assert!(level_angle.abs() < 1.0, "{level_angle}");
    assert!(
        (falling_angle - 60.0).abs() < 1.0,
        "turned 60 degrees: {falling_angle}"
    );
    // Held across the stroke on both legs: the band is the square's width, 30 px.
    for width in [level_width, falling_width] {
        assert!((width - 30.0).abs() < 1.0, "{width}");
    }
}
