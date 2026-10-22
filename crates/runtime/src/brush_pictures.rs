//! The brush panel's pictures on a worker of their own: the editor's preview strip (`strip.rs`)
//! whenever what it shows changed, and each row's picture of its brush once the brush is new or
//! changed, while the list shows (the built-ins' are made at start). One at a time, the strip
//! first, so the frame never waits; derived, never stored. A row's picture is one S-shaped stroke
//! of its brush drawn by the product renderer, so it shows exactly what the brush paints: black on
//! white, which the panel turns into its ink and alpha.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use openpaint_app::{Brush, BrushKey, BrushLibrary, Toolbox};
use openpaint_core::{
    Artwork, CanvasPoint, CanvasSize, Color, CoreError, StrokeSample, StrokeStyle, StrokeTool,
};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RgbaImage};
use openpaint_ui::{Shell, design};

use crate::error::RuntimeError;
use crate::shared::SharedCore;
use crate::strip::{StripJob, evaluate_strip};
use crate::worker::Queue;

/// Samples along an S-curve stroke, and milliseconds between them: an unhurried stroke.
const SAMPLES: u16 = 64;
const SAMPLE_MS: u32 = 6;
/// A row picture's largest diameter as a share of its height, so wide brushes fit.
const HEIGHT_SHARE: f32 = 0.55;
/// Its smallest diameter as a share of its height, so thin brushes show their grain.
const THINNEST_SHARE: f32 = 0.15;

enum Job {
    Row(Brush, [u32; 2]),
    Strip(StripJob),
}

enum Done {
    Row(Brush, Option<RgbaImage>),
    Strip(Option<RgbaImage>),
}

pub(crate) struct BrushPictures {
    worker: Queue<Job, Done>,
    /// The brush each row's picture was made of.
    rows: HashMap<BrushKey, Brush>,
    /// The strip asked for last, and the picture the worker made of it last.
    strip: Option<StripJob>,
    shown: Option<RgbaImage>,
    /// Pixels per point the row pictures are made at.
    pixels_per_point: f32,
}

impl BrushPictures {
    /// Makes the pictures of `library`'s brushes at once, at `pixels_per_point`, for `shell`.
    pub(crate) fn start(
        core: SharedCore,
        library: &BrushLibrary,
        (shell, pixels_per_point): (&mut Shell, f32),
    ) -> Result<Self, RuntimeError> {
        let size = row_size(pixels_per_point);
        let previews = (library.brushes().iter())
            .map(|brush| Ok((brush.key().clone(), row_picture(&core.get(), brush, size)?)))
            .collect::<Result<_, RuntimeError>>()?;
        shell.set_brush_previews(previews);
        let rows = library
            .brushes()
            .iter()
            .map(|brush| (brush.key().clone(), brush.clone()))
            .collect();
        let worker = Queue::spawn("openpaint-brushes", move |job| make(&core.get(), job))?;
        Ok(Self {
            worker,
            rows,
            strip: None,
            shown: None,
            pixels_per_point,
        })
    }

    /// True while a picture is being made or waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// Every row's picture and the strip are made again (those made while the device was lost
    /// failed); the pictures shown stay until then.
    pub(crate) fn redo(&mut self) {
        self.rows.clear();
        self.strip = None;
    }

    /// The strip's last picture, while the editor shows.
    pub(crate) fn strip(&self) -> Option<&RgbaImage> {
        self.shown.as_ref()
    }

    /// One frame: what the worker made goes to the panel; then, if it is idle, the strip as the
    /// editor shows it now when that changed, else the first row picture out of date while the
    /// list shows. Returns whether a picture arrived, which the next frame shows.
    pub(crate) fn step(&mut self, toolbox: &Toolbox, shell: &mut Shell) -> bool {
        let mut arrived = false;
        while let Some(done) = self.worker.take() {
            arrived = true;
            match done {
                Done::Row(brush, picture) => {
                    let key = brush.key().clone();
                    if let Some(picture) = picture {
                        shell.set_brush_picture(key.clone(), picture);
                    }
                    self.rows.insert(key, brush);
                }
                Done::Strip(Some(picture)) => {
                    shell.set_strip_picture(picture.clone());
                    self.shown = Some(picture);
                }
                Done::Strip(None) => {}
            }
        }
        if self.worker.busy() {
            return arrived;
        }
        let Some(asked) = shell.strip() else {
            self.strip = None;
            self.shown = None;
            self.rows(toolbox.library(), shell);
            return arrived;
        };
        if let Ok(style) = toolbox.style() {
            let job = StripJob {
                size: asked.size,
                style: style.with_tool(StrokeTool::Paint),
                strokes: asked.strokes,
            };
            if self.strip.as_ref() != Some(&job) {
                self.strip = Some(job.clone());
                self.worker.submit(Job::Strip(job));
            }
        }
        arrived
    }

    /// The first brush of `library` whose row picture was made of another brush, or none.
    fn rows(&mut self, library: &BrushLibrary, shell: &Shell) {
        if !shell.brushes_listed() {
            return;
        }
        let stale = library
            .brushes()
            .iter()
            .find(|brush| self.rows.get(brush.key()) != Some(brush));
        if let Some(brush) = stale {
            let size = row_size(self.pixels_per_point);
            self.worker.submit(Job::Row(brush.clone(), size));
        }
    }
}

fn make(core: &Core, job: Job) -> Done {
    match job {
        Job::Row(brush, size) => {
            let picture = row_picture(core, &brush, size)
                .inspect_err(|error| log::error!("no picture of {}: {error}", brush.name()))
                .ok();
            Done::Row(brush, picture)
        }
        Job::Strip(job) => Done::Strip(
            evaluate_strip(core, &job)
                .inspect_err(|error| log::error!("no preview strip: {error}"))
                .ok(),
        ),
    }
}

/// A row picture's size in pixels at `pixels_per_point`: `design::BRUSH_PREVIEW` points.
fn row_size(pixels_per_point: f32) -> [u32; 2] {
    let points = design::BRUSH_PREVIEW;
    [points.x, points.y].map(|side| (side * pixels_per_point).round().max(8.0) as u32)
}

/// `brush`'s row picture of `size` pixels: one S-curve across it, black on white, its pressure
/// rising and falling again.
fn row_picture(core: &Core, brush: &Brush, size: [u32; 2]) -> Result<RgbaImage, RuntimeError> {
    let [width, height] = size.map(|side| side as f32);
    let diameter = brush
        .size()
        .default
        .clamp(THINNEST_SHARE * height, HEIGHT_SHARE * height);
    let style = StrokeStyle::new(
        Arc::clone(brush.recipe()),
        diameter,
        brush.opacity(),
        gray(0)?,
    )?;
    let samples = s_curve([width, height], 0.5 * height, 0.22)?;
    let blank = Artwork::new(CanvasSize::new(size[0], size[1])?);
    let mut stroke = blank.begin_stroke(style, samples[0])?;
    stroke.extend(&samples[1..])?;
    drawn_alone(core, &stroke.finish().artwork, gray(255)?)
}

/// An S-curve across a picture of `width` x `height` pixels from `margin` pixels in on either
/// side, swinging `swing` of the height up and down, its pressure rising to 1 at the middle and
/// falling again, upright.
pub(crate) fn s_curve(
    [width, height]: [f32; 2],
    margin: f32,
    swing: f32,
) -> Result<Vec<StrokeSample>, CoreError> {
    (0..SAMPLES)
        .map(|i| {
            let t = f32::from(i) / f32::from(SAMPLES - 1);
            let x = margin + (width - 2.0 * margin) * t;
            let y = 0.5 * height - swing * height * (TAU * t).sin();
            let pressure = 0.2 + 0.8 * (PI * t).sin();
            StrokeSample::new(
                CanvasPoint::new(x, y)?,
                pressure,
                0.0,
                0.0,
                u32::from(i) * SAMPLE_MS,
            )
        })
        .collect()
}

/// `artwork` drawn whole on `background` into a target of its own and read back, its draws paced
/// beside the screen's (made on the brushes worker while the screen draws on).
pub(crate) fn drawn_alone(
    core: &Core,
    artwork: &Artwork,
    background: Color,
) -> Result<RgbaImage, RuntimeError> {
    let canvas = artwork.canvas();
    let size = [canvas.width(), canvas.height()];
    let mut target = CanvasTarget::new(core, size)?;
    target.pace_draws();
    let request = FrameRequest {
        artwork,
        active: None,
        background,
    };
    target.draw(core, &request, &[PixelRect::full(size)])?;
    Ok(target.read(core)?)
}

/// An sRGB grey from an 8-bit value; blending happens on encoded values, so the raster core gives
/// back exactly that byte.
fn gray(value: u8) -> Result<Color, CoreError> {
    let level = f32::from(value) / 255.0;
    Color::rgb(level, level, level)
}
