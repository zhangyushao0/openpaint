//! The brush editor's preview strip (S46): a small white canvas the strip's size in pixels at the
//! screen's density, holding a sample stroke (an S-curve across it, its pressure rising to 1 at
//! the middle and falling again, upright) and the strokes the pen drew on the strip, each drawn
//! with the style the canvas's next stroke takes, painting. It is an artwork like any other,
//! evaluated by the one evaluator, so the strip shows what the canvas would.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use openpaint_core::{Artwork, CanvasPoint, CanvasSize, CoreError, StrokeSample, StrokeStyle};
use openpaint_render::{CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage};

use crate::canvas::PAPER;
use crate::error::RuntimeError;

/// Samples along the sample stroke.
const SAMPLES: u16 = 64;
/// Milliseconds between them: an unhurried stroke.
const SAMPLE_MS: u32 = 6;
/// Where the sample stroke starts and ends, as a share of the strip's width from either end.
const MARGIN: f32 = 0.12;
/// How far the S-curve swings up and down, as a share of the strip's height.
const SWING: f32 = 0.24;

/// What the strip shows: its size in pixels, the style its strokes take and the pen's strokes,
/// oldest first, in strip pixels with times from each stroke's start.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StripJob {
    pub(crate) size: [u32; 2],
    pub(crate) style: StrokeStyle,
    pub(crate) strokes: Vec<Arc<[StrokeSample]>>,
}

/// The sample stroke across a strip of `size` pixels.
pub(crate) fn sample_stroke([width, height]: [u32; 2]) -> Result<Vec<StrokeSample>, CoreError> {
    let (width, height) = (width as f32, height as f32);
    let margin = MARGIN * width;
    (0..SAMPLES)
        .map(|i| {
            let t = f32::from(i) / f32::from(SAMPLES - 1);
            let x = margin + (width - 2.0 * margin) * t;
            let y = 0.5 * height - SWING * height * (TAU * t).sin();
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

/// The strip's artwork: the sample stroke, then the pen's, each with the job's style; the n-th
/// stroke's stamps take seed n, so the strip looks the same on every evaluation.
pub(crate) fn strip_artwork(job: &StripJob) -> Result<Artwork, CoreError> {
    let [width, height] = job.size;
    let mut artwork = Artwork::new(CanvasSize::new(width, height)?);
    let sample: Arc<[StrokeSample]> = sample_stroke(job.size)?.into();
    for (seed, samples) in (0u32..).zip(std::iter::once(&sample).chain(&job.strokes)) {
        let Some((first, rest)) = samples.split_first() else {
            continue;
        };
        let style = job.style.clone().with_seed(seed);
        let mut stroke = artwork.begin_stroke(style, *first)?;
        stroke.extend(rest)?;
        artwork = stroke.finish().artwork;
    }
    Ok(artwork)
}

/// The strip evaluated and read back.
pub(crate) fn evaluate_strip(core: &Core, job: &StripJob) -> Result<RgbaImage, RuntimeError> {
    let artwork = strip_artwork(job)?;
    let mut target = CanvasTarget::new(core, job.size)?;
    let request = FrameRequest {
        artwork: &artwork,
        active: None,
        background: PAPER,
    };
    RegionEvaluator::new(core, &mut target).evaluate(&request, &[PixelRect::full(job.size)])?;
    Ok(target.read(core)?)
}

#[cfg(test)]
mod tests {
    use openpaint_app::{BrushIntent, BrushKey, BrushLibrary, Toolbox};
    use openpaint_core::StrokeTool;

    use super::*;

    /// The size of the strip at the tablet's density: 304 x 160 points at 1.75.
    const SIZE: [u32; 2] = [532, 280];

    fn job(key: &str) -> StripJob {
        let mut toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
        toolbox.dispatch(&BrushIntent::Choose(BrushKey::new(key)));
        let style = toolbox.style().expect("style").with_tool(StrokeTool::Paint);
        let pen: Vec<StrokeSample> = (0..30u16)
            .map(|i| {
                let at = CanvasPoint::new(60.0 + 12.0 * f32::from(i), 200.0).expect("point");
                let pressure = 0.3 + 0.02 * f32::from(i);
                StrokeSample::new(at, pressure, 0.4, 1.0, u32::from(i) * 5).expect("sample")
            })
            .collect();
        StripJob {
            size: SIZE,
            style,
            strokes: vec![pen.into()],
        }
    }

    /// The strip's strokes drawn on a canvas of its size as the canvas draws them: each stroke
    /// a few samples a frame in progress over what was committed before, then committed.
    fn on_a_canvas(core: &Core, job: &StripJob) -> RgbaImage {
        let finished = strip_artwork(job).expect("artwork");
        let mut artwork = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
        let mut target = CanvasTarget::new(core, SIZE).expect("target");
        let mut evaluator = RegionEvaluator::new(core, &mut target);
        evaluator
            .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
            .expect("evaluate");
        for stroke in finished.layers()[0].strokes().iter() {
            let samples = stroke.samples();
            let mut session = artwork
                .begin_stroke(stroke.style().clone(), samples[0])
                .expect("begin");
            for chunk in samples[1..].chunks(3) {
                let frame = FrameRequest {
                    active: Some(session.active()),
                    ..request(&artwork)
                };
                evaluator.evaluate(&frame, &[]).expect("evaluate");
                session.extend(chunk).expect("extend");
            }
            artwork = session.finish().artwork;
            evaluator
                .evaluate(&request(&artwork), &[])
                .expect("evaluate");
        }
        target.read(core).expect("readback")
    }

    fn request(artwork: &Artwork) -> FrameRequest<'_> {
        FrameRequest {
            artwork,
            active: None,
            background: PAPER,
        }
    }

    #[test]
    fn the_strip_shows_exactly_what_a_canvas_of_its_size_shows() {
        let core = Core::headless().expect("a GPU adapter is required for runtime tests");
        for key in ["inker", "pencil", "soft-brush", "textured-round", "speckle"] {
            let job = job(key);
            let strip = evaluate_strip(&core, &job).expect("strip");
            assert_eq!(strip.size(), SIZE);
            let canvas = on_a_canvas(&core, &job);
            assert!(strip == canvas, "{key}: the strip differs from the canvas");
            let painted = strip.pixels().chunks(4).filter(|p| p[0] < 250).count();
            assert!(
                painted > 500,
                "{key}: the strip shows the strokes ({painted})"
            );
        }
    }
}
