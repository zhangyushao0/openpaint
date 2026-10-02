//! `render-stroke`: evaluate a hard-coded stroke through the region evaluator, write the PNG
//! and print its hash.

use std::f32::consts::{PI, TAU};

use openpaint_core::{
    Artwork, CanvasPoint, CanvasSize, Color, CoreError, StrokeSample, StrokeStyle,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator};

use crate::error::HarnessError;
use crate::output::write_png;

/// The wave of `crates/render/tests/stroke/fixture.rs`, so the printed hash is the golden value
/// of `crates/render/tests/stroke/golden.rs`.
const SIZE: [u32; 2] = [320, 200];

pub fn run() -> Result<(), HarnessError> {
    let artwork = wave()?;
    let core = Core::headless()?;
    let mut target = CanvasTarget::new(&core, SIZE)?;
    let request = FrameRequest {
        artwork: &artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(&core, &mut target).evaluate(&request, &[PixelRect::full(SIZE)])?;
    let image = target.read(&core)?;
    let path = write_png("stroke.png", &image)?;
    println!("adapter {}", core.adapter());
    println!("wrote {}", path.display());
    println!("hash {:016x}", image.content_hash());
    Ok(())
}

/// An S-curve whose pressure rises from 0.1 to 1 and falls back, 80 % opaque.
fn wave() -> Result<Artwork, CoreError> {
    let style = StrokeStyle::new(40.0, 0.8, Color::rgb(0.12, 0.25, 0.65)?)?;
    let samples = (0..48u16)
        .map(|i| {
            let t = f32::from(i) / 47.0;
            let position = CanvasPoint::new(30.0 + 260.0 * t, 100.0 + 55.0 * (TAU * t).sin())?;
            let pressure = 0.1 + 0.9 * (PI * t).sin();
            StrokeSample::new(position, pressure, 0.0, 0.0, u32::from(i) * 8)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let blank = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1])?);
    let mut session = blank.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}
