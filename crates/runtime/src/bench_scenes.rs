//! The benchmark's scenes (S55, `bench.rs`): a 1024 x 1024 canvas whose middle holds mostly one
//! kind of op. Brush scenes are strokes of a built-in brush at its first size (one at its
//! largest), each the path `cargo device-input pen-many` lays (a 500 px wave, 52 samples 8 ms
//! apart, pressed 0.8) turned by a golden angle more about the canvas's middle; the plain scenes
//! one stroke of 1, 8 or 64 straight spans whose capsules cover the whole square; the rest fills,
//! inserted pictures, smudges, blurs and adjustments over one fill.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use openpaint_app::BrushLibrary;
use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, Blur, BrushRecipe, CanvasPoint, CanvasSize, Color, Hsb,
    LayerCommand, Patch, Placement, RegionMask, StrokeSample, StrokeStyle, StrokeTool,
};
use openpaint_render::{RenderError, Timed};

use crate::error::RuntimeError;

pub(crate) const CANVAS: u32 = 1024;
const MIDDLE: [f32; 2] = [512.0, 512.0];
/// Strokes in a brush scene.
const STROKES: u32 = 16;
const GOLDEN: f32 = 2.399_963_2;

/// One scene: how it is made, the scene whose time is its base, and how many ops of its kind
/// a plan lists.
pub(crate) struct Scene {
    pub(crate) name: &'static str,
    pub(crate) base: Option<&'static str>,
    pub(crate) kind: fn(&Timed) -> f64,
    make: fn(&BrushLibrary) -> Result<Artwork, RuntimeError>,
}

impl Scene {
    pub(crate) fn artwork(&self, library: &BrushLibrary) -> Result<Artwork, RuntimeError> {
        (self.make)(library)
    }
}

/// The empty scene's "ops" are its tiles: its time a pixel is the loop's fixed cost.
fn tiles(timed: &Timed) -> f64 {
    timed.tiles as f64
}

fn spans(timed: &Timed) -> f64 {
    (timed.listed.spans + timed.listed.stamped) as f64
}

fn fills(timed: &Timed) -> f64 {
    timed.listed.fills as f64
}

fn patches(timed: &Timed) -> f64 {
    timed.listed.patches as f64
}

fn smudges(timed: &Timed) -> f64 {
    timed.listed.smudges as f64
}

fn filters(timed: &Timed) -> f64 {
    timed.listed.filters as f64
}

fn others(timed: &Timed) -> f64 {
    timed.listed.others as f64
}

const fn scene(
    name: &'static str,
    base: Option<&'static str>,
    kind: fn(&Timed) -> f64,
    make: fn(&BrushLibrary) -> Result<Artwork, RuntimeError>,
) -> Scene {
    Scene {
        name,
        base,
        kind,
        make,
    }
}

const EMPTY: Option<&str> = Some("empty");
const FILLED: Option<&str> = Some("fill-1");

pub(crate) const SCENES: [Scene; 19] = [
    scene("empty", None, tiles, |_| blank()),
    scene("plain-1", EMPTY, spans, |_| plain(1)),
    scene("plain-8", EMPTY, spans, |_| plain(8)),
    scene("plain-64", EMPTY, spans, |_| plain(64)),
    scene("ink-pen", EMPTY, spans, |l| {
        brushed(l, "ink-pen", None, STROKES)
    }),
    scene("soft-brush", EMPTY, spans, |l| {
        brushed(l, "soft-brush", None, STROKES)
    }),
    scene("pencil", EMPTY, spans, |l| {
        brushed(l, "pencil", None, STROKES)
    }),
    scene("textured-round", EMPTY, spans, |l| {
        brushed(l, "textured-round", None, STROKES)
    }),
    scene("opaque-paint", EMPTY, spans, |l| {
        brushed(l, "opaque-paint", None, STROKES)
    }),
    scene("opaque-paint-400", EMPTY, spans, |l| {
        brushed(l, "opaque-paint", Some(400.0), 2)
    }),
    scene("speckle", EMPTY, spans, |l| {
        brushed(l, "speckle", None, STROKES)
    }),
    scene("flat", EMPTY, spans, |l| brushed(l, "flat", None, STROKES)),
    scene("fill", EMPTY, fills, |_| fills_over(&blank()?, 8)),
    scene("fill-1", EMPTY, fills, |_| fills_over(&blank()?, 1)),
    scene("patch", EMPTY, patches, |_| pictures(4)),
    scene("smudge", FILLED, smudges, |l| smudged(l, 8)),
    scene("blur", FILLED, filters, |_| adjusted(4, blurring)),
    scene("adjust", FILLED, others, |_| adjusted(8, toning)),
    scene("mixed", EMPTY, spans, mixed),
];

fn blank() -> Result<Artwork, RuntimeError> {
    Ok(Artwork::new(CanvasSize::new(CANVAS, CANVAS)?))
}

fn missing(what: &'static str) -> RuntimeError {
    RuntimeError::Render(RenderError::Invalid(what))
}

fn sample(point: [f32; 2], pressure: f32, i: u32) -> Result<StrokeSample, RuntimeError> {
    let position = CanvasPoint::new(point[0], point[1])?;
    Ok(StrokeSample::new(position, pressure, 0.0, 0.0, 8 * i)?)
}

/// `artwork` with a stroke of `style` along `points`, pressed `pressure(i)`, committed.
pub(crate) fn drawn(
    artwork: &Artwork,
    style: StrokeStyle,
    points: &[[f32; 2]],
    pressure: impl Fn(u32) -> f32,
) -> Result<Artwork, RuntimeError> {
    let samples = (0..)
        .zip(points)
        .map(|(i, point)| sample(*point, pressure(i), i))
        .collect::<Result<Vec<_>, _>>()?;
    let mut session = artwork.begin_stroke(style, samples[0])?;
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}

/// The wave `pen-many` lays, through `middle`, turned by `k` golden angles about it.
pub(crate) fn wave(k: u32, middle: [f32; 2]) -> Vec<[f32; 2]> {
    let (sin, cos) = (k as f32 * GOLDEN).sin_cos();
    (0..52u16)
        .map(|i| {
            let t = f32::from(i) / 51.0;
            let [x, y] = [500.0f32.mul_add(t, -250.0), 43.0 * (PI * t).sin()];
            [middle[0] + x * cos - y * sin, middle[1] + x * sin + y * cos]
        })
        .collect()
}

fn color() -> Result<Color, RuntimeError> {
    Ok(Color::rgb(0.2, 0.3, 0.6)?)
}

/// A stroke of built-in brush `key` at `size` (its first size without one), seed 0, painting.
pub(crate) fn style_of(
    library: &BrushLibrary,
    key: &str,
    size: Option<f32>,
) -> Result<StrokeStyle, RuntimeError> {
    let index = library
        .find(key)
        .ok_or(missing("the benchmark's brush is not built in"))?;
    let brush = &library.brushes()[index];
    let size = size.unwrap_or(brush.size().default);
    Ok(StrokeStyle::new(
        Arc::clone(brush.recipe()),
        size,
        brush.opacity(),
        color()?,
    )?)
}

/// `count` strokes of built-in brush `key` at `size` (its first size without one) over `artwork`.
fn brushed_over(
    artwork: &Artwork,
    library: &BrushLibrary,
    (key, size, tool): (&str, Option<f32>, StrokeTool),
    count: u32,
) -> Result<Artwork, RuntimeError> {
    let style = style_of(library, key, size)?;
    let mut artwork = artwork.clone();
    for k in 0..count {
        let style = style.clone().with_seed(k).with_tool(tool);
        artwork = drawn(&artwork, style, &wave(k, MIDDLE), |_| 0.8)?;
    }
    Ok(artwork)
}

fn brushed(
    library: &BrushLibrary,
    key: &str,
    size: Option<f32>,
    count: u32,
) -> Result<Artwork, RuntimeError> {
    brushed_over(&blank()?, library, (key, size, StrokeTool::Paint), count)
}

/// One plain round stroke of `spans` straight spans along the canvas's middle, 1000 px across,
/// so every capsule covers the whole square; the pressure alternates so every sample is a knot.
fn plain(spans: u16) -> Result<Artwork, RuntimeError> {
    let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 1000.0, 0.5, color()?)?;
    let points: Vec<[f32; 2]> = (0..=spans)
        .map(|i| {
            let t = f32::from(i) / f32::from(spans);
            [400.0f32.mul_add(t, MIDDLE[0] - 200.0), MIDDLE[1]]
        })
        .collect();
    drawn(&blank()?, style, &points, |i| 1.0 - 0.01 * (i % 2) as f32)
}

/// `count` discs filled over `artwork`: one covering the middle, or several whose edges cross it.
fn fills_over(artwork: &Artwork, count: u32) -> Result<Artwork, RuntimeError> {
    let canvas = artwork.canvas();
    let mut artwork = artwork.clone();
    for k in 0..count {
        let (radius, offset) = if count == 1 {
            (420.0, 0.0)
        } else {
            (140.0, 110.0)
        };
        let (sin, cos) = (k as f32 * GOLDEN).sin_cos();
        let centre = [offset * cos + MIDDLE[0], offset * sin + MIDDLE[1]];
        let inside: Vec<bool> = (0..CANVAS * CANVAS)
            .map(|at| {
                let [x, y] = [at % CANVAS, at / CANVAS].map(|v| v as f32 + 0.5);
                (x - centre[0]).hypot(y - centre[1]) <= radius
            })
            .collect();
        let mask = RegionMask::from_pixels(canvas, &inside)?;
        let shade = Color::rgb(0.9 - 0.08 * k as f32, 0.4, 0.2 + 0.08 * k as f32)?;
        artwork = artwork.fill_region(mask, shade)?.artwork;
    }
    Ok(artwork)
}

/// `count` pictures inserted on layers of their own, each 320 px square over the middle.
fn pictures(count: u32) -> Result<Artwork, RuntimeError> {
    let mut artwork = blank()?;
    let side = 320u32;
    for k in 0..count {
        let pixels: Vec<u8> = (0..side * side)
            .flat_map(|at| {
                let [x, y] = [at % side, at / side];
                [(x % 256) as u8, (y % 256) as u8, (k * 60) as u8, 200]
            })
            .collect();
        let origin = [MIDDLE[0] as u32 - side / 2; 2];
        let patch = Patch::new(artwork.canvas(), origin, [side, side], pixels)?;
        let layered = artwork.apply(ArtworkCommand::Layer(LayerCommand::Create))?;
        let placed = layered
            .artwork
            .transform_region(None, patch, Placement::IDENTITY)?;
        artwork = placed.artwork;
    }
    Ok(artwork)
}

/// The middle filled, then `count` smudges of the Soft Brush at 60 px across it.
fn smudged(library: &BrushLibrary, count: u32) -> Result<Artwork, RuntimeError> {
    let filled = fills_over(&blank()?, 1)?;
    let tool = ("soft-brush", Some(60.0), StrokeTool::Smudge);
    brushed_over(&filled, library, tool, count)
}

/// The middle filled, then `count` adjustments made by `of(k)`.
fn adjusted(count: u32, of: fn(u32) -> Adjustment) -> Result<Artwork, RuntimeError> {
    let mut artwork = fills_over(&blank()?, 1)?;
    for k in 0..count {
        artwork = artwork.adjust_region(None, of(k))?.artwork;
    }
    Ok(artwork)
}

fn blurring(k: u32) -> Adjustment {
    Adjustment::Blur(Blur::new(2.0 + k as f32))
}

fn toning(k: u32) -> Adjustment {
    let turn = (k as f32 * GOLDEN / TAU).fract();
    Adjustment::Hsb(Hsb::new(turn, 0.55, 0.52))
}

/// What a painted place holds: fills, then strokes of every texture brush and the Ink Pen.
fn mixed(library: &BrushLibrary) -> Result<Artwork, RuntimeError> {
    let mut artwork = fills_over(&blank()?, 2)?;
    for key in [
        "ink-pen",
        "textured-round",
        "opaque-paint",
        "speckle",
        "flat",
    ] {
        artwork = brushed_over(&artwork, library, (key, None, StrokeTool::Paint), 4)?;
    }
    Ok(artwork)
}
