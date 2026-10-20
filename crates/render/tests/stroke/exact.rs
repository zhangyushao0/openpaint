//! Exact pixels where a formula gives them: a flat fill is its colour; a solid stamp at an opacity
//! is that share of its colour over the paper; each blend mode of a layer over another is the
//! W3C formula on the two colours, and a faded Normal layer their mix; a point adjustment of a
//! fill is the adjustment's formula on its colour. Each is read at a pixel well inside, at 1x on
//! the canvas and at 2x and 4x in the export, rounded to 8 bits as the finish rounds.

use std::sync::Arc;

use openpaint_core::{
    Adjustment, Artwork, BlendMode, BrushRecipe, Color, Hsb, LayerCommand, Rotation, Shape, Stamp,
    StrokeStyle,
};

use crate::fixture::{SIZE, blank, cold, core, pixel, sample, with_layer};
use crate::raster::{layer, region};
use crate::scale::whole;

const SOURCE: [f64; 3] = [0.8, 0.35, 0.6];
const UNDER: [f64; 3] = [0.4, 0.7, 0.2];

fn color([r, g, b]: [f64; 3]) -> Color {
    Color::rgb(r as f32, g as f32, b as f32).expect("colour")
}

/// The 8-bit value the finish stores for `value`, checked to lie clear of a rounding edge so the
/// float order of the GPU cannot turn it.
fn level(value: f64) -> u8 {
    let scaled = value.clamp(0.0, 1.0) * 255.0;
    let fraction = scaled - scaled.floor();
    assert!(
        (fraction - 0.5).abs() > 0.01,
        "{value} lies on a rounding edge"
    );
    (scaled + 0.5).floor() as u8
}

fn opaque(rgb: [f64; 3]) -> [u8; 4] {
    [level(rgb[0]), level(rgb[1]), level(rgb[2]), 255]
}

/// `artwork`'s active layer with the rectangle from (100, 60) to (220, 140) filled with `rgb`.
fn filled(artwork: &Artwork, rgb: [f64; 3]) -> Artwork {
    let rectangle = region(|x, y| (100..220).contains(&x) && (60..140).contains(&y));
    (artwork.fill_region(rectangle, color(rgb)))
        .expect("fill")
        .artwork
}

/// The pixel at the canvas's middle, at 1x and in the 2x and 4x exports: all of them `wanted`.
fn shows(artwork: &Artwork, wanted: [u8; 4], what: &str) {
    let core = core();
    let middle = SIZE.map(|side| side / 2);
    assert_eq!(pixel(&cold(&core, artwork), middle), wanted, "{what} at 1x");
    for scale in [2, 4] {
        let at = middle.map(|side| side * scale);
        let exported = whole(&core, artwork, scale);
        assert_eq!(pixel(&exported, at), wanted, "{what} at {scale}x");
    }
}

#[test]
fn a_flat_fill_is_its_colour() {
    shows(&filled(&blank(), SOURCE), opaque(SOURCE), "a fill");
}

#[test]
fn a_solid_stamp_lays_its_opacity_of_its_colour_over_the_paper() {
    let disc = Shape::stamped(
        (Stamp::Disc, 1.0),
        0.2,
        0.0,
        (Rotation::Fixed, 0.0, 0.0),
        0.0,
    )
    .expect("disc");
    let recipe = BrushRecipe::plain().with_shape(disc).expect("stamped");
    let style = StrokeStyle::new(Arc::new(recipe), 60.0, 0.4, color(SOURCE)).expect("style");
    // A tap leaves one stamp: 30 px of radius about the middle.
    let middle = SIZE.map(|side| side as f32 / 2.0);
    let session =
        (blank().begin_stroke(style, sample(middle[0], middle[1], 1.0, 0))).expect("begin");
    let tapped = session.finish().artwork;
    let laid = SOURCE.map(|c| 0.4 * c + 0.6);
    shows(&tapped, opaque(laid), "a stamp at 40 %");
}

/// `UNDER` filled on the bottom layer, `SOURCE` on a layer over it by `blend` at `opacity`.
fn over(blend: BlendMode, opacity: f32) -> Artwork {
    let under = filled(&blank(), UNDER);
    let top = with_layer(&under, opacity);
    let id = top.active();
    filled(
        &layer(&top, LayerCommand::SetBlend { layer: id, blend }),
        SOURCE,
    )
}

#[test]
fn each_blend_mode_of_one_colour_over_another_is_its_formula() {
    let each = |f: fn(f64, f64) -> f64| -> [f64; 3] { [0, 1, 2].map(|c| f(SOURCE[c], UNDER[c])) };
    let modes: [(BlendMode, [f64; 3]); 4] = [
        (BlendMode::Normal, each(|s, _| s)),
        (BlendMode::Multiply, each(|s, d| s * d)),
        (BlendMode::Screen, each(|s, d| s + d - s * d)),
        (
            BlendMode::Overlay,
            each(|s, d| {
                if 2.0 * d <= 1.0 {
                    2.0 * s * d
                } else {
                    1.0 - 2.0 * (1.0 - d) * (1.0 - s)
                }
            }),
        ),
    ];
    for (blend, wanted) in modes {
        shows(&over(blend, 1.0), opaque(wanted), &format!("{blend:?}"));
    }
    let mixed = each(|s, d| 0.5 * s + 0.5 * d);
    shows(
        &over(BlendMode::Normal, 0.5),
        opaque(mixed),
        "Normal at 50 %",
    );
}

#[test]
fn a_point_adjustment_of_a_fill_is_its_formula() {
    let luma = 0.299 * SOURCE[0] + 0.587 * SOURCE[1] + 0.114 * SOURCE[2];
    for (brightness, wanted) in [(0.5, luma), (0.25, 0.5 * luma), (0.75, 0.5 + 0.5 * luma)] {
        // Saturation 0: the Rec. 601 grey; brightness below 0.5 towards black, above to white.
        let adjustment = Adjustment::Hsb(Hsb::new(0.5, 0.0, brightness));
        let adjusted = (filled(&blank(), SOURCE).adjust_region(None, adjustment))
            .expect("adjusted")
            .artwork;
        shows(
            &adjusted,
            opaque([wanted; 3]),
            &format!("grey at {brightness}"),
        );
    }
}
