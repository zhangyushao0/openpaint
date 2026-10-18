//! The raster core (S57's probe, S59's product path): every source of an artwork's layers
//! (strokes of every recipe stage, erasing, Alpha Lock, a selection's clip, a fill, a cut, a
//! turned picture, a grey mask, clipping chains, the four blend modes, layer opacity; S65's
//! records and groups: `raster_kinds.rs`)
//! exported by the raster core equals the evaluator's image within one 8-bit level away from
//! antialiased edges (D86's rule) at 1x, 2x, 4x and 8x, draws alike bit for bit every time and
//! in bands as in one piece; S57's probe scene against the evaluator likewise.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, Color, LayerCommand, MaskCommand, Patch,
    Placement, RegionMask, StrokeSample, StrokeStyle, StrokeTool,
};
use openpaint_render::{Banding, Core, ImageDifference, RgbaImage, image_difference};

use crate::fixture::{
    SIZE, airbrush, blank, core, diagonal_with, graphite, plain, rich, sample, wave, wave_with,
    with_layer, with_stroke,
};
use crate::scale::{export, whole};
use crate::stamps::brushes;

/// A loop of 40 samples about `centre`, pressed from 0.4 to 1.
fn ring(centre: [f32; 2], radius: f32) -> Vec<StrokeSample> {
    (0..40u16)
        .map(|i| {
            let t = f32::from(i) / 39.0;
            let angle = std::f32::consts::TAU * 0.9 * t;
            let [x, y] = [
                centre[0] + radius * angle.cos(),
                centre[1] + radius * angle.sin(),
            ];
            sample(x, y, 0.4 + 0.6 * t, u32::from(i) * 8)
        })
        .collect()
}

/// S57's probe scene: a hard and a soft covering capsule with grain, the pencil's graphite on
/// paper, a bell airbrush, four stamped built-ins.
fn strokes() -> Artwork {
    let mut artwork = with_stroke(&blank(), wave());
    artwork = with_stroke(&artwork, wave_with(rich()));
    artwork = with_stroke(&artwork, diagonal_with(graphite()));
    artwork = with_stroke(&artwork, diagonal_with(airbrush()));
    let color = Color::rgb(0.3, 0.6, 0.2).expect("colour");
    for (k, (_, recipe, size)) in brushes().into_iter().take(4).enumerate() {
        let style = StrokeStyle::new(Arc::new(recipe), size * 0.5, 0.9, color)
            .expect("style")
            .with_seed(k as u32 + 7);
        let centre = [70.0 + 60.0 * k as f32, 60.0 + 25.0 * (k % 2) as f32];
        artwork = with_stroke(&artwork, (style, ring(centre, 30.0)));
    }
    artwork
}

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

pub fn region(inside: impl Fn(u32, u32) -> bool) -> RegionMask {
    let pixels: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| inside(at % SIZE[0], at / SIZE[0]))
        .collect();
    RegionMask::from_pixels(canvas(), &pixels).expect("mask")
}

pub fn color(r: f32, g: f32, b: f32) -> Color {
    Color::rgb(r, g, b).expect("colour")
}

pub fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    artwork
        .apply(ArtworkCommand::Layer(command))
        .expect("layer")
        .artwork
}

/// A line of `style` across the canvas at height `y`.
fn line(style: StrokeStyle, y: f32) -> (StrokeStyle, Vec<StrokeSample>) {
    let samples = (0..12u16)
        .map(|i| {
            sample(
                20.0 + 25.0 * f32::from(i),
                y + 6.0 * f32::from(i % 3),
                0.9,
                u32::from(i) * 8,
            )
        })
        .collect();
    (style, samples)
}

/// The probe's strokes, an eraser, a stroke inside a lasso, a fill, a cut and Alpha Lock on the
/// first layer.
fn first_layer() -> Artwork {
    let mut artwork = strokes();
    let (style, samples) = diagonal_with(rich());
    artwork = with_stroke(&artwork, (style.with_tool(StrokeTool::Erase), samples));
    let lasso = region(|x, y| (x as i32 - 160).abs() + (y as i32 - 100).abs() < 70);
    let (style, samples) = line(plain(18.0, 0.9, color(0.9, 0.7, 0.1)), 90.0);
    let session = artwork.begin_stroke(style, samples[0]).expect("begin");
    let mut session = session.clipped(lasso).expect("clipped");
    session.extend(&samples[1..]).expect("extend");
    artwork = session.finish().artwork;
    let disc = region(|x, y| {
        let (dx, dy) = (x as f32 - 230.0, y as f32 - 60.0);
        dx * dx + dy * dy <= 900.0
    });
    artwork = (artwork.fill_region(disc, color(0.9, 0.3, 0.1)))
        .expect("fill")
        .artwork;
    let cut = region(|x, y| (40..70).contains(&x) && (20..180).contains(&y));
    artwork = artwork.cut(Some(&cut)).expect("cut").artwork;
    let id = artwork.active();
    artwork = layer(
        &artwork,
        LayerCommand::SetAlphaLocked {
            layer: id,
            locked: true,
        },
    );
    with_stroke(
        &artwork,
        line(plain(30.0, 0.8, color(0.1, 0.2, 0.9)), 130.0),
    )
}

/// A 64 x 48 picture: an opaque gradient with a half transparent stripe.
pub fn picture() -> Patch {
    let mut pixels = Vec::new();
    for y in 0..48u32 {
        for x in 0..64u32 {
            let pixel = if (20..28).contains(&y) {
                [60, 20, 10, 128]
            } else {
                [(x * 4) as u8, (y * 5) as u8, 150, 255]
            };
            pixels.extend(pixel);
        }
    }
    Patch::new(canvas(), [40, 120], [64, 48], pixels).expect("patch")
}

/// `first_layer`; a turned picture with a grey mask on a Screen layer at 80 %; a layer clipping
/// to it by Multiply and one by Overlay; an Overlay layer at 70 % on top.
pub fn every_raster_source() -> Artwork {
    let created = with_layer(&first_layer(), 0.8);
    let turn = Placement::around([72.0, 144.0], 1.3, 0.4, [false, false]).expect("turn");
    let placed = (created.transform_region(None, picture(), turn))
        .expect("placed")
        .artwork;
    let base = placed.active();
    let masked = (placed.apply(ArtworkCommand::Mask(MaskCommand::Add(base))))
        .expect("mask")
        .artwork;
    let grey = (masked.fill_region(region(|x, _| x < 70), color(0.4, 0.4, 0.4)))
        .expect("grey")
        .artwork;
    let mut artwork = with_stroke(&grey, line(plain(14.0, 1.0, Color::BLACK), 150.0));
    artwork = layer(&artwork, LayerCommand::Select(base));
    artwork = layer(
        &artwork,
        LayerCommand::SetBlend {
            layer: base,
            blend: BlendMode::Screen,
        },
    );
    artwork = with_stroke(&artwork, line(plain(24.0, 0.7, color(0.2, 0.5, 0.3)), 40.0));
    for (blend, y, tint) in [
        (BlendMode::Multiply, 110.0, 0.9),
        (BlendMode::Overlay, 60.0, 0.3),
    ] {
        artwork = with_layer(&artwork, 0.9);
        let top = artwork.active();
        artwork = layer(
            &artwork,
            LayerCommand::SetClipping {
                layer: top,
                clipping: true,
            },
        );
        artwork = layer(&artwork, LayerCommand::SetBlend { layer: top, blend });
        artwork = with_stroke(&artwork, line(plain(40.0, 1.0, color(tint, 0.6, 0.9)), y));
    }
    artwork = with_layer(&artwork, 0.7);
    let top = artwork.active();
    artwork = layer(
        &artwork,
        LayerCommand::SetBlend {
            layer: top,
            blend: BlendMode::Overlay,
        },
    );
    with_stroke(
        &artwork,
        (
            plain(50.0, 1.0, color(0.8, 0.2, 0.6)),
            ring([160.0, 100.0], 60.0),
        ),
    )
}

/// The evaluator's whole output of `artwork` at `scale` (what exports were before S59).
pub fn evaluated(core: &Core, artwork: &Artwork, scale: u32) -> RgbaImage {
    let size = SIZE.map(|side| side * scale);
    (core.evaluated_image(artwork, (scale, [0, 0]), size)).expect("evaluated")
}

/// `rastered` against `reference` by D86's rule: no pixel away from antialiased edges more than
/// one 8-bit level off.
pub fn assert_near(reference: &RgbaImage, rastered: &RgbaImage, what: &str) -> ImageDifference {
    let found = image_difference(reference, rastered).expect("same size");
    println!("{what}: {found:?}");
    assert_eq!(found.beyond_one_inside, 0, "{what}: {found:?}");
    assert!(
        found.mean.iter().all(|mean| *mean < 0.05),
        "{what}: {found:?}"
    );
    found
}

#[test]
fn every_raster_source_exports_within_a_level_of_the_evaluator_away_from_edges() {
    let core = core();
    let artwork = every_raster_source();
    for scale in [1, 2, 4, 8] {
        let rastered = whole(&core, &artwork, scale);
        let pixels = rastered.pixels().as_chunks::<4>().0;
        let painted = pixels.iter().filter(|pixel| **pixel != [255; 4]).count();
        let share = painted as f64 / pixels.len() as f64;
        assert!(share > 0.2, "{scale}x paints {share}");
        assert_near(
            &evaluated(&core, &artwork, scale),
            &rastered,
            &format!("{scale}x"),
        );
    }
}

#[test]
fn the_raster_core_draws_alike_every_time_and_in_bands_as_in_one_piece() {
    let core = core();
    let artwork = every_raster_source();
    for scale in [1, 2, 4] {
        let one = whole(&core, &artwork, scale);
        assert!(one == whole(&core, &artwork, scale), "{scale}x again");
        let banding = Banding {
            rows: 37 * scale,
            columns: 61 * scale + 5,
        };
        assert!(
            export(&core, &artwork, scale, banding) == one,
            "{scale}x bands"
        );
    }
}

/// The probe's image against the evaluator's over the target of `SIZE` output pixels at
/// `scale`, the canvas's middle in its middle; and the share of the probe's pixels painted.
fn compared(artwork: &Artwork, scale: u32) -> (ImageDifference, f64) {
    let core = core();
    let at = SIZE.map(|side| side / 2 - side / (2 * scale));
    let (_, evaluated) = core
        .time_evaluation(artwork, (scale, at), SIZE, 1)
        .expect("evaluate");
    let raster = core
        .time_raster(artwork, (scale, at), SIZE, (1, 1024))
        .expect("raster");
    assert!(raster.repeatable, "{scale}x: it draws alike every time");
    let pixels = raster.image.pixels().as_chunks::<4>().0;
    let painted = pixels.iter().filter(|p| **p != [255; 4]).count();
    let share = painted as f64 / pixels.len() as f64;
    let found = image_difference(&evaluated, &raster.image).expect("same size");
    (found, share)
}

#[test]
fn every_stroke_kind_rasters_within_a_level_of_the_evaluator_away_from_edges() {
    let artwork = strokes();
    for scale in [1, 4, 8] {
        let (found, painted) = compared(&artwork, scale);
        println!("{scale}x: {painted:.3} painted, {found:?}");
        assert!(painted > 0.2, "{scale}x: the probe paints {painted}");
        assert_eq!(found.beyond_one_inside, 0, "{scale}x: {found:?}");
        assert!(
            found.largest.iter().all(|off| *off <= 2),
            "{scale}x: {found:?}"
        );
        assert!(
            found.mean.iter().all(|mean| *mean < 0.01),
            "{scale}x: {found:?}"
        );
    }
}
