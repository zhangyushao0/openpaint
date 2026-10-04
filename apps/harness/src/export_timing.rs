//! `export-timing`: a busy artwork at the tablet's 2732 x 2048 preset (three layers, 2,000 ink
//! strokes, 300 pencil strokes, 40 fills), exported at 1x, 2x and 4x through the same bands and
//! streaming PNG encoder the app uses; prints the planning (scene, smudges and binning on the
//! CPU), GPU and readback, and encoding times. Run with `--release` for numbers that mean
//! anything.

use std::sync::Arc;
use std::time::Instant;

use openpaint_core::{
    Accumulation, Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError,
    Curve, Deposit, Dynamics, Grain, LayerCommand, Paper, PaperTexture, RegionMask, StrokeSample,
    StrokeStyle, Tip,
};
use openpaint_render::{Banding, Core};
use openpaint_runtime::ImageFormat;
use openpaint_store::ImageEncoder;

use crate::error::HarnessError;

const SIZE: [u32; 2] = [2732, 2048];

pub fn run() -> Result<(), HarnessError> {
    let started = Instant::now();
    let artwork = busy()?;
    println!("built the artwork in {} ms", started.elapsed().as_millis());
    let core = Core::headless()?;
    println!("adapter {}", core.adapter());
    for scale in [1, 2, 4, 1] {
        let started = Instant::now();
        let mut bands = core.export_bands(&artwork, Color::WHITE, scale, Banding::DEFAULT)?;
        let [width, height] = bands.size();
        let mut encoder = ImageEncoder::new(ImageFormat::Png, [width, height])
            .map_err(|_| HarnessError::Missing("the PNG encoder refused the size"))?;
        let mut encoding = std::time::Duration::ZERO;
        for band in &mut bands {
            let band = band?;
            let at = Instant::now();
            encoder
                .rows(band.pixels())
                .map_err(|_| HarnessError::Missing("the PNG encoder refused a band"))?;
            encoding += at.elapsed();
        }
        let (planned, waited) = bands.timing();
        let at = Instant::now();
        let file = encoder
            .finish()
            .map_err(|_| HarnessError::Missing("the PNG encoder failed"))?;
        encoding += at.elapsed();
        println!(
            "{scale}x {width} x {height}: {} ms in all; planned and submitted {} ms, GPU and \
             readback {} ms, encoded {} ms, {} bytes",
            started.elapsed().as_millis(),
            planned.as_millis(),
            waited.as_millis(),
            encoding.as_millis(),
            file.len()
        );
    }
    Ok(())
}

/// A deterministic pseudo-random sequence in `0..1`.
struct Random(u64);

impl Random {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn busy() -> Result<Artwork, CoreError> {
    let mut random = Random(29);
    let mut artwork = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1])?);
    let ink = Arc::new(BrushRecipe::plain());
    let pencil = pencil()?;
    for layer in 0..3 {
        if layer > 0 {
            artwork = artwork
                .apply(ArtworkCommand::Layer(LayerCommand::Create))?
                .artwork;
        }
        for i in 0..700 {
            let recipe = if i % 7 == 0 { &pencil } else { &ink };
            artwork = stroke(&artwork, recipe, &mut random)?;
        }
        for _ in 0..13 {
            artwork = fill(&artwork, &mut random)?;
        }
    }
    Ok(artwork)
}

fn stroke(
    artwork: &Artwork,
    recipe: &Arc<BrushRecipe>,
    random: &mut Random,
) -> Result<Artwork, CoreError> {
    let color = Color::rgb(random.next(), random.next(), random.next())?;
    let style = StrokeStyle::new(Arc::clone(recipe), 4.0 + 36.0 * random.next(), 1.0, color)?;
    let mut at = [
        random.next() * SIZE[0] as f32,
        random.next() * SIZE[1] as f32,
    ];
    let mut samples = Vec::new();
    for i in 0..40u32 {
        at[0] = (at[0] + 30.0 * (random.next() - 0.5)).clamp(0.0, SIZE[0] as f32 - 1.0);
        at[1] = (at[1] + 30.0 * (random.next() - 0.5)).clamp(0.0, SIZE[1] as f32 - 1.0);
        let position = CanvasPoint::new(at[0], at[1])?;
        samples.push(StrokeSample::new(
            position,
            0.3 + 0.7 * random.next(),
            0.0,
            0.0,
            i * 8,
        )?);
    }
    let mut session = artwork.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}

fn fill(artwork: &Artwork, random: &mut Random) -> Result<Artwork, CoreError> {
    let (x, y) = (random.next() * 2400.0, random.next() * 1700.0);
    let (w, h) = (40.0 + 300.0 * random.next(), 40.0 + 300.0 * random.next());
    let inside: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| {
            let (px, py) = ((at % SIZE[0]) as f32, (at / SIZE[0]) as f32);
            px >= x && px < x + w && py >= y && py < y + h
        })
        .collect();
    let mask = RegionMask::from_pixels(artwork.canvas(), &inside)?;
    let color = Color::rgb(random.next(), random.next(), random.next())?;
    Ok(artwork.fill_region(mask, color)?.artwork)
}

/// The S16 pencil: graphite on the sketch paper under a flat tip.
fn pencil() -> Result<Arc<BrushRecipe>, CoreError> {
    let size = Dynamics::new(
        Curve::new(&[[0.0, 0.3], [1.0, 1.0]])?,
        Curve::new(&[[0.0, 1.0]])?,
    )?;
    let opacity = Dynamics::new(Curve::new(&[[0.0, 0.29]])?, Curve::new(&[[0.0, 1.0]])?)?;
    let contact = Curve::new(&[[0.0, 0.4], [1.0, 0.95]])?;
    let paper = Paper::new(PaperTexture::Sketch, contact, 4.5, 0.3, 0.3)?;
    let deposit = Accumulation::new(Tip::Flat, 3.1, 1.0, Some(paper))?;
    let recipe = BrushRecipe::new(0.0, size, opacity, Grain::NONE)?;
    Ok(Arc::new(recipe.with_deposit(Deposit::Accumulate(deposit))?))
}
