//! Fill edges against line art (S30): a ColorDrop flood (`openpaint_raster`, the fill worker's
//! own) inside a closed ink circle, exported at 1x, 2x and 4x. Between the fill and the ink
//! nothing of the paper shows, for thick and 1.5-pixel lines and small circles; outside the
//! line the fill reaches at most a pixel and a half (where a thin line's antialiased pixels
//! were flooded); banded exports equal one piece.

mod gpu;

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, LayerId, StrokeSample, StrokeStyle,
};
use openpaint_render::{Banding, Core, RgbaImage};

const SIZE: [u32; 2] = [128, 128];
const CENTRE: [f32; 2] = [64.3, 63.6];
/// The fill's colour: its blue is far below the paper's, so paper shows in blue.
const FILL: [f32; 3] = [0.95, 0.75, 0.2];

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

/// A closed ink circle of `radius` and line `width` about `CENTRE`, filled by a ColorDrop at
/// its centre (threshold 0.6, the default).
fn filled(core: &Core, radius: f32, width: f32) -> Artwork {
    let style =
        StrokeStyle::new(Arc::new(BrushRecipe::plain()), width, 1.0, Color::BLACK).expect("ink");
    let samples: Vec<StrokeSample> = (0..=180u16)
        .map(|i| {
            let a = std::f32::consts::TAU * f32::from(i) / 180.0;
            let point =
                CanvasPoint::new(CENTRE[0] + radius * a.cos(), CENTRE[1] + radius * a.sin());
            StrokeSample::new(point.expect("point"), 1.0, 0.0, 0.0, u32::from(i) * 8)
                .expect("sample")
        })
        .collect();
    let mut session = Artwork::new(canvas())
        .begin_stroke(style, samples[0])
        .expect("begin");
    session.extend(&samples[1..]).expect("extend");
    let drawn = session.finish().artwork;
    let pixels = core.layer_pixels(&drawn, LayerId::new(1)).expect("pixels");
    let seed = [CENTRE[0] as u32, CENTRE[1] as u32];
    let mask =
        openpaint_raster::connected_region(canvas(), pixels.pixels(), seed, 0.6).expect("flood");
    let [r, g, b] = FILL;
    let color = Color::rgb(r, g, b).expect("colour");
    drawn.fill_region(mask, color).expect("fill").artwork
}

fn export(core: &Core, artwork: &Artwork, scale: u32, banding: Banding) -> RgbaImage {
    core.export_bands(artwork, Color::WHITE, scale, banding)
        .expect("bands")
        .image()
        .expect("image")
}

/// Rec. 601 luma of 8-bit values.
fn luma([r, g, b]: [f32; 3]) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// How much lighter than the fill any pixel inside the line's outer edge less an output pixel
/// is (0 to 1, the share of the way to the paper's white: a light seam between fill and ink),
/// and how far past the line's outer edge the fill shows (canvas pixels).
fn measured(image: &RgbaImage, scale: u32, radius: f32, width: f32) -> (f32, f32) {
    let fill_luma = luma(FILL.map(|c| (c * 255.0).round()));
    let [w, h] = image.size();
    let outer = radius + width / 2.0;
    let (mut paper, mut beyond) = (0.0f32, 0.0f32);
    for y in 0..h {
        for x in 0..w {
            let at = ((y * w + x) * 4) as usize;
            let [r, g, b] = [0, 1, 2].map(|c| f32::from(image.pixels()[at + c]));
            let centre = [
                (x as f32 + 0.5) / scale as f32,
                (y as f32 + 0.5) / scale as f32,
            ];
            let rho = (centre[0] - CENTRE[0]).hypot(centre[1] - CENTRE[1]);
            if rho <= outer - 1.0 / scale as f32 {
                let light = (luma([r, g, b]) - fill_luma).max(0.0) / (255.0 - fill_luma);
                paper = paper.max(light);
            }
            // Fill colour shows: red well above green and blue.
            if rho > outer && r - b > 40.0 {
                beyond = beyond.max(rho - outer);
            }
        }
    }
    (paper, beyond)
}

#[test]
fn no_paper_shows_between_a_fill_and_its_ink_line_at_any_scale() {
    let core = gpu::core();
    let mut failed = Vec::new();
    for (radius, width) in [(40.0, 6.0), (40.0, 1.5), (12.0, 3.0), (6.0, 2.0)] {
        let artwork = filled(&core, radius, width);
        for scale in [1, 2, 4] {
            let image = export(&core, &artwork, scale, Banding::DEFAULT);
            let (paper, beyond) = measured(&image, scale, radius, width);
            eprintln!(
                "radius {radius}, line {width} at {scale}x: lighter than the fill by {paper:.3}, fill \
                 {beyond:.2} px past the line"
            );
            // Measured 2026-10-04: at most 0.018 (the 1.5-pixel line at 1x), else below 0.003;
            // the fill never past the line. Hard edges (S29) left 0.69 at 4x along that line.
            if paper >= 0.03 || beyond >= 1.5 {
                failed.push((radius, width, scale));
            }
        }
    }
    assert!(failed.is_empty(), "paper or fill shows: {failed:?}");
}

#[test]
fn banded_fill_edges_equal_one_piece() {
    let core = gpu::core();
    let artwork = filled(&core, 40.0, 3.0);
    for scale in [2, 4] {
        let banding = Banding {
            rows: 37 * scale,
            columns: 53 * scale + 3,
        };
        let one = Banding {
            rows: SIZE[1] * scale,
            columns: SIZE[0] * scale,
        };
        assert_eq!(
            export(&core, &artwork, scale, banding),
            export(&core, &artwork, scale, one),
            "{scale}x"
        );
    }
}
