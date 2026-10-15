//! `test-art`'s larger kinds (S58, `test_art.rs`): an A4 page like the author's busiest artwork
//! and an hour's session of strokes at the author's pace.

use std::f32::consts::TAU;
use std::sync::Arc;

use openpaint_app::BrushLibrary;
use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, Color, LayerCommand, MaskCommand, Patch,
    Placement, RegionMask, StrokeStyle, StrokeTool,
};

use crate::error::HarnessError;
use crate::test_art::{A4, BRUSHES, Hand, SCREEN, stroke, unit};

/// The A4 page's dense place and its radius: most strokes cross it (the author's busiest artwork
/// is dense in one place).
const DENSE: [f32; 2] = [2450.0, 820.0];
const DENSE_RADIUS: f32 = 420.0;
/// The share of a layer's strokes that start in the dense place.
const DENSE_SHARE: f32 = 0.45;
/// Smudges over the dense place (the author's busiest artwork holds 9).
const SMUDGES: u32 = 9;

/// One layer's strokes: its brushes in turn, how many, and the salt its numbers take.
struct Strokes<'a> {
    brushes: &'a [&'a str],
    count: u32,
    salt: u64,
}

/// An A4 page of `n` strokes over five layers: a page fill and the Pencil's sketch; two fills,
/// the painting brushes and 9 smudges over the dense place; a picture inserted; the inking
/// brushes with a mask hiding part of them; the texture brushes on a Multiply layer.
pub fn a4(library: &BrushLibrary, n: u32) -> Result<Artwork, HarnessError> {
    let canvas = CanvasSize::new(A4[0], A4[1])?;
    let mut artwork = Artwork::new(canvas);
    artwork = fill(&artwork, |_, _| true, [0.96, 0.93, 0.86])?;
    let sketch = Strokes {
        brushes: &["pencil"],
        count: n * 20 / 100,
        salt: 1,
    };
    artwork = strokes(&artwork, library, &sketch)?;
    artwork = layer(&artwork)?;
    let disc = |centre: [f32; 2], radius: f32| {
        move |x: f32, y: f32| (x - centre[0]).hypot(y - centre[1]) <= radius
    };
    artwork = fill(&artwork, disc([1200.0, 1300.0], 700.0), [0.55, 0.7, 0.85])?;
    artwork = fill(&artwork, disc(DENSE, 520.0), [0.85, 0.55, 0.4])?;
    let paint = Strokes {
        brushes: &["hard-round", "soft-brush", "flat", "opaque-paint"],
        count: n * 35 / 100,
        salt: 2,
    };
    artwork = strokes(&artwork, library, &paint)?;
    artwork = smudges(&artwork, library)?;
    artwork = picture(&artwork)?;
    artwork = layer(&artwork)?;
    let ink = Strokes {
        brushes: &["ink-pen", "inker", "monoline"],
        count: n * 30 / 100,
        salt: 4,
    };
    artwork = strokes(&artwork, library, &ink)?;
    artwork = masked(&artwork, library)?;
    artwork = layer(&artwork)?;
    let blend = LayerCommand::SetBlend {
        layer: artwork.active(),
        blend: BlendMode::Multiply,
    };
    artwork = artwork.apply(ArtworkCommand::Layer(blend))?.artwork;
    let left = n - sketch.count - paint.count - ink.count;
    let texture = Strokes {
        brushes: &["textured-round", "speckle"],
        count: left,
        salt: 5,
    };
    strokes(&artwork, library, &texture)
}

/// An hour's session: `minutes` x 50 strokes on a screen-size canvas, a stroke every 1.2 s of 342
/// samples 2.5 px and 3.3 ms apart (the author's, STATUS), the ten built-ins in turn ten strokes
/// each, every tenth stroke a smudge, a new layer every 500 strokes (`long-session`'s pace).
pub fn session(library: &BrushLibrary, minutes: u32) -> Result<Artwork, HarnessError> {
    let mut artwork = Artwork::new(CanvasSize::new(SCREEN[0], SCREEN[1])?);
    let [width, height] = SCREEN.map(|side| side as f32);
    let bounds = [150.0, 150.0, width - 150.0, height - 150.0];
    for k in 0..minutes * 50 {
        if k > 0 && k % 500 == 0 {
            artwork = layer(&artwork)?;
        }
        let key = u64::from(k);
        let smudge = k % 10 == 9;
        let brush = if smudge {
            "soft-brush"
        } else {
            BRUSHES[(k as usize / 10) % BRUSHES.len()]
        };
        let hand = Hand {
            start: [
                bounds[0] + (bounds[2] - bounds[0]) * unit(key, 70),
                bounds[1] + (bounds[3] - bounds[1]) * unit(key, 71),
            ],
            angle: TAU * unit(key, 72),
            turn: (unit(key, 73) - 0.5) * 0.02,
            count: 342,
            spacing: 2.5,
            bounds,
        };
        let mut style = style(library, brush, 1.0, colour(key, 74)?, k)?;
        if smudge {
            style = style.with_tool(StrokeTool::Smudge);
        }
        artwork = stroke(&artwork, style, &hand.samples()?)?;
    }
    Ok(artwork)
}

/// A stroke of built-in `key` at `factor` times its first size (within its range) in `color`,
/// jittered by `seed`.
fn style(
    library: &BrushLibrary,
    key: &str,
    factor: f32,
    color: Color,
    seed: u32,
) -> Result<StrokeStyle, HarnessError> {
    let index = library
        .find(key)
        .ok_or(HarnessError::Missing("a test-art brush is not built in"))?;
    let brush = &library.brushes()[index];
    let range = brush.size();
    let size = (range.default * factor).clamp(range.min, range.max);
    let style = StrokeStyle::new(Arc::clone(brush.recipe()), size, brush.opacity(), color)?;
    Ok(style.with_seed(seed))
}

/// A colour drawn from `k` and `salt`, never too light to see.
fn colour(k: u64, salt: u64) -> Result<Color, HarnessError> {
    let channel = |at: u64| 0.1 + 0.75 * unit(k, salt * 8 + at);
    Ok(Color::rgb(channel(0), channel(1), channel(2))?)
}

/// `layer`'s strokes on the artwork's target: brushes in turn, sizes 0.75 to 2.25 times their
/// first, starting in the dense place by `DENSE_SHARE`, else anywhere on the page.
fn strokes(
    artwork: &Artwork,
    library: &BrushLibrary,
    layer: &Strokes<'_>,
) -> Result<Artwork, HarnessError> {
    let canvas = artwork.canvas();
    let page = [
        80.0,
        80.0,
        canvas.width() as f32 - 80.0,
        canvas.height() as f32 - 80.0,
    ];
    let mut artwork = artwork.clone();
    for k in 0..layer.count {
        let key = u64::from(k) * 16 + layer.salt * 1_000_003;
        let start = if unit(key, 1) < DENSE_SHARE {
            let (angle, reach) = (TAU * unit(key, 2), DENSE_RADIUS * unit(key, 3).sqrt());
            [
                DENSE[0] + reach * angle.cos(),
                DENSE[1] + reach * angle.sin(),
            ]
        } else {
            [
                page[0] + (page[2] - page[0]) * unit(key, 2),
                page[1] + (page[3] - page[1]) * unit(key, 3),
            ]
        };
        let hand = Hand {
            start,
            angle: TAU * unit(key, 4),
            turn: (unit(key, 5) - 0.5) * 0.03,
            count: 80 + (260.0 * unit(key, 6)) as u32,
            spacing: 2.5 + 3.5 * unit(key, 7),
            bounds: page,
        };
        let brush = layer.brushes[k as usize % layer.brushes.len()];
        let factor = 0.75 + 1.5 * unit(key, 8);
        let style = style(library, brush, factor, colour(key, 9)?, k)?;
        artwork = stroke(&artwork, style, &hand.samples()?)?;
    }
    Ok(artwork)
}

/// `SMUDGES` Soft Brush smudges of 80 to 160 px across the dense place.
fn smudges(artwork: &Artwork, library: &BrushLibrary) -> Result<Artwork, HarnessError> {
    let mut artwork = artwork.clone();
    let page = [0.0, 0.0, A4[0] as f32, A4[1] as f32];
    for k in 0..SMUDGES {
        let key = u64::from(k) + 900;
        let (angle, reach) = (TAU * unit(key, 1), DENSE_RADIUS * unit(key, 2).sqrt());
        let hand = Hand {
            start: [
                DENSE[0] + reach * angle.cos(),
                DENSE[1] + reach * angle.sin(),
            ],
            angle: TAU * unit(key, 3),
            turn: (unit(key, 4) - 0.5) * 0.02,
            count: 120,
            spacing: 3.0,
            bounds: page,
        };
        let factor = (80.0 + 80.0 * unit(key, 5)) / 100.0;
        let style = style(library, "soft-brush", factor, colour(key, 6)?, k)?;
        let smudge = style.with_tool(StrokeTool::Smudge);
        artwork = stroke(&artwork, smudge, &hand.samples()?)?;
    }
    Ok(artwork)
}

/// The ink layer's mask, three large black Soft Brush strokes on it hiding part of the ink over
/// the dense place; the layer's paint the target again after.
fn masked(artwork: &Artwork, library: &BrushLibrary) -> Result<Artwork, HarnessError> {
    let id = artwork.active();
    let mut artwork = artwork
        .apply(ArtworkCommand::Mask(MaskCommand::Add(id)))?
        .artwork;
    let page = [0.0, 0.0, A4[0] as f32, A4[1] as f32];
    for k in 0..3u32 {
        let hand = Hand {
            start: [DENSE[0] - 300.0 + 300.0 * k as f32, DENSE[1] - 200.0],
            angle: TAU * 0.25,
            turn: 0.0,
            count: 140,
            spacing: 3.0,
            bounds: page,
        };
        let black = Color::rgb(0.0, 0.0, 0.0)?;
        let style = style(library, "soft-brush", 2.0, black, k)?;
        artwork = stroke(&artwork, style, &hand.samples()?)?;
    }
    let paint = ArtworkCommand::Layer(LayerCommand::Select(id));
    Ok(artwork.apply(paint)?.artwork)
}

/// A new layer above the active one, active.
fn layer(artwork: &Artwork) -> Result<Artwork, HarnessError> {
    let create = ArtworkCommand::Layer(LayerCommand::Create);
    Ok(artwork.apply(create)?.artwork)
}

/// The pixels `inside` holds filled with `rgb` on the target.
fn fill(
    artwork: &Artwork,
    inside: impl Fn(f32, f32) -> bool,
    [r, g, b]: [f32; 3],
) -> Result<Artwork, HarnessError> {
    let canvas = artwork.canvas();
    let width = canvas.width();
    let pixels: Vec<bool> = (0..width * canvas.height())
        .map(|at| inside((at % width) as f32 + 0.5, (at / width) as f32 + 0.5))
        .collect();
    let mask = RegionMask::from_pixels(canvas, &pixels)?;
    Ok(artwork.fill_region(mask, Color::rgb(r, g, b)?)?.artwork)
}

/// A 640 x 480 gradient inserted on a layer of its own near the page's lower left (an inserted
/// picture is a transform record holding it, S26).
fn picture(artwork: &Artwork) -> Result<Artwork, HarnessError> {
    let side = [640u32, 480];
    let pixels: Vec<u8> = (0..side[0] * side[1])
        .flat_map(|at| {
            let [x, y] = [at % side[0], at / side[0]];
            [
                (x * 255 / side[0]) as u8,
                (y * 255 / side[1]) as u8,
                160,
                255,
            ]
        })
        .collect();
    let patch = Patch::new(artwork.canvas(), [300, 1700], side, pixels)?;
    let layered = layer(artwork)?;
    Ok(layered
        .transform_region(None, patch, Placement::IDENTITY)?
        .artwork)
}
