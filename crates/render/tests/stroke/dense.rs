//! A dense place (300 crossing round strokes; 120 Textured Round ones) drawn in small regions
//! equals it drawn in one, bit for bit, at 1x, 2x and 4x: as an export cut into many small bands
//! and pieces against one piece, and on the canvas in many small parts against the whole. A pixel
//! never depends on which regions are drawn with it.

use std::f32::consts::PI;
use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasSize, Color, Curve, Dynamics, Grain, Rotation, Shape, ShapeTexture,
    Stamp, StrokeSample, StrokeStyle,
};
use openpaint_render::{Banding, CanvasTarget, PixelRect};

use crate::fixture::{SIZE, cold, core, request, sample};
use crate::scale::{export, whole};

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

/// `artwork` exported in bands of 23 rows cut into pieces of 29 columns at 1x, 2x and 4x equals
/// one piece; on the canvas, 40 x 30 parts drawn one by one equal the whole drawn at once.
fn small_regions_equal_one(artwork: &Artwork, what: &str) {
    let core = core();
    for scale in [1, 2, 4] {
        let banding = Banding {
            rows: 23,
            columns: 29,
        };
        let pieces = export(&core, artwork, scale, banding);
        assert!(pieces == whole(&core, artwork, scale), "{what} at {scale}x");
    }
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    for y in (0..SIZE[1] as i32).step_by(30) {
        for x in (0..SIZE[0] as i32).step_by(40) {
            let part = PixelRect {
                min: [x, y],
                max: [x + 40, y + 30],
            };
            target
                .draw(&core, &request(artwork), &[part])
                .expect("a part");
        }
    }
    let parts = target.read(&core).expect("read");
    assert!(parts == cold(&core, artwork), "{what}: parts = whole");
}

#[test]
fn a_dense_place_drawn_in_small_regions_equals_one_bit_for_bit() {
    small_regions_equal_one(&dense(300, round), "300 crossing rounds");
    small_regions_equal_one(&dense(120, textured), "120 crossing Textured Rounds");
}
