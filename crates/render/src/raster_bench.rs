//! The raster core's benchmark (S57's probe, kept as the reference for the rewrite's slices): an
//! artwork drawn over one region of output pixels at a scale by the product's raster core, its
//! plan laid out once, then drawn again and again with each draw timed by GPU timestamps (or the
//! wall time of a waited submission), and compared with the evaluator's image (`raster_bench` in
//! the runtime, `cargo device-input bench raster…`, harness `bench raster…`).

use std::time::{Duration, Instant};

use openpaint_core::Artwork;

use super::{Finish, RasterTargets, SceneBuffers};
use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::WARM_ENOUGH;
use crate::filter::Filters;
use crate::geom::PixelRect;
use crate::gpu_clock::GpuClock;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::pace::Group;
use crate::program::scene_ops;
use crate::raster_plan::{RasterCounts, RasterPlan};
use crate::readback::{FORMAT, read_rgba, target_texture};
use crate::scene::Scene;
use crate::scene_reuse::Earlier;
use crate::smudge::{Smudges, Wanted, around_output};

/// Draws before the timed ones: at least this many and this long (as the evaluator's benchmark
/// warms, `evaluator_bench.rs`).
const WARMUP: usize = 3;
const WARM_TIME: Duration = Duration::from_millis(250);
/// Timed draws at most (the GPU clock's slots).
const MOST: usize = 16;

/// What drawing an output region through the raster core took, and what it drew.
#[derive(Debug, Clone)]
pub struct RasterTimed {
    /// GPU time of each draw, from the first pass's start to the finish's end, ns, fastest first.
    pub times: Vec<f64>,
    /// Measured by GPU timestamps; else by the wall time of each submission waited for.
    pub stamped: bool,
    pub counts: RasterCounts,
    /// Render passes a draw takes.
    pub passes: usize,
    pub atlas: [u32; 2],
    /// The atlas's and the canvas's texel formats, and their bytes.
    pub formats: [String; 2],
    pub bytes: [u64; 2],
    /// CPU time: the plan, a draw's values and instances written (median), its encoding and
    /// submission (median), ns.
    pub plan_ns: f64,
    pub upload_ns: f64,
    pub encode_ns: f64,
    /// The raster core's pipelines, how many and how long they took to make (once per device).
    pub pipelines: usize,
    pub compile_ns: f64,
    /// The region drawn, 8 bits per channel, and whether it was bit for bit what the warm-up's
    /// draws drew (the blend unit combines in the order the draws are given).
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

/// One region of a scene ready to draw again and again.
struct Bench<'a> {
    core: &'a Core,
    buffers: SceneBuffers,
    plan: RasterPlan,
    output: Output,
    targets: RasterTargets,
    out: wgpu::Texture,
}

impl Bench<'_> {
    /// One draw, timed by `clock` when given; its CPU times.
    fn draw(&mut self, mut clock: Option<&mut GpuClock>) -> Result<[f64; 2], RenderError> {
        let view = self.out.create_view(&Default::default());
        let finish = Finish {
            view: &view,
            format: FORMAT,
            at: [0, 0],
            only: &[],
        };
        let stamps = match clock.as_deref_mut() {
            Some(clock) => {
                let timing = clock.start(Group::default());
                let timing = timing.ok_or(RenderError::Invalid("no free timestamp slot"))?;
                Some(clock.stamps(timing))
            }
            None => None,
        };
        let core = self.core;
        let paper = (self.output, [1.0; 4]);
        let scene = (&self.buffers, &self.plan);
        let drawn = (self.targets).draw(core, scene, paper, &finish, stamps.as_ref())?;
        if let (Some(clock), Some(stamps)) = (clock, stamps) {
            clock.finish(&core.gpu, stamps.timing);
        }
        Ok([drawn.upload_ns, drawn.encode_ns])
    }
}

impl Core {
    /// Draws `artwork` on white paper over a region of `size` output pixels whose top-left lies
    /// at canvas pixel `at`, at `scale` output pixels per canvas pixel, through the raster core
    /// with an atlas `atlas` pixels square: planned once, then drawn after a warm-up `repeats`
    /// times (at most 16), each timed.
    ///
    /// # Errors
    /// `Invalid` for a scale other than 1, 2, 4 or 8, or a stroke's region larger than the
    /// atlas; as the GPU otherwise.
    pub fn time_raster(
        &self,
        artwork: &Artwork,
        (scale, at): (u32, [u32; 2]),
        size: [u32; 2],
        (repeats, atlas): (usize, u32),
    ) -> Result<RasterTimed, RenderError> {
        let output = Output::at(scale, at.map(|v| v * scale))?;
        let scene = Scene::build_knowing(artwork, None, (Vec::new(), Earlier::default()))?;
        let buffers = self.residuals_for(artwork, &scene, (output, size))?;
        let started = Instant::now();
        let units = (&scene, buffers.bases());
        let plan = RasterPlan::build(units, (output, size), [atlas; 2])?;
        let plan_ns = started.elapsed().as_secs_f64() * 1e9;
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let mut bench = Bench {
            core: self,
            buffers,
            plan,
            output,
            targets: RasterTargets::new(self, size, atlas)?,
            out: target_texture(&self.gpu, (size, FORMAT), usage)?,
        };
        let mut cpu = Vec::new();
        let warming = Instant::now();
        while (cpu.len() < WARMUP || warming.elapsed() < WARM_TIME)
            && warming.elapsed() < WARM_ENOUGH
        {
            cpu.push(bench.draw(None)?);
            self.wait_idle()?;
        }
        let warm = read_rgba(&self.gpu, &bench.out)?;
        let (times, stamped) = self.raster_runs(&mut bench, repeats.clamp(1, MOST))?;
        let image = read_rgba(&self.gpu, &bench.out)?;
        let median = |at: usize| {
            let mut all: Vec<f64> = cpu.iter().map(|times| times[at]).collect();
            all.sort_by(f64::total_cmp);
            all[all.len() / 2]
        };
        let pipelines = self.raster_pipelines()?;
        let (formats, bytes) = bench.targets.formats();
        Ok(RasterTimed {
            repeatable: warm == image,
            times,
            stamped,
            counts: bench.plan.counts,
            passes: bench.plan.counts.passes,
            atlas: bench.targets.atlas(),
            formats,
            bytes,
            plan_ns,
            upload_ns: median(0),
            encode_ns: median(1),
            pipelines: pipelines.count,
            compile_ns: pipelines.compile_ns,
            image,
        })
    }

    /// `scene`'s buffers on the GPU with the residuals its smudges read over a region of `size`
    /// pixels at `output`, made by a smudge engine of their own (S61).
    fn residuals_for(
        &self,
        artwork: &Artwork,
        scene: &Scene,
        (output, size): (Output, [u32; 2]),
    ) -> Result<SceneBuffers, RenderError> {
        let ops = scene_ops(&self.gpu, scene, None);
        let [x, y] = output.origin.map(|at| at as i32);
        let rect = PixelRect {
            min: [x, y],
            max: [x + size[0] as i32, y + size[1] as i32],
        };
        let near = [around_output(&rect, output.scale)];
        let mut smudges = Smudges::new(self);
        let mut filters = Filters::default();
        let wanted = (&mut filters, &Wanted::Near(&near));
        smudges.prepare(self, (scene, &ops), artwork, wanted)?;
        Ok(SceneBuffers::of(&ops, smudges.held(scene, &filters)))
    }

    /// `repeats` draws of the bench, each timed by GPU timestamps (or by waiting for it): their
    /// times, ns, fastest first, and whether timestamps measured them.
    fn raster_runs(
        &self,
        bench: &mut Bench<'_>,
        repeats: usize,
    ) -> Result<(Vec<f64>, bool), RenderError> {
        let mut times = Vec::new();
        let stamped = match GpuClock::new(&self.gpu) {
            Some(mut clock) => {
                for _ in 0..repeats {
                    bench.draw(Some(&mut clock))?;
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
                    bench.draw(None)?;
                    self.wait_idle()?;
                    times.push(started.elapsed().as_secs_f64() * 1e9);
                }
                false
            }
        };
        times.sort_by(f64::total_cmp);
        Ok((times, stamped))
    }
}
