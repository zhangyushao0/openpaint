//! Strokes made as a pen makes them, for the benchmarks and the harness's test artworks: a path
//! committed as one stroke (`drawn`), the pile's strokes crossing one place at every angle
//! (`pile_path`, `test-art pile`'s) and a built-in brush's style (`style_of`).

use std::f32::consts::PI;
use std::sync::Arc;

use openpaint_app::{BrushKey, BrushLibrary};
use openpaint_core::{Artwork, CanvasPoint, Color, StrokeSample, StrokeStyle};
use openpaint_render::RenderError;

use crate::error::RuntimeError;

fn sample(point: [f32; 2], pressure: f32, i: u32) -> Result<StrokeSample, RuntimeError> {
    let position = CanvasPoint::new(point[0], point[1])?;
    Ok(StrokeSample::new(position, pressure, 0.0, 0.0, 8 * i)?)
}

/// `artwork` with a stroke of `style` along `points`, pressed `pressure(i)`, samples 8 ms apart,
/// committed.
///
/// # Errors
/// As the artwork refuses the stroke.
pub fn drawn(
    artwork: &Artwork,
    style: StrokeStyle,
    points: &[[f32; 2]],
    pressure: impl Fn(u32) -> f32,
) -> Result<Artwork, RuntimeError> {
    let samples = (0..)
        .zip(points)
        .map(|(i, point)| sample(*point, pressure(i), i))
        .collect::<Result<Vec<_>, _>>()?;
    let mut session = artwork.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}

/// The pile's stroke `k` about `middle`: a 500 px path gently
/// bent through `middle`, turned about its box's middle by `k` golden angles and moved by
/// `(37 k mod 281 - 140, 59 k mod 281 - 140)` screen pixels at fit (0.92 screen pixels a canvas
/// pixel).
#[must_use]
pub fn pile_path(k: u32, middle: [f32; 2]) -> Vec<[f32; 2]> {
    let path: Vec<[f32; 2]> = (0..52u16)
        .map(|i| {
            let t = f32::from(i) / 51.0;
            let x = 500.0f32.mul_add(t, middle[0] - 250.0);
            [x, 43.0f32.mul_add((PI * t).sin(), middle[1])]
        })
        .collect();
    if k == 0 {
        return path;
    }
    let centre = [0, 1].map(|axis| {
        let values = path.iter().map(|p| p[axis]);
        let (low, high) = values.fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(v), h.max(v)));
        (low + high) / 2.0
    });
    let angle = f64::from(k) * 2.399_963_2;
    let (cos, sin) = (angle.cos() as f32, angle.sin() as f32);
    let shift = [37, 59].map(|step| ((k * step) % 281) as f32 - 140.0);
    path.iter()
        .map(|point| {
            let [x, y] = [point[0] - centre[0], point[1] - centre[1]];
            [
                centre[0] + shift[0] / 0.92 + x * cos - y * sin,
                centre[1] + shift[1] / 0.92 + x * sin + y * cos,
            ]
        })
        .collect()
}

/// A stroke of built-in brush `key` at `size` (its first size without one), seed 0, painting.
///
/// # Errors
/// `Invalid` for a brush not built in.
pub fn style_of(
    library: &BrushLibrary,
    key: &str,
    size: Option<f32>,
) -> Result<StrokeStyle, RuntimeError> {
    let brush = library
        .brush(&BrushKey::new(key))
        .ok_or(RuntimeError::Render(RenderError::Invalid(
            "the brush is not built in",
        )))?;
    let size = size.unwrap_or(brush.size().default);
    let color = Color::rgb(0.2, 0.3, 0.6)?;
    Ok(StrokeStyle::new(
        Arc::clone(brush.recipe()),
        size,
        brush.opacity(),
        color,
    )?)
}
