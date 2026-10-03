//! Stroke fixtures shared by the GPU tests. `apps/harness/src/stroke.rs` draws the same wave,
//! so `render-stroke` prints the golden hash of `golden.rs`.

use std::sync::Arc;

use openpaint_core::{
    Accumulation, Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError,
    Curve, Deposit, Dynamics, Grain, LayerCommand, Paper, PaperTexture, StrokeSample, StrokeStyle,
    Tip,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};

pub const SIZE: [u32; 2] = [320, 200];

pub fn core() -> Core {
    Core::headless().expect("a GPU adapter is required for render tests")
}

pub fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("canvas size"))
}

/// The plain round brush, the only brush before recipes.
pub fn plain(size: f32, opacity: f32, color: Color) -> StrokeStyle {
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), size, opacity, color).expect("style")
}

/// An S-curve across the canvas whose pressure rises from 0.1 to 1 and falls back: tapered
/// round ends, translucent so any seam between segments would show.
pub fn wave() -> (StrokeStyle, Vec<StrokeSample>) {
    wave_with(Arc::new(BrushRecipe::plain()))
}

/// The wave drawn with `recipe`.
pub fn wave_with(recipe: Arc<BrushRecipe>) -> (StrokeStyle, Vec<StrokeSample>) {
    let color = Color::rgb(0.12, 0.25, 0.65).expect("colour");
    let style = StrokeStyle::new(recipe, 40.0, 0.8, color).expect("style");
    let samples = (0..48u16)
        .map(|i| {
            let t = f32::from(i) / 47.0;
            let x = 30.0 + 260.0 * t;
            let y = 100.0 + 55.0 * (std::f32::consts::TAU * t).sin();
            let pressure = 0.1 + 0.9 * (std::f32::consts::PI * t).sin();
            sample(x, y, pressure, u32::from(i) * 8)
        })
        .collect();
    (style, samples)
}

/// Every stage at work: a soft edge, size over pressure and speed, opacity over pressure, a
/// coarse grain.
pub fn rich() -> Arc<BrushRecipe> {
    let curve = |points: &[[f32; 2]]| Curve::new(points).expect("curve");
    let size = Dynamics::new(
        curve(&[[0.0, 0.3], [1.0, 1.0]]),
        curve(&[[0.0, 1.0], [3.0, 0.6]]),
    );
    let opacity = Dynamics::new(curve(&[[0.0, 0.2], [1.0, 1.0]]), curve(&[[0.0, 1.0]]));
    let grain = Grain::new(0.7, 5).expect("grain");
    let recipe = BrushRecipe::new(0.6, size.expect("size"), opacity.expect("opacity"), grain);
    Arc::new(recipe.expect("recipe"))
}

/// The S16 pencil: graphite building up on the sketch paper under a flat tip.
pub fn graphite() -> Arc<BrushRecipe> {
    let curve = |points: &[[f32; 2]]| Curve::new(points).expect("curve");
    let size = Dynamics::new(curve(&[[0.0, 0.3], [1.0, 1.0]]), curve(&[[0.0, 1.0]]));
    let opacity = Dynamics::new(curve(&[[0.0, 0.29]]), curve(&[[0.0, 1.0]]));
    let paper = Paper::new(
        PaperTexture::Sketch,
        curve(&[[0.0, 0.4], [1.0, 0.95]]),
        4.5,
        0.3,
        0.3,
    );
    let deposit = Accumulation::new(Tip::Flat, 3.1, 1.0, Some(paper.expect("paper")));
    accumulating(size, opacity, deposit)
}

/// An airbrush: a bell tip building up density, no paper.
pub fn airbrush() -> Arc<BrushRecipe> {
    let curve = |points: &[[f32; 2]]| Curve::new(points).expect("curve");
    let size = Dynamics::new(curve(&[[0.0, 0.8], [1.0, 1.0]]), curve(&[[0.0, 1.0]]));
    let opacity = Dynamics::new(curve(&[[0.0, 0.05], [1.0, 1.0]]), curve(&[[0.0, 1.0]]));
    accumulating(size, opacity, Accumulation::new(Tip::Bell, 1.2, 1.0, None))
}

fn accumulating(
    size: Result<Dynamics, CoreError>,
    opacity: Result<Dynamics, CoreError>,
    deposit: Result<Accumulation, CoreError>,
) -> Arc<BrushRecipe> {
    let recipe = BrushRecipe::new(
        0.0,
        size.expect("size"),
        opacity.expect("opacity"),
        Grain::NONE,
    );
    let recipe = recipe.expect("recipe");
    let deposit = Deposit::Accumulate(deposit.expect("deposit"));
    Arc::new(recipe.with_deposit(deposit).expect("accumulating"))
}

/// A straight stroke crossing the wave, pressure rising along it.
pub fn diagonal() -> (StrokeStyle, Vec<StrokeSample>) {
    diagonal_with(Arc::new(BrushRecipe::plain()))
}

/// The diagonal drawn with `recipe`.
pub fn diagonal_with(recipe: Arc<BrushRecipe>) -> (StrokeStyle, Vec<StrokeSample>) {
    let color = Color::rgb(0.85, 0.2, 0.1).expect("colour");
    let style = StrokeStyle::new(recipe, 24.0, 0.6, color).expect("style");
    let samples = (0..40u16)
        .map(|i| {
            let t = f32::from(i) / 39.0;
            sample(
                60.0 + 200.0 * t,
                30.0 + 140.0 * t,
                0.3 + 0.7 * t,
                u32::from(i) * 8,
            )
        })
        .collect();
    (style, samples)
}

pub fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("canvas point");
    StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
}

/// `artwork` with the stroke drawn and finished on its active layer.
pub fn with_stroke(
    artwork: &Artwork,
    (style, samples): (StrokeStyle, Vec<StrokeSample>),
) -> Artwork {
    let mut session = artwork.begin_stroke(style, samples[0]).expect("begin");
    session.extend(&samples[1..]).expect("extend");
    session.finish().artwork
}

/// `artwork` with a new active layer of `opacity` above the active one.
pub fn with_layer(artwork: &Artwork, opacity: f32) -> Artwork {
    let created = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .expect("create layer")
        .artwork;
    let layer = created.active();
    let command = LayerCommand::SetOpacity { layer, opacity };
    created
        .apply(ArtworkCommand::Layer(command))
        .expect("layer opacity")
        .artwork
}

pub fn request(artwork: &Artwork) -> FrameRequest<'_> {
    FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    }
}

/// Evaluates the whole of `artwork` into a fresh target and reads it back.
pub fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    target.read(core).expect("readback")
}

/// The RGBA pixel at `[x, y]`.
pub fn pixel(image: &RgbaImage, [x, y]: [u32; 2]) -> [u8; 4] {
    let at = ((y * image.size()[0] + x) * 4) as usize;
    let mut out = [0; 4];
    out.copy_from_slice(&image.pixels()[at..at + 4]);
    out
}
