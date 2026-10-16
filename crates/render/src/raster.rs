//! The raster core (S57's probe, product code since S59; DECISIONS D86): a region of a
//! scene's output drawn by the GPU's rasterizer rather than evaluated per pixel over binned ops.
//! Each stroke's coverage (or density, and a paper stroke's contact) lies in a region of a scratch
//! atlas, combined by the blend unit (max or add); each region is composited onto a float layer by
//! the evaluator's paint, fills and patches drawn there too, a mask onto a float target of its
//! own; each layer ends onto a float canvas (or into its clipping chain) by its mode, and the
//! canvas is rounded into the destination as the evaluator rounds it (`raster_plan.rs` the CPU's
//! part, `raster_gpu.rs` the pipelines, `raster_frame.rs` the passes, raster*.wgsl the stages).
//! A smudge is a pass applying its residual to the layer from a copy of it (S61, D88). It draws
//! the view tiers above 1x and exports for every scene `raster_route.rs` routes to it; the 1x
//! canvas, the stroke being drawn, picks and pictures stay on the evaluator until S62.

use std::time::Instant;

use crate::core::Core;
use crate::error::RenderError;
use crate::gpu_clock::Stamps;
use crate::output::Output;
use crate::program::SceneOps;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{Pass, RasterPlan, Scratch};
use crate::readback::target_texture;
use crate::smudge::Held;

#[path = "raster_frame.rs"]
mod frame;
use frame::{Reads, instance_bytes, values};

/// The buffers a scene's ops lie in on the GPU as the raster core binds them, the scene's serial
/// (S60: a worker draws from them while the canvas keeps its scene), and its smudges' residuals
/// (S61).
#[derive(Debug, Clone)]
pub(crate) struct SceneBuffers {
    pub(crate) ops: wgpu::Buffer,
    pub(crate) masks: wgpu::Buffer,
    pub(crate) patches: wgpu::Buffer,
    pub(crate) serial: u64,
    pub(crate) residuals: Held,
}

impl SceneBuffers {
    /// The buffers of `ops`, for the scene named `serial`, its smudges' residuals as `residuals`
    /// holds them.
    pub(crate) fn of(ops: &SceneOps, serial: u64, residuals: Held) -> Self {
        Self {
            ops: ops.ops(),
            masks: ops.masks.clone(),
            patches: ops.patches.clone(),
            serial,
            residuals,
        }
    }

    /// Where each of the scene's smudges' residuals starts in the arena bound.
    pub(crate) fn bases(&self) -> &[Option<u32>] {
        &self.residuals.bases
    }
}

/// The atlas of a view tier's and an export's raster targets, pixels a side: 64 regions of a
/// whole 256-pixel tile, 8 of an export's 1024 x 512 piece.
pub(crate) const ATLAS: u32 = 2048;

/// A region's float targets, the atlas and the bindings the raster core draws through, kept
/// between regions (a view tier's tiles, an export's pieces).
pub(crate) struct RasterTargets {
    /// The largest region it draws, and the atlas.
    capacity: [u32; 2],
    atlas: Target,
    /// By `Scratch`: the canvas, the chain, the layer, the mask, the backdrop; the chain, the
    /// mask and the backdrop made when a region first needs them.
    scratch: [Option<Target>; 5],
    /// Bound where a draw reads nothing.
    unread: Target,
    values: wgpu::Buffer,
    instances: wgpu::Buffer,
    /// Group 0 for the scene of that serial with those residuals.
    scene: Option<(u64, [wgpu::Buffer; 2], wgpu::BindGroup)>,
    /// Group 1, made again when a scratch target is made.
    reads: Option<Reads>,
}

/// A texture with the one view every pass draws into or reads it through (a view made per pass
/// had the driver make a framebuffer per pass).
pub(crate) struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn of(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&Default::default()),
            texture,
        }
    }
}

/// Where a region's pixels go: a destination's view, its format, and the pixel holding the
/// region's top-left.
pub(crate) struct Finish<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) at: [u32; 2],
}

/// What one draw cost the CPU, ns: writing its values and instances, encoding and submitting.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Drawn {
    pub(crate) upload_ns: f64,
    pub(crate) encode_ns: f64,
}

impl RasterTargets {
    /// Targets for regions of up to `capacity` pixels, with an atlas `atlas` pixels a side.
    ///
    /// # Errors
    /// As the GPU: sizes the device cannot hold.
    pub(crate) fn new(core: &Core, capacity: [u32; 2], atlas: u32) -> Result<Self, RenderError> {
        let pipelines = core.raster_pipelines()?;
        let drawn = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let gpu = &core.gpu;
        let buffer = |label, size, usage| {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let mut scratch: [Option<Target>; 5] = Default::default();
        for at in [Scratch::Canvas, Scratch::Layer] {
            scratch[at as usize] = Some(Target::of(target_texture(
                gpu,
                (capacity, pipelines.layer),
                drawn,
            )?));
        }
        Ok(Self {
            capacity,
            atlas: Target::of(target_texture(gpu, ([atlas; 2], pipelines.atlas), drawn)?),
            scratch,
            unread: Target::of(target_texture(gpu, ([1, 1], pipelines.layer), drawn)?),
            values: buffer("raster values", 64, wgpu::BufferUsages::UNIFORM),
            instances: buffer("raster instances", 1 << 16, wgpu::BufferUsages::VERTEX),
            scene: None,
            reads: None,
        })
    }

    /// The largest region it draws.
    pub(crate) const fn capacity(&self) -> [u32; 2] {
        self.capacity
    }

    /// The atlas's side.
    pub(crate) fn atlas(&self) -> [u32; 2] {
        [self.atlas.texture.width(), self.atlas.texture.height()]
    }

    /// Bytes of its textures and buffers.
    pub(crate) fn bytes(&self) -> u64 {
        let texels = |texture: &wgpu::Texture| {
            let texel = texture.format().block_copy_size(None).unwrap_or(16);
            u64::from(texture.width()) * u64::from(texture.height()) * u64::from(texel)
        };
        let scratch: u64 = (self.scratch.iter().flatten())
            .map(|at| texels(&at.texture))
            .sum();
        texels(&self.atlas.texture) + scratch + self.instances.size() + self.values.size()
    }

    /// The atlas's and the canvas's texel formats and bytes (the benchmark's line).
    pub(crate) fn formats(&self) -> ([String; 2], [u64; 2]) {
        let canvas = &self.scratch[0].as_ref().unwrap_or(&self.atlas).texture;
        let formats =
            [&self.atlas.texture, canvas].map(|texture| format!("{:?}", texture.format()));
        let bytes = [&self.atlas.texture, canvas].map(|texture| {
            let texel = texture.format().block_copy_size(None).unwrap_or(16);
            u64::from(texture.width()) * u64::from(texture.height()) * u64::from(texel)
        });
        (formats, bytes)
    }

    /// Draws `plan` (of the scene in `scene`) at `output` on paper `background`, rounds it into
    /// `finish`, and submits it; its passes timed by `stamps`.
    ///
    /// # Errors
    /// As `prepare`.
    pub(crate) fn draw(
        &mut self,
        core: &Core,
        (scene, plan): (&SceneBuffers, &RasterPlan),
        (output, background): (Output, [f32; 4]),
        finish: &Finish<'_>,
        stamps: Option<&Stamps>,
    ) -> Result<Drawn, RenderError> {
        let place = (output, background);
        let (buffer, drawn) = self.prepare(core, (scene, plan), place, finish, stamps)?;
        core.gpu.submit([buffer]);
        Ok(drawn)
    }

    /// As `draw`, the commands encoded but not submitted (S60: a worker submits them once the
    /// GPU's queue has room); its values and instances land before the next submission.
    ///
    /// # Errors
    /// `Invalid` for a region larger than its capacity or a destination format the raster core
    /// does not finish into; as the GPU.
    pub(crate) fn prepare(
        &mut self,
        core: &Core,
        (scene, plan): (&SceneBuffers, &RasterPlan),
        (output, background): (Output, [f32; 4]),
        finish: &Finish<'_>,
        stamps: Option<&Stamps>,
    ) -> Result<(wgpu::CommandBuffer, Drawn), RenderError> {
        let [width, height] = plan.size;
        if width > self.capacity[0] || height > self.capacity[1] || width * height == 0 {
            return Err(RenderError::Invalid("a raster region outside its targets"));
        }
        let pipelines = core.raster_pipelines()?;
        let started = Instant::now();
        self.make_scratch(core, pipelines, plan)?;
        self.bind_scene(core, pipelines, scene);
        let gpu = &core.gpu;
        let atlas = self.atlas();
        gpu.queue.write_buffer(
            &self.values,
            0,
            &values(output, plan.size, atlas, background),
        );
        let bytes = instance_bytes(&plan.instances, finish.at);
        if bytes.len() as u64 > self.instances.size() {
            self.instances = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("raster instances"),
                size: (bytes.len() as u64).next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        gpu.queue.write_buffer(&self.instances, 0, &bytes);
        let uploaded = Instant::now();
        let finished = plan.instances.len() as u32;
        let buffer = self.encode(
            core,
            pipelines,
            (plan, background),
            (finish, finished),
            stamps,
        )?;
        let drawn = Drawn {
            upload_ns: (uploaded - started).as_secs_f64() * 1e9,
            encode_ns: uploaded.elapsed().as_secs_f64() * 1e9,
        };
        Ok((buffer, drawn))
    }

    /// The chain, the mask and the backdrop, when `plan` uses one not made yet.
    fn make_scratch(
        &mut self,
        core: &Core,
        pipelines: &RasterPipelines,
        plan: &RasterPlan,
    ) -> Result<(), RenderError> {
        for pass in &plan.passes {
            let used = match pass {
                Pass::Draw { target, .. } | Pass::Clear(target) => [Some(*target), None],
                Pass::Copy(_) => [Some(Scratch::Backdrop), None],
                Pass::Smudge { target, .. } => [Some(*target), Some(Scratch::Backdrop)],
                Pass::End { target, reads, .. } => [Some(*target), Some(*reads)],
                Pass::Coverage(_) => [None, None],
            };
            for at in used.into_iter().flatten() {
                if self.scratch[at as usize].is_some() {
                    continue;
                }
                let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST;
                let format = (self.capacity, pipelines.layer);
                self.scratch[at as usize] =
                    Some(Target::of(target_texture(&core.gpu, format, usage)?));
                self.reads = None;
            }
        }
        Ok(())
    }

    /// Group 0 for `scene`, made when the scene or its residuals' buffers are new.
    fn bind_scene(&mut self, core: &Core, pipelines: &RasterPipelines, scene: &SceneBuffers) {
        let (serial, residuals) = (scene.serial, &scene.residuals.buffers);
        if (self.scene.as_ref())
            .is_some_and(|(held, bound, _)| *held == serial && bound == residuals)
        {
            return;
        }
        let [arena, active] = residuals;
        let buffers = [
            &self.values,
            &scene.ops,
            &scene.masks,
            &core.evaluate.textures,
            &scene.patches,
            arena,
            active,
        ];
        let group = core
            .gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raster scene"),
                layout: &pipelines.layouts[0],
                entries: &(0..)
                    .zip(buffers)
                    .map(|(binding, buffer)| wgpu::BindGroupEntry {
                        binding,
                        resource: buffer.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
            });
        self.scene = Some((serial, residuals.clone(), group));
    }
}

impl Core {
    /// The raster core's pipelines, made by the first caller (how long that took is logged), any
    /// other waiting until they are: the runtime makes them on its raster worker as the device is
    /// made (S60), so the screen's frames never wait for them (`raster_made`).
    pub(crate) fn raster_pipelines(&self) -> Result<&RasterPipelines, RenderError> {
        if let Some(pipelines) = self.raster.get() {
            return Ok(pipelines);
        }
        let _one = self
            .making
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(pipelines) = self.raster.get() {
            return Ok(pipelines);
        }
        let started = Instant::now();
        let mut made = RasterPipelines::new(&self.gpu)?;
        made.compile_ns = started.elapsed().as_secs_f64() * 1e9;
        log::info!(
            "raster core: {} pipelines made in {:.0} ms ({:?} atlas, {:?} targets)",
            made.count,
            made.compile_ns / 1e6,
            made.atlas,
            made.layer
        );
        Ok(self.raster.get_or_init(|| made))
    }

    /// The raster core's pipelines when they are made, without waiting (S60).
    pub(crate) fn raster_made(&self) -> Option<&RasterPipelines> {
        self.raster.get()
    }

    /// Makes the raster core's pipelines now unless they are made (S60: the runtime's raster
    /// worker, as the device is made, so the first zoom finds them).
    ///
    /// # Errors
    /// As the GPU: a shader the device refuses.
    pub fn make_raster_pipelines(&self) -> Result<(), RenderError> {
        self.raster_pipelines().map(drop)
    }
}

#[path = "raster_bench.rs"]
mod bench;
pub use bench::RasterTimed;

#[path = "raster_difference.rs"]
mod difference;
pub use difference::{EDGE, ImageDifference, image_difference};

#[path = "raster_passes.rs"]
mod passes;
pub use passes::PassCost;
