//! Accumulating strokes (D38): density builds up where a stroke crosses itself, capped by its
//! opacity; a covering stroke stays one even coverage; density is laid per distance travelled,
//! so more input events over the same path change nothing and joints leave no dots; the paper
//! takes more graphite under more pressure.

use std::sync::Arc;

use openpaint_core::{BrushRecipe, Color, StrokeSample, StrokeStyle};

use crate::fixture::{airbrush, blank, cold, core, graphite, pixel, sample, with_stroke};

/// Black at `size` and `opacity` with `recipe`.
fn style(recipe: Arc<BrushRecipe>, size: f32, opacity: f32) -> StrokeStyle {
    StrokeStyle::new(recipe, size, opacity, Color::BLACK).expect("style")
}

/// Samples every `step` px along straight lines through `corners`, at `pressure`.
fn path(corners: &[[f32; 2]], step: f32, pressure: f32) -> Vec<StrokeSample> {
    let mut points = Vec::new();
    for pair in corners.windows(2) {
        let ([ax, ay], [bx, by]) = (pair[0], pair[1]);
        let count = ((bx - ax).hypot(by - ay) / step).ceil().max(1.0) as u32;
        points.extend((0..count).map(|k| {
            let t = k as f32 / count as f32;
            [ax + (bx - ax) * t, ay + (by - ay) * t]
        }));
    }
    points.extend(corners.last());
    (0u32..)
        .zip(points)
        .map(|(i, [x, y])| sample(x, y, pressure, i * 2))
        .collect()
}

/// Down to the right, up, down to the left: the stroke crosses itself at (160, 100).
const CROSSING: [[f32; 2]; 4] = [[40.0, 60.0], [280.0, 140.0], [280.0, 60.0], [40.0, 140.0]];

#[test]
fn an_accumulating_stroke_darkens_where_it_crosses_itself_up_to_its_opacity() {
    let core = core();
    let stroke = (style(airbrush(), 30.0, 0.5), path(&CROSSING, 2.0, 0.6));
    let image = cold(&core, &with_stroke(&blank(), stroke));
    let [once, crossed] = [[80, 73], [160, 100]].map(|at| pixel(&image, at)[0]);
    assert!(
        crossed + 20 < once,
        "crossing {crossed} against once {once}"
    );
    // Never more than the stroke's opacity: half way to black at most.
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let darkest = pixels.iter().map(|p| p[0]).min().expect("pixels");
    assert!(darkest >= 127, "{darkest}");
}

#[test]
fn a_covering_stroke_crossing_itself_stays_one_even_coverage() {
    let core = core();
    let plain = Arc::new(BrushRecipe::plain());
    let stroke = (style(plain, 30.0, 0.5), path(&CROSSING, 2.0, 0.6));
    let image = cold(&core, &with_stroke(&blank(), stroke));
    assert_eq!(pixel(&image, [160, 100]), pixel(&image, [80, 73]));
}

#[test]
fn more_samples_over_the_same_path_lay_the_same_density_without_dots() {
    let core = core();
    let line = [[40.0, 100.0], [280.0, 100.0]];
    for recipe in [airbrush(), graphite()] {
        let draw = |step| {
            let stroke = (
                style(Arc::clone(&recipe), 20.0, 1.0),
                path(&line, step, 0.6),
            );
            cold(&core, &with_stroke(&blank(), stroke))
        };
        let (sparse, dense) = (draw(60.0), draw(1.0));
        for x in (70..250).step_by(7) {
            for y in [96, 100, 103] {
                let (a, b) = (pixel(&sparse, [x, y]), pixel(&dense, [x, y]));
                let differs = a.iter().zip(&b).any(|(a, b)| a.abs_diff(*b) > 1);
                assert!(!differs, "at ({x}, {y}): {a:?} against {b:?}");
            }
        }
    }
    // Along an airbrush's body every pixel of a row is one value: no dots at the joints.
    let stroke = (style(airbrush(), 20.0, 1.0), path(&line, 3.0, 0.6));
    let image = cold(&core, &with_stroke(&blank(), stroke));
    let row: Vec<u8> = (70..250).map(|x| pixel(&image, [x, 100])[0]).collect();
    let (low, high) = (row.iter().min(), row.iter().max());
    assert!(high.zip(low).is_some_and(|(h, l)| h - l <= 1), "{row:?}");
}

#[test]
fn graphite_takes_more_of_the_paper_under_more_pressure() {
    let core = core();
    let line = [[40.0, 100.0], [280.0, 100.0]];
    let mean = |pressure| {
        let stroke = (style(graphite(), 16.0, 1.0), path(&line, 2.0, pressure));
        let image = cold(&core, &with_stroke(&blank(), stroke));
        let values: Vec<f32> = (60..260)
            .flat_map(|x| (98..103).map(move |y| [x, y]))
            .map(|at| f32::from(pixel(&image, at)[0]))
            .collect();
        let average = values.iter().sum::<f32>() / values.len() as f32;
        let spread = values
            .iter()
            .fold(0.0f32, |most, v| most.max((v - average).abs()));
        (average, spread)
    };
    let (light, light_spread) = mean(0.2);
    let (heavy, _) = mean(0.9);
    assert!(heavy + 40.0 < light, "heavy {heavy} against light {light}");
    assert!(
        light_spread > 10.0,
        "the paper's grain shows: {light_spread}"
    );
}
