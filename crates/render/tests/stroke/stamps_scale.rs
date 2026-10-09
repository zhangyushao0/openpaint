//! Shapes and scatter at every scale (S45): a square stamp's and a texture stamp's rim is one
//! output pixel wide at 1x, 2x and 4x; a texture stamp of half a pixel's radius keeps the
//! texture's mean wherever it lands; each stamped built-in drawn sample by sample equals the
//! stroke so far cold, its lift's frame the commit; it exports in bands as in one piece, its view
//! tiers equal the export bit for bit, and committing and undoing it through the changed regions
//! equal cold evaluations.

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, Color, Rotation, Shape, ShapeTexture, Stamp, StrokeSample,
    StrokeStyle,
};
use openpaint_render::{
    Banding, CanvasTarget, PixelRect, RegionEvaluator, RgbaImage, changed_regions,
};

use crate::fixture::{SIZE, blank, cold, core, request, sample, with_stroke};
use crate::scale::{export, whole};
use crate::stamps::{brushes, stamped};
use crate::taper_tilt::drawn_with_a_blunt_end;
use crate::view::{drawn_while_zoomed, tiles_are_the_export};

const GRAINY: &[u8] = include_bytes!("../../assets/shape-grainy.bin");

/// One stamp of `stamp` (aspect 1, square to the canvas) of radius `radius` at `[x, y]`.
fn tap(stamp: Stamp, radius: f32, [x, y]: [f32; 2]) -> (StrokeStyle, Vec<StrokeSample>) {
    let shape = Shape::stamped((stamp, 1.0), 1.0, 0.0, (Rotation::Fixed, 0.0, 0.0), 0.0);
    let flat = [[0.0, 1.0]];
    let recipe = stamped(shape.expect("shape"), &flat, &flat);
    let style = StrokeStyle::new(Arc::new(recipe), 2.0 * radius, 1.0, Color::BLACK);
    let at = CanvasPoint::new(x, y).expect("point");
    let first = StrokeSample::new(at, 1.0, 0.0, 0.0, 0).expect("sample");
    (style.expect("style"), vec![first])
}

/// The longest run of partly covered pixels down column `x` of `image` (red channel).
fn widest_rim(image: &RgbaImage, x: u32) -> usize {
    let width = image.size()[0];
    let (mut longest, mut run) = (0, 0);
    for y in 0..image.size()[1] {
        let red = image.pixels()[((y * width + x) * 4) as usize];
        run = if red > 0 && red < 255 { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}

/// How many pixels are partly covered, and how many pairs of neighbours lie on either side of
/// half coverage: about the rim's length in pixels times `|cos| + |sin|` of its slope, the
/// pixels a rim antialiased over one pixel partly covers (two pixels' width: twice as many).
fn rim_pixels(image: &RgbaImage) -> (usize, usize) {
    let [width, height] = image.size();
    let red = |x: u32, y: u32| image.pixels()[((y * width + x) * 4) as usize];
    let (mut partial, mut crossings) = (0, 0);
    for y in 0..height {
        for x in 0..width {
            let here = red(x, y);
            partial += usize::from(here > 0 && here < 255);
            let side = here < 128;
            crossings += usize::from(x + 1 < width && (red(x + 1, y) < 128) != side);
            crossings += usize::from(y + 1 < height && (red(x, y + 1) < 128) != side);
        }
    }
    (partial, crossings)
}

#[test]
fn a_squares_and_a_textures_rim_is_one_output_pixel_wide_at_every_scale() {
    let core = core();
    // A square's top edge at y = 100.3 - 40 = 60.3; a grainy disc, holes and all.
    let square = with_stroke(&blank(), tap(Stamp::Square, 40.0, [100.0, 100.3]));
    let grainy = Stamp::Texture(ShapeTexture::Grainy);
    let texture = with_stroke(&blank(), tap(grainy, 60.0, [220.0, 100.3]));
    for scale in [1u32, 2, 4] {
        let image = whole(&core, &square, scale);
        for column in [80, 100, 120] {
            let rim = widest_rim(&image, column * scale);
            assert!(
                rim <= 1,
                "the square at {scale}x, column {column}: {rim} pixels"
            );
        }
        let (partial, crossings) = rim_pixels(&whole(&core, &texture, scale));
        let ratio = partial as f32 / crossings as f32;
        eprintln!(
            "texture at {scale}x: {partial} partial pixels, {crossings} crossings, {ratio:.2}"
        );
        assert!(
            ratio < 1.3,
            "the texture's rim at {scale}x: {ratio} pixels wide"
        );
    }
}

#[test]
fn a_texture_stamp_of_half_a_pixel_keeps_the_textures_mean_wherever_it_lands() {
    let core = core();
    let mean = GRAINY
        .iter()
        .map(|byte| ((f32::from(*byte) - 128.0) / 16.0 + 0.5).clamp(0.0, 1.0))
        .sum::<f32>()
        / GRAINY.len() as f32;
    let grainy = Stamp::Texture(ShapeTexture::Grainy);
    let mut worst = 0.0f32;
    for k in 0..16u8 {
        let offset = [f32::from(k % 4) / 4.0, f32::from(k / 4) / 4.0];
        let at = [40.5 + 16.0 * f32::from(k) + offset[0], 100.0 + offset[1]];
        let image = cold(&core, &with_stroke(&blank(), tap(grainy, 0.5, at)));
        // The coverage laid around it over the stamp's square, one pixel.
        let laid: f32 = image
            .pixels()
            .chunks(4)
            .map(|rgba| f32::from(255 - rgba[0]) / 255.0)
            .sum();
        assert!(
            (laid - mean).abs() <= 0.1 * mean,
            "landing {offset:?}: {laid} for the texture's mean {mean}"
        );
        worst = worst.max((laid - mean).abs() / mean);
    }
    eprintln!(
        "a half-pixel stamp: at most {:.1} % off the mean {mean:.3}",
        100.0 * worst
    );
}

/// A curve with rising and falling pressure, as a hand draws it.
fn curve() -> Vec<StrokeSample> {
    (0..48u16)
        .map(|i| {
            let t = f32::from(i) / 47.0;
            let x = 40.0 + 240.0 * t;
            let y = 100.0 + 50.0 * (std::f32::consts::TAU * t).sin();
            sample(
                x,
                y,
                0.3 + 0.7 * (std::f32::consts::PI * t).sin(),
                6 * u32::from(i),
            )
        })
        .collect()
}

fn style(recipe: BrushRecipe, size: f32) -> StrokeStyle {
    let color = Color::rgb(0.2, 0.3, 0.6).expect("colour");
    StrokeStyle::new(Arc::new(recipe), size, 0.9, color)
        .expect("style")
        .with_seed(0x5EED)
}

#[test]
fn every_stamped_brush_drawn_sample_by_sample_is_cold_and_lifts_to_its_commit() {
    for (name, recipe, size) in brushes() {
        eprintln!("{name}");
        let style = style(recipe, size);
        drawn_with_a_blunt_end(&style, &style, &curve());
    }
}

#[test]
fn every_stamped_brush_exports_in_bands_as_in_one_piece() {
    let core = core();
    for (name, recipe, size) in brushes() {
        let artwork = with_stroke(&blank(), (style(recipe, size), curve()));
        assert!(
            export(&core, &artwork, 1, Banding::DEFAULT) == cold(&core, &artwork),
            "{name}"
        );
        for scale in [2, 4] {
            let banding = Banding {
                rows: 37 * scale,
                columns: 61 * scale + 5,
            };
            let banded = export(&core, &artwork, scale, banding);
            assert!(
                banded == whole(&core, &artwork, scale),
                "{name} at {scale}x"
            );
        }
    }
}

#[test]
fn every_stamped_brushs_view_tiers_are_its_export() {
    let core = core();
    for (name, recipe, size) in brushes() {
        eprintln!("{name}");
        let style = style(recipe, size / 4.0);
        tiles_are_the_export(&core, &with_stroke(&blank(), (style.clone(), curve())));
        drawn_while_zoomed(&style, &style);
    }
}

#[test]
fn committing_and_undoing_stamped_strokes_through_the_changed_regions_is_cold() {
    let core = core();
    let base = blank();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut shown: Artwork = base.clone();
    let mut versions = Vec::new();
    for (_, recipe, size) in brushes() {
        versions.push(with_stroke(
            versions.last().unwrap_or(&base),
            (style(recipe, size), curve()),
        ));
    }
    versions.extend(versions.clone().into_iter().rev().skip(1));
    versions.push(base);
    for next in versions {
        let changed = changed_regions(&shown, &next);
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&next), &changed)
            .expect("evaluate");
        assert!(target.read(&core).expect("read") == cold(&core, &next));
        shown = next;
    }
}
