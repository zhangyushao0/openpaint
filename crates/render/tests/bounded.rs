//! Bounded dispatches (S49): an evaluation cut into many small dispatches equals one dispatch bit
//! for bit, on a dense place of 300 crossing strokes, in the canvas and in exports at 1x, 2x and
//! 4x. A screen frame's share of the waiting tiles is `frame_budget.rs`'s (S53).
//!
//! A test binary of its own: the tests change the device's dispatch bound, which every
//! evaluation on the device follows, so they run one at a time on a device no other test uses.
//! Textured stamps compare within one level in a few channels, the development GPU's two results
//! (STATUS Known limits), as S48's tests do; plain strokes bit for bit.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, Curve, Dynamics, Grain, Rotation, Shape,
    ShapeTexture, Stamp, StrokeSample, StrokeStyle,
};
use openpaint_render::{
    Banding, CanvasTarget, Core, DISPATCH_WORK, FrameRequest, PixelRect, RegionEvaluator, RgbaImage,
};

const SIZE: [u32; 2] = [320, 200];

fn core() -> &'static Core {
    static CORE: OnceLock<Core> = OnceLock::new();
    CORE.get_or_init(|| Core::headless().expect("a GPU adapter is required for render tests"))
}

/// The tests one at a time: each sets the device's dispatch bound.
fn alone() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
}

/// Stroke `k` of many crossing the canvas's middle at every angle (as `pen-many` lays them).
fn crossing(k: u32) -> Vec<StrokeSample> {
    let angle = k as f32 * 2.399_963_2;
    let shift = [((k * 37) % 61) as f32 - 30.0, ((k * 59) % 41) as f32 - 20.0];
    (0..20u16)
        .map(|i| {
            let t = f32::from(i) / 19.0;
            let s = 180.0 * (t - 0.5);
            let x = 160.0 + shift[0] + s * angle.cos();
            let y = 100.0 + shift[1] + s * angle.sin();
            sample(x, y, 0.6 + 0.3 * (PI * t).sin(), u32::from(i) * 8)
        })
        .collect()
}

fn hue(k: u32) -> Color {
    let share = (k % 7) as f32 / 7.0;
    Color::rgb(0.2 + 0.6 * share, 0.3, 0.8 - 0.6 * share).expect("colour")
}

fn round(k: u32) -> StrokeStyle {
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), 24.0, 0.7, hue(k)).expect("style")
}

/// The Textured Round as `crates/app/brushes/textured-round.toml` holds it.
fn textured(k: u32) -> StrokeStyle {
    let curve = |points: &[[f32; 2]]| Curve::new(points).expect("curve");
    let flat = curve(&[[0.0, 1.0]]);
    let size = Dynamics::new(curve(&[[0.0, 0.35], [1.0, 1.0]]), flat.clone()).expect("size");
    let opacity = Dynamics::new(curve(&[[0.0, 0.25], [1.0, 1.0]]), flat).expect("opacity");
    let chalk = Shape::stamped(
        (Stamp::Texture(ShapeTexture::Chalk), 1.0),
        0.1,
        0.0,
        (Rotation::Fixed, 0.0, 15.0),
        0.0,
    )
    .expect("shape");
    let grain = Grain::new(0.3, 5).expect("grain");
    let recipe = BrushRecipe::new(0.0, size, opacity, grain)
        .and_then(|recipe| recipe.with_shape(chalk))
        .expect("recipe");
    StrokeStyle::new(Arc::new(recipe), 24.0, 0.9, hue(k))
        .expect("style")
        .with_seed(k)
}

/// `count` crossing strokes of `style(k)` over one place.
fn dense(count: u32, style: impl Fn(u32) -> StrokeStyle) -> Artwork {
    let blank = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
    (0..count).fold(blank, |artwork, k| {
        let samples = crossing(k);
        let mut stroke = artwork.begin_stroke(style(k), samples[0]).expect("begin");
        stroke.extend(&samples[1..]).expect("extend");
        stroke.finish().artwork
    })
}

fn request(artwork: &Artwork) -> FrameRequest<'_> {
    FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    }
}

/// The canvas evaluated whole into a new target, then the output at 2x and 4x in one piece.
fn outputs(core: &Core, artwork: &Artwork) -> Vec<RgbaImage> {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut images = vec![target.read(core).expect("read")];
    for scale in [1, 2, 4] {
        let banding = Banding {
            rows: SIZE[1] * scale,
            columns: SIZE[0] * scale,
        };
        let bands = core.export_bands(artwork, Color::WHITE, scale, banding);
        images.push(bands.expect("bands").image().expect("export"));
    }
    images
}

/// The outputs evaluated in one dispatch each, and bounded to `work` a dispatch, with how many
/// dispatches the bounded ones took.
fn one_and_bounded(
    core: &Core,
    artwork: &Artwork,
    work: u64,
) -> (Vec<RgbaImage>, Vec<RgbaImage>, u64) {
    core.set_dispatch_work(u64::MAX);
    let one = outputs(core, artwork);
    core.set_dispatch_work(work);
    let before = core.dispatches();
    let bounded = outputs(core, artwork);
    let dispatches = core.dispatches() - before;
    core.set_dispatch_work(DISPATCH_WORK);
    (one, bounded, dispatches)
}

#[test]
fn a_bounded_evaluation_equals_one_dispatch_bit_for_bit_at_every_scale() {
    let _one = alone();
    let core = core();
    let artwork = dense(300, round);
    let (one, bounded, dispatches) = one_and_bounded(core, &artwork, 300);
    eprintln!("300 crossing strokes in {dispatches} bounded dispatches");
    assert!(dispatches > 100, "{dispatches} dispatches");
    for (index, (a, b)) in one.iter().zip(&bounded).enumerate() {
        assert!(
            a == b,
            "output {index} differs between one dispatch and bounded ones"
        );
    }
}

#[test]
fn bounded_textured_rounds_equal_one_dispatch_within_the_gpus_level() {
    let _one = alone();
    let core = core();
    let artwork = dense(120, textured);
    let (one, bounded, dispatches) = one_and_bounded(core, &artwork, 300);
    assert!(dispatches > 50, "{dispatches} dispatches");
    for (index, (a, b)) in one.iter().zip(&bounded).enumerate() {
        let apart: Vec<u8> = (a.pixels().iter().zip(b.pixels()))
            .map(|(x, y)| x.abs_diff(*y))
            .collect();
        let differ = apart.iter().filter(|d| **d > 0).count();
        assert!(
            apart.iter().all(|d| *d <= 1) && differ * 50 <= apart.len(),
            "output {index}: {differ} channels differ"
        );
    }
}
