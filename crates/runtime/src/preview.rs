//! The brush picker's previews: each brush draws one S-shaped stroke through the product
//! evaluator, so a preview shows exactly what the brush paints: black on white, which the panel
//! turns into its ink and alpha.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use openpaint_app::{Brush, BrushKey, BrushLibrary};
use openpaint_core::{
    Artwork, CanvasPoint, CanvasSize, Color, CoreError, StrokeSample, StrokeStyle,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};
use openpaint_ui::design;

use crate::error::RuntimeError;

/// Samples along the preview stroke.
const SAMPLES: u16 = 64;
/// Milliseconds between samples: an unhurried stroke.
const SAMPLE_MS: u32 = 6;
/// The largest diameter as a share of the preview's height, so wide brushes fit.
const HEIGHT_SHARE: f32 = 0.55;
/// The smallest diameter as a share of the preview's height, so thin brushes show their grain.
const THINNEST_SHARE: f32 = 0.15;

/// One preview per brush, in library order, `design::BRUSH_PREVIEW` points large at
/// `pixels_per_point`.
pub(crate) fn brush_previews(
    core: &Core,
    library: &BrushLibrary,
    pixels_per_point: f32,
) -> Result<Vec<(BrushKey, RgbaImage)>, RuntimeError> {
    let size = preview_size(pixels_per_point);
    library
        .brushes()
        .iter()
        .map(|brush| Ok((brush.key().clone(), brush_picture(core, brush, size)?)))
        .collect()
}

/// A preview's size in pixels at `pixels_per_point`.
pub(crate) fn preview_size(pixels_per_point: f32) -> [u32; 2] {
    let points = design::BRUSH_PREVIEW;
    [points.x, points.y].map(|side| (side * pixels_per_point).round().max(8.0) as u32)
}

/// `brush`'s preview of `size` pixels.
pub(crate) fn brush_picture(
    core: &Core,
    brush: &Brush,
    size: [u32; 2],
) -> Result<RgbaImage, RuntimeError> {
    let artwork = stroke(brush, size)?;
    let mut target = CanvasTarget::new(core, size)?;
    let request = FrameRequest {
        artwork: &artwork,
        active: None,
        background: gray(255)?,
    };
    RegionEvaluator::new(core, &mut target).evaluate(&request, &[PixelRect::full(size)])?;
    Ok(target.read(core)?)
}

/// A blank artwork of `size` with one S-shaped stroke across it, black on white, its pressure
/// rising and falling again.
fn stroke(brush: &Brush, [width, height]: [u32; 2]) -> Result<Artwork, CoreError> {
    let (width, height) = (width as f32, height as f32);
    let diameter = brush
        .size()
        .default
        .clamp(THINNEST_SHARE * height, HEIGHT_SHARE * height);
    let ink = gray(0)?;
    let style = StrokeStyle::new(Arc::clone(brush.recipe()), diameter, brush.opacity(), ink)?;
    let margin = 0.5 * height;
    let samples = (0..SAMPLES)
        .map(|i| {
            let t = f32::from(i) / f32::from(SAMPLES - 1);
            let x = margin + (width - 2.0 * margin) * t;
            let y = 0.5 * height - 0.22 * height * (TAU * t).sin();
            let pressure = 0.2 + 0.8 * (PI * t).sin();
            StrokeSample::new(
                CanvasPoint::new(x, y)?,
                pressure,
                0.0,
                0.0,
                u32::from(i) * SAMPLE_MS,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let blank = Artwork::new(CanvasSize::new(width as u32, height as u32)?);
    let mut session = blank.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}

/// An sRGB grey from an 8-bit value; blending happens on encoded values, so the evaluator gives
/// back exactly that byte.
fn gray(value: u8) -> Result<Color, CoreError> {
    let level = f32::from(value) / 255.0;
    Color::rgb(level, level, level)
}
