//! `brush-sheet`: every built-in brush paints one stroke at its default size (pressure rising,
//! then easing, along a gentle S with one sharp turn), written at 1x and, cropped to its turn,
//! at 4x: `artifacts/harness/brush-<key>.png` and `brush-<key>-4x.png`, for judging how a brush
//! looks while tuning its file (S45). A tap at full pressure comes first: one stamp.

use std::f32::consts::PI;
use std::sync::Arc;

use openpaint_app::BrushLibrary;
use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, Rotation, Shape, ShapeTexture, Stamp,
    StrokeSample, StrokeStyle,
};
use openpaint_render::{Banding, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator};

use crate::error::HarnessError;
use crate::output::{write_png, write_rgba};

const SIZE: [u32; 2] = [720, 200];
/// The 4x crop: its top-left canvas pixel and its size in canvas pixels.
const CROP: ([u32; 2], [u32; 2]) = ([20, 0], [200, 140]);

pub fn run() -> Result<(), HarnessError> {
    let core = Core::headless()?;
    let library = BrushLibrary::built_in().map_err(openpaint_runtime::RuntimeError::from)?;
    for (index, brush) in library.brushes().iter().enumerate() {
        let style = StrokeStyle::new(
            Arc::clone(brush.recipe()),
            brush.size().default,
            brush.opacity(),
            Color::BLACK,
        )?
        .with_seed(0x5EED_0000 + index as u32);
        let artwork = stroke(style)?;
        let mut target = CanvasTarget::new(&core, SIZE)?;
        let request = FrameRequest {
            artwork: &artwork,
            active: None,
            background: Color::WHITE,
        };
        RegionEvaluator::new(&core, &mut target).evaluate(&request, &[PixelRect::full(SIZE)])?;
        let key = brush.key().as_str();
        let path = write_png(&format!("brush-{key}.png"), &target.read(&core)?)?;
        let fine = core
            .export_bands(&artwork, Color::WHITE, 4, Banding::DEFAULT)?
            .image()?;
        let ([x, y], [width, height]) = CROP;
        let stride = (SIZE[0] * 4 * 4) as usize;
        let rows = (4 * y..4 * (y + height)).flat_map(|row| {
            let at = row as usize * stride + (4 * x * 4) as usize;
            fine.pixels()[at..at + (4 * width * 4) as usize].to_vec()
        });
        let crop: Vec<u8> = rows.collect();
        let dir = path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        write_rgba(
            &dir.join(format!("brush-{key}-4x.png")),
            [4 * width, 4 * height],
            &crop,
        )?;
        println!("{}: wrote {}", brush.name(), path.display());
    }
    shapes(&core)
}

/// Every shape texture as one large stamp, side by side: `artifacts/harness/shapes.png`.
fn shapes(core: &Core) -> Result<(), HarnessError> {
    let size = [4 * 200, 200];
    let mut artwork = Artwork::new(CanvasSize::new(size[0], size[1])?);
    for (k, texture) in ShapeTexture::ALL.into_iter().enumerate() {
        let shape = Shape::stamped(
            (Stamp::Texture(texture), 1.0),
            1.0,
            0.0,
            (Rotation::Fixed, 0.0, 0.0),
            0.0,
        )?;
        let recipe = BrushRecipe::plain().with_shape(shape)?;
        let style = StrokeStyle::new(Arc::new(recipe), 190.0, 1.0, Color::BLACK)?;
        let at = CanvasPoint::new(100.0 + 200.0 * k as f32, 100.0)?;
        let tap = StrokeSample::new(at, 1.0, 0.0, 0.0, 0)?;
        artwork = artwork.begin_stroke(style, tap)?.finish().artwork;
    }
    let mut target = CanvasTarget::new(core, size)?;
    let request = FrameRequest {
        artwork: &artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(core, &mut target).evaluate(&request, &[PixelRect::full(size)])?;
    let path = write_png("shapes.png", &target.read(core)?)?;
    println!("shape textures: wrote {}", path.display());
    Ok(())
}

/// Along x from 40 to 680: a gentle S, a sharp turn near the middle; pressure from 0.2 up to 1
/// and back to 0.6.
fn stroke(style: StrokeStyle) -> Result<Artwork, HarnessError> {
    let samples = (0..=160u16)
        .map(|i| {
            let t = f32::from(i) / 160.0;
            let x = 40.0 + 640.0 * t;
            let turn = if t < 0.5 { 0.0 } else { (t - 0.5) * 160.0 };
            let y = 110.0 + 30.0 * (2.0 * PI * t).sin() - turn;
            let pressure = 0.2 + 0.8 * (PI * t.min(0.8) / 1.6).sin();
            let position = CanvasPoint::new(x, y)?;
            StrokeSample::new(position, pressure, 0.6, 0.0, 4 * u32::from(i))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let blank = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1])?);
    // A tap first, at full pressure: one stamp of a stamped brush.
    let tap = StrokeSample::new(CanvasPoint::new(60.0, 40.0)?, 1.0, 0.6, 0.0, 0)?;
    let tapped = blank.begin_stroke(style.clone(), tap)?.finish().artwork;
    let mut session = tapped.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}
