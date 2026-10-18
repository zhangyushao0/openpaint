//! The raster core (S57's probe, product code since S59; DECISIONS D86): a region of a
//! scene's output drawn by the GPU's rasterizer rather than evaluated per pixel over binned ops.
//! Each stroke's coverage (or density, and a paper stroke's contact) lies in a region of a scratch
//! atlas, combined by the blend unit (max or add); each region is composited onto a float layer by
//! the evaluator's paint, fills and patches drawn there too, a mask onto a float target of its
//! own; each layer ends onto a float canvas (or into its clipping chain) by its mode, and the
//! canvas is rounded into the destination as the evaluator rounds it (`raster_plan.rs` the CPU's
//! part, `raster_gpu.rs` the pipelines, `raster_frame.rs` the passes, raster*.wgsl the stages).
//! A smudge is a pass applying its residual to the layer from a copy of it (S61, D88). It draws
//! every scene `raster_route.rs` routes to it: the screen's canvas and its view tiers, exports,
//! layer samples and pictures, brush pictures and a smudge's material (S62, D89), and the stroke
//! being drawn (S63, D90: `drawing.rs`, a frame's regions one batch, `raster_batch.rs`).

use crate::core::Core;
use crate::error::RenderError;
use crate::gpu_clock::Stamps;
use crate::output::Output;
use crate::program::SceneOps;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{Pass, RasterPlan, Scratch};
use crate::readback::target_texture;
use crate::smudge::Held;
use crate::view_tiles::TIER_TILE;

#[path = "raster_frame.rs"]
mod frame;
use frame::Reads;

/// The buffers a scene's ops lie in on the GPU as the raster core binds them (S60: a worker draws
/// from them while the canvas keeps its scene), and its smudges' residuals (S61).
#[derive(Debug, Clone)]
pub(crate) struct SceneBuffers {
    pub(crate) ops: wgpu::Buffer,
    pub(crate) masks: wgpu::Buffer,
    pub(crate) patches: wgpu::Buffer,
    pub(crate) residuals: Held,
}

impl SceneBuffers {
    /// The buffers of `ops`, its smudges' residuals as `residuals` holds them.
    pub(crate) fn of(ops: &SceneOps, residuals: Held) -> Self {
        Self {
            ops: ops.ops(),
            masks: ops.masks.clone(),
            patches: ops.patches.clone(),
            residuals,
        }
    }

    /// The five buffers group 0 binds beside the values: what names a bind group of them.
    fn bound(&self) -> [wgpu::Buffer; 5] {
        let [arena, active] = self.residuals.buffers.clone();
        [
            self.ops.clone(),
            self.masks.clone(),
            self.patches.clone(),
            arena,
            active,
        ]
    }

    /// Where each of the scene's smudges' residuals starts in the arena bound.
    pub(crate) fn bases(&self) -> &[Option<u32>] {
        &self.residuals.bases
    }
}

/// The atlas of a view tier's and an export's raster targets, pixels a side: 64 regions of a
/// whole 256-pixel tile, 8 of an export's 1024 x 512 piece.
pub(crate) const ATLAS: u32 = 2048;

/// A target no side of which is longer is small (`RasterTargets::sized`).
const SMALL: u32 = 4 * TIER_TILE;

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
    /// Each region's values, `VALUES_STRIDE` bytes apart, and its instances, one after another
    /// (S63: a batch's regions in one command buffer); where the region being encoded starts.
    values: wgpu::Buffer,
    instances: wgpu::Buffer,
    bound: Bound,
    /// Group 0 for the scene's buffers and residuals it was made with.
    scene: Option<([wgpu::Buffer; 5], wgpu::BindGroup)>,
    /// Group 1, made again when a scratch target is made.
    reads: Option<Reads>,
}

/// A texture with the one view every pass draws into or reads it through (a view made per pass
/// had the driver make a framebuffer per pass).
#[derive(Clone)]
pub(crate) struct Target {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

impl Target {
    pub(crate) fn of(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&Default::default()),
            texture,
        }
    }
}

/// Where a region's pixels go: a destination's view, its format, the pixel holding the region's
/// top-left, and the destination's pixels it writes (S62: a smudge's material, only the tiles its
/// residual reads); empty: the whole region.
pub(crate) struct Finish<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) at: [u32; 2],
    pub(crate) only: &'a [crate::geom::PixelRect],
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
        // S63: a stroke's drawing copies snapshots into the targets and coverage into the atlas.
        let drawn = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST;
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
            values: buffer(
                "raster values",
                4 * VALUES_STRIDE,
                wgpu::BufferUsages::UNIFORM,
            ),
            instances: buffer("raster instances", 1 << 16, wgpu::BufferUsages::VERTEX),
            bound: Bound::default(),
            scene: None,
            reads: None,
        })
    }

    /// Targets for drawing a canvas of `size` pixels on the frame loop's thread (S62: the
    /// canvas's changes, a whole canvas at once, its view tiers' changes): regions of up to four
    /// view tiles a side (S63: a frame of a stroke's drawing is one region), two on a small canvas
    /// (a brush's picture, the editor's strip), no larger than the canvas but for a view tile,
    /// and an atlas holding two of them (a pencil's density and contact).
    ///
    /// # Errors
    /// As `new`.
    pub(crate) fn sized(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let small = size.iter().all(|side| *side <= SMALL);
        let most = if small { 2 * TIER_TILE } else { 4 * TIER_TILE };
        // A view tier's tile, drawn in place, at least.
        let capacity = size.map(|side| side.clamp(TIER_TILE, most));
        let longest = capacity[0].max(capacity[1]);
        let atlas = (2 * longest)
            .next_power_of_two()
            .clamp(2 * TIER_TILE, ATLAS);
        Self::new(core, capacity, atlas)
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
        (output, paper): (Output, [f32; 4]),
        finish: &Finish<'_>,
        stamps: Option<&Stamps>,
    ) -> Result<(wgpu::CommandBuffer, Drawn), RenderError> {
        let region = Region {
            plan,
            output,
            paper,
            finish: Some(finish),
        };
        self.prepare_all(core, scene, (&[region], Pages::NONE), stamps)
    }

    /// The chain, the mask and the backdrop, when `plan` uses one not made yet.
    fn make_scratch(
        &mut self,
        core: &Core,
        pipelines: &RasterPipelines,
        plan: &RasterPlan,
    ) -> Result<(), RenderError> {
        for pass in &plan.passes {
            let used: Vec<Scratch> = match pass {
                Pass::Draw { target, .. } | Pass::Clear(target) => vec![*target],
                Pass::Copy(_) => vec![Scratch::Backdrop],
                Pass::Smudge { target, .. } => vec![*target, Scratch::Backdrop],
                Pass::End { target, reads, .. } => vec![*target, *reads],
                Pass::Restore(copies) | Pass::Keep(copies) => {
                    copies.iter().map(|copy| copy.target).collect()
                }
                Pass::Coverage(_) | Pass::Kept { .. } | Pass::Zero(_) => Vec::new(),
            };
            for at in used {
                self.make(core, pipelines, at)?;
            }
        }
        Ok(())
    }

    /// Every scratch target made now (S63: the screen canvas's ahead, while nothing touches the
    /// screen, so no stroke's frame makes one: making one took up to 30 ms on the tablet).
    ///
    /// # Errors
    /// As the GPU.
    pub(crate) fn make_every(&mut self, core: &Core) -> Result<(), RenderError> {
        let pipelines = core.raster_pipelines()?;
        for at in [Scratch::Chain, Scratch::Mask, Scratch::Backdrop] {
            self.make(core, pipelines, at)?;
        }
        Ok(())
    }

    /// Scratch target `at`, made when it is not.
    fn make(
        &mut self,
        core: &Core,
        pipelines: &RasterPipelines,
        at: Scratch,
    ) -> Result<(), RenderError> {
        if self.scratch[at as usize].is_some() {
            return Ok(());
        }
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST;
        let format = (self.capacity, pipelines.layer);
        self.scratch[at as usize] = Some(Target::of(target_texture(&core.gpu, format, usage)?));
        self.reads = None;
        Ok(())
    }

    /// Group 0 for `scene`, made when its buffers or its residuals' are new (or the values'): the
    /// values bound one region's worth at a time, at the offset of the region drawn.
    fn bind_scene(&mut self, core: &Core, pipelines: &RasterPipelines, scene: &SceneBuffers) {
        let bound = scene.bound();
        if (self.scene.as_ref()).is_some_and(|(held, _)| *held == bound) {
            return;
        }
        let [ops, masks, patches, arena, active] = &bound;
        let buffers = [ops, masks, &core.evaluate.textures, patches, arena, active];
        let values = wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &self.values,
                offset: 0,
                size: wgpu::BufferSize::new(VALUES_BYTES),
            }),
        };
        let entries: Vec<wgpu::BindGroupEntry<'_>> = std::iter::once(values)
            .chain(
                (1..)
                    .zip(buffers)
                    .map(|(binding, buffer)| wgpu::BindGroupEntry {
                        binding,
                        resource: buffer.as_entire_binding(),
                    }),
            )
            .collect();
        let group = core
            .gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raster scene"),
                layout: &pipelines.layouts[0],
                entries: &entries,
            });
        self.scene = Some((bound, group));
    }
}

#[path = "raster_batch.rs"]
mod batch;
pub(crate) use batch::Region;
use batch::{Bound, VALUES_BYTES, VALUES_STRIDE, timed};

#[path = "raster_bench.rs"]
mod bench;
pub use bench::RasterTimed;

#[path = "raster_difference.rs"]
mod difference;
pub use difference::{EDGE, ImageDifference, image_difference};

#[path = "raster_passes.rs"]
mod passes;
pub use passes::PassCost;

#[path = "raster_slots.rs"]
mod slots;
pub(crate) use slots::Pages;
