//! The raster core probe (S57, measurement only; DECISIONS D84): one layer's strokes drawn by the
//! GPU's rasterizer rather than evaluated per pixel over binned ops, timed against the evaluator
//! on the same output region and compared with its image. Each stroke's coverage (or density, and
//! a paper stroke's contact) lies in a region of a scratch atlas, combined by the blend unit (max
//! or add); then each region is composited onto a float layer by the evaluator's paint, and the
//! layer onto the paper (`raster_plan.rs` the CPU's part, `raster_gpu.rs` the pipelines,
//! `raster.wgsl` the stages). Reached only through the benchmark (`cargo device-input bench`,
//! harness `bench`) and its test; nothing the app shows goes through it.

use std::time::{Duration, Instant};

use openpaint_core::Artwork;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::WARM_ENOUGH;
use crate::gpu_clock::GpuClock;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::pace::Group;
use crate::program::scene_ops;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{RasterCounts, RasterPlan};
use crate::readback::read_rgba;
use crate::scene::Scene;
use crate::scene_reuse::Earlier;

#[path = "raster_frame.rs"]
mod frame;
use frame::Frame;

/// Evaluations before the timed ones: at least this many and this long (as the evaluator's
/// benchmark warms, `evaluator_bench.rs`).
const WARMUP: usize = 3;
const WARM_TIME: Duration = Duration::from_millis(250);
/// Timed evaluations at most (the GPU clock's slots).
const MOST: usize = 16;

/// What drawing an output region through the probe took, and what it drew.
#[derive(Debug, Clone)]
pub struct RasterTimed {
    /// GPU time of each evaluation, from the first pass's start to the last's end, ns, fastest
    /// first.
    pub times: Vec<f64>,
    /// Measured by GPU timestamps; else by the wall time of each submission waited for.
    pub stamped: bool,
    pub counts: RasterCounts,
    /// Render passes an evaluation takes: two a batch, one more for the paper.
    pub passes: usize,
    pub atlas: [u32; 2],
    /// The atlas's and the layer's texel formats, and their bytes.
    pub formats: [String; 2],
    pub bytes: [u64; 2],
    /// CPU time: the plan (instances, tiles, batches), its instances' upload (the scene's ops are
    /// the evaluator's upload too), and encoding one evaluation's passes (median), ns.
    pub plan_ns: f64,
    pub upload_ns: f64,
    pub encode_ns: f64,
    /// CPU time the probe's 9 pipelines took to make (shader module and compilation), once
    /// per device, ns.
    pub compile_ns: f64,
    /// The region drawn, 8 bits per channel, and whether it was bit for bit what the warm-up's
    /// evaluations drew (the blend unit combines in the order the draws are given).
    pub image: RgbaImage,
    pub repeatable: bool,
}

impl RasterTimed {
    /// The middle time, ns.
    #[must_use]
    pub fn median(&self) -> f64 {
        self.times.get(self.times.len() / 2).copied().unwrap_or(0.0)
    }
}

impl Core {
    /// Draws `artwork` (one plain layer of finished strokes, else `Invalid`) on white paper over
    /// a target of `size` output pixels whose top-left lies at canvas pixel `at`, at `scale`
    /// output pixels per canvas pixel, through the raster probe with an atlas `atlas` pixels
    /// square: planned once, then evaluated after a warm-up `repeats` times (at most 16), each
    /// timed on the GPU.
    ///
    /// # Errors
    /// `Invalid` for other content, a scale other than 1, 2, 4 or 8, or a stroke's region
    /// larger than the atlas; as the GPU otherwise.
    pub fn time_raster(
        &self,
        artwork: &Artwork,
        (scale, at): (u32, [u32; 2]),
        size: [u32; 2],
        (repeats, atlas): (usize, u32),
    ) -> Result<RasterTimed, RenderError> {
        let pipelines = self.raster_pipelines()?;
        let output = Output::at(scale, at.map(|v| v * scale))?;
        let scene = Scene::build_knowing(artwork, None, (Vec::new(), Earlier::default()))?;
        let started = Instant::now();
        let plan = RasterPlan::build(&scene, output, size, [atlas; 2])?;
        let plan_ns = nanoseconds(started.elapsed());
        let ops = scene_ops(&self.gpu, &scene, None);
        let frame = Frame::new(self, pipelines, (&plan, &ops), (output, size, [atlas; 2]))?;
        let mut encoded = Vec::new();
        let warming = Instant::now();
        let mut warmed = 0;
        while (warmed < WARMUP || warming.elapsed() < WARM_TIME) && warming.elapsed() < WARM_ENOUGH
        {
            let started = Instant::now();
            let buffers = frame.encode(&self.gpu, pipelines, &plan, None);
            encoded.push(nanoseconds(started.elapsed()));
            self.gpu.submit(buffers);
            self.wait_idle()?;
            warmed += 1;
        }
        let warm = read_rgba(&self.gpu, &frame.out)?;
        let (times, stamped) =
            self.raster_runs(&frame, pipelines, &plan, repeats.clamp(1, MOST))?;
        encoded.sort_by(f64::total_cmp);
        let image = read_rgba(&self.gpu, &frame.out)?;
        Ok(RasterTimed {
            repeatable: warm == image,
            times,
            stamped,
            counts: plan.counts,
            passes: frame.passes(&plan),
            atlas: [atlas; 2],
            formats: [pipelines.atlas, pipelines.layer].map(|format| format!("{format:?}")),
            bytes: frame.bytes(),
            plan_ns,
            upload_ns: frame.upload_ns,
            encode_ns: encoded[encoded.len() / 2],
            compile_ns: pipelines.compile_ns,
            image,
        })
    }

    /// The probe's pipelines, made on first use (how long that took is logged).
    fn raster_pipelines(&self) -> Result<&RasterPipelines, RenderError> {
        if let Some(pipelines) = self.raster.get() {
            return Ok(pipelines);
        }
        let started = Instant::now();
        let mut made = RasterPipelines::new(&self.gpu)?;
        made.compile_ns = nanoseconds(started.elapsed());
        Ok(self.raster.get_or_init(|| made))
    }

    /// `repeats` evaluations of the frame, each timed by GPU timestamps (or by waiting for it):
    /// their times, ns, fastest first, and whether timestamps measured them.
    fn raster_runs(
        &self,
        frame: &Frame,
        pipelines: &RasterPipelines,
        plan: &RasterPlan,
        repeats: usize,
    ) -> Result<(Vec<f64>, bool), RenderError> {
        let mut times = Vec::new();
        let stamped = match GpuClock::new(&self.gpu) {
            Some(mut clock) => {
                for _ in 0..repeats {
                    let timing = (clock.start(Group::default()))
                        .ok_or(RenderError::Invalid("no free timestamp slot"))?;
                    let buffers = frame.encode(&self.gpu, pipelines, plan, Some((&clock, timing)));
                    self.gpu.submit(buffers);
                    clock.finish(&self.gpu, timing);
                }
                // Each reading comes back once the GPU is done and its buffer mapped.
                for _ in 0..100 {
                    self.wait_idle()?;
                    times.extend(clock.readings(&self.gpu).into_iter().map(|(_, ns)| ns));
                    if times.len() >= repeats {
                        break;
                    }
                }
                true
            }
            None => {
                for _ in 0..repeats {
                    let started = Instant::now();
                    self.gpu
                        .submit(frame.encode(&self.gpu, pipelines, plan, None));
                    self.wait_idle()?;
                    times.push(nanoseconds(started.elapsed()));
                }
                false
            }
        };
        times.sort_by(f64::total_cmp);
        Ok((times, stamped))
    }
}

fn nanoseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e9
}

#[path = "raster_difference.rs"]
mod difference;
pub use difference::{EDGE, ImageDifference, image_difference};

#[path = "raster_passes.rs"]
mod passes;
pub use passes::PassCost;
