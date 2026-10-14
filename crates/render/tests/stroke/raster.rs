//! The raster core probe (S57): a small scene of every kind of stroke (a hard and a soft
//! covering capsule with grain, the pencil's graphite on paper, a bell airbrush, the four stamped
//! built-ins) drawn through the GPU's rasterizer equals the evaluator's image within one 8-bit
//! level away from antialiased edges, at 1x, 4x and 8x, and draws alike bit for bit every time.

use std::sync::Arc;

use openpaint_core::{Artwork, Color, StrokeSample, StrokeStyle};
use openpaint_render::{ImageDifference, image_difference};

use crate::fixture::{
    SIZE, airbrush, blank, core, diagonal_with, graphite, rich, sample, wave, wave_with,
    with_stroke,
};
use crate::stamps::brushes;

/// A loop of 40 samples about `centre`, pressed from 0.4 to 1.
fn ring(centre: [f32; 2], radius: f32) -> Vec<StrokeSample> {
    (0..40u16)
        .map(|i| {
            let t = f32::from(i) / 39.0;
            let angle = std::f32::consts::TAU * 0.9 * t;
            let [x, y] = [
                centre[0] + radius * angle.cos(),
                centre[1] + radius * angle.sin(),
            ];
            sample(x, y, 0.4 + 0.6 * t, u32::from(i) * 8)
        })
        .collect()
}

fn scene() -> Artwork {
    let mut artwork = with_stroke(&blank(), wave());
    artwork = with_stroke(&artwork, wave_with(rich()));
    artwork = with_stroke(&artwork, diagonal_with(graphite()));
    artwork = with_stroke(&artwork, diagonal_with(airbrush()));
    let color = Color::rgb(0.3, 0.6, 0.2).expect("colour");
    for (k, (_, recipe, size)) in brushes().into_iter().take(4).enumerate() {
        let style = StrokeStyle::new(Arc::new(recipe), size * 0.5, 0.9, color)
            .expect("style")
            .with_seed(k as u32 + 7);
        let centre = [70.0 + 60.0 * k as f32, 60.0 + 25.0 * (k % 2) as f32];
        artwork = with_stroke(&artwork, (style, ring(centre, 30.0)));
    }
    artwork
}

/// The probe's image against the evaluator's over the target of `SIZE` output pixels at
/// `scale`, the canvas's middle in its middle; and the share of the probe's pixels painted.
fn compared(artwork: &Artwork, scale: u32) -> (ImageDifference, f64) {
    let core = core();
    let at = SIZE.map(|side| side / 2 - side / (2 * scale));
    let (_, evaluated) = core
        .time_evaluation(artwork, (scale, at), SIZE, 1)
        .expect("evaluate");
    let raster = core
        .time_raster(artwork, (scale, at), SIZE, (1, 1024))
        .expect("raster");
    assert!(
        raster.repeatable,
        "{scale}x: the probe draws alike every time"
    );
    let pixels = raster.image.pixels().as_chunks::<4>().0;
    let painted = pixels.iter().filter(|p| **p != [255; 4]).count();
    let share = painted as f64 / pixels.len() as f64;
    let found = image_difference(&evaluated, &raster.image).expect("same size");
    (found, share)
}

#[test]
fn every_stroke_kind_rasters_within_a_level_of_the_evaluator_away_from_edges() {
    let artwork = scene();
    for scale in [1, 4, 8] {
        let (found, painted) = compared(&artwork, scale);
        println!("{scale}x: {painted:.3} painted, {found:?}");
        assert!(painted > 0.2, "{scale}x: the probe paints {painted}");
        assert_eq!(found.beyond_one_inside, 0, "{scale}x: {found:?}");
        assert!(
            found.largest.iter().all(|off| *off <= 2),
            "{scale}x: {found:?}"
        );
        assert!(
            found.mean.iter().all(|mean| *mean < 0.01),
            "{scale}x: {found:?}"
        );
    }
}
