//! The raster core: a region of a scene's output drawn by the GPU's rasterizer.
//! Each stroke's coverage (or density, and a paper stroke's contact) lies in a region of a scratch
//! atlas, combined by the blend unit (max or add); each region is composited onto a float layer by
//! the stroke's paint, fills and patches drawn there too, a mask onto a float target of its own;
//! each layer ends onto a float canvas (or into its clipping chain) by its mode, and the canvas is
//! rounded into the destination (`raster_plan.rs` the CPU's part, `raster_gpu.rs` the pipelines,
//! `raster_frame.rs` the passes, raster*.wgsl the stages). A smudge is a pass applying its
//! residual to the layer from a copy of it; so are an adjustment and a filter's result. It draws
//! every scene: the screen's canvas and its view tiers, exports, layer samples and pictures, brush
//! pictures, a smudge's and a filter's material, and the stroke being drawn (`canvas/drawing.rs`, a
//! frame's regions one batch, `raster_batch.rs`).

use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{Pass, RasterPlan, SCRATCH, Scratch};
use crate::readback::{target_texture, texture_bytes};
use crate::scene_ops::SceneOps;
use crate::smudge::{Bases, Held};
use crate::view_tiles::TIER_TILE;

#[path = "raster_frame.rs"]
mod frame;

#[path = "raster_reads.rs"]
mod reads;
use reads::Reads;

/// The buffers a scene's ops lie in on the GPU as the raster core binds them (a worker draws
/// from them while the canvas keeps its scene), and its smudges' residuals.
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

    /// Where each of the scene's smudges' residuals and filters' results starts in the arena
    /// bound.
    pub(crate) fn bases(&self) -> &Bases {
        &self.residuals.bases
    }
}

/// The atlas of a view tier's and an export's raster targets, pixels a side: 64 regions of a
/// whole 256-pixel tile, 8 of an export's 1024 x 512 piece.
pub(crate) const ATLAS: u32 = 2048;

/// How the atlas and the float targets are used: drawn into, read, and copied into (a stroke's
/// drawing copies snapshots into the targets and coverage into the atlas).
const DRAWN: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_DST);

/// A target no side of which is longer is small (`RasterTargets::sized`).
const SMALL: u32 = 4 * TIER_TILE;

/// A region's float targets, the atlas and the bindings the raster core draws through, kept
/// between regions (a view tier's tiles, an export's pieces).
pub(crate) struct RasterTargets {
    /// The largest region it draws, and the atlas.
    capacity: [u32; 2],
    atlas: Target,
    /// By `Scratch::index`: the canvas, the chain, the layer, the mask, the backdrop, the group's
    /// backdrop and the levels; all but the canvas and the layer made when a region first needs
    /// them.
    scratch: [Option<Target>; SCRATCH],
    /// Bound where a draw reads nothing.
    unread: Target,
    /// Each region's values, `VALUES_STRIDE` bytes apart, and its instances, one after another
    /// (a batch's regions in one command buffer); where the region being encoded starts.
    values: wgpu::Buffer,
    instances: wgpu::Buffer,
    bound: Bound,
    /// Group 0 for the scene's buffers and residuals it was made with.
    scene: Option<([wgpu::Buffer; 5], wgpu::BindGroup)>,
    /// Group 1 by what it reads, made again when a scratch target is made.
    reads: Reads,
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

/// Where a region's pixels go: a destination's view, its format, and each place they go to
/// there (a view tier's tiles each to a slot of its own).
pub(crate) struct Finish<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) into: Vec<Placed>,
}

/// One place a region's pixels go to: the destination pixel holding the region's top-left (it
/// may lie outside the destination when `only` names what is written), and the destination's
/// pixels written from there (a smudge's material, only the tiles its residual reads; a view
/// tier's tile, its part of the region); empty: the whole region.
pub(crate) struct Placed {
    pub(crate) at: [i32; 2],
    pub(crate) only: Vec<PixelRect>,
}

impl<'a> Finish<'a> {
    /// The whole region into `view` of `format`, its top-left at destination pixel `at`.
    pub(crate) fn at(
        view: &'a wgpu::TextureView,
        format: wgpu::TextureFormat,
        at: [u32; 2],
    ) -> Self {
        Self {
            view,
            format,
            into: vec![Placed {
                at: at.map(|at| i32::try_from(at).unwrap_or(i32::MAX)),
                only: Vec::new(),
            }],
        }
    }
}

impl RasterTargets {
    /// Targets for regions of up to `capacity` pixels, with an atlas `atlas` pixels a side.
    ///
    /// # Errors
    /// As the GPU: sizes the device cannot hold.
    pub(crate) fn new(core: &Core, capacity: [u32; 2], atlas: u32) -> Result<Self, RenderError> {
        let pipelines = core.raster_pipelines()?;
        let gpu = &core.gpu;
        let buffer = |label, size, usage| {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let mut targets = Self {
            capacity,
            atlas: Target::of(target_texture(gpu, ([atlas; 2], pipelines.atlas), DRAWN)?),
            scratch: Default::default(),
            unread: Target::of(target_texture(gpu, ([1, 1], pipelines.layer), DRAWN)?),
            values: buffer(
                "raster values",
                4 * VALUES_STRIDE,
                wgpu::BufferUsages::UNIFORM,
            ),
            instances: buffer("raster instances", 1 << 16, wgpu::BufferUsages::VERTEX),
            bound: Bound::default(),
            scene: None,
            reads: Reads::default(),
        };
        for at in [Scratch::Canvas, Scratch::Layer] {
            targets.make(core, pipelines, at)?;
        }
        Ok(targets)
    }

    /// Targets for drawing a canvas of `size` pixels on the frame loop's thread (the
    /// canvas's changes, a whole canvas at once, its view tiers' changes): regions of up to four
    /// view tiles a side (a frame of a stroke's drawing is one region), two on a small canvas
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
        let scratch: u64 = (self.scratch.iter().flatten())
            .map(|at| texture_bytes(&at.texture))
            .sum();
        texture_bytes(&self.atlas.texture) + scratch + self.instances.size() + self.values.size()
    }

    /// Draws `plan` (of the scene in `scene`) at `output` on `paper`, rounds it into `finish`,
    /// and submits it.
    ///
    /// # Errors
    /// `Invalid` for a region larger than its capacity or a destination format the raster core
    /// does not finish into; as the GPU.
    pub(crate) fn draw(
        &mut self,
        core: &Core,
        (scene, plan): (&SceneBuffers, &RasterPlan),
        (output, paper): (Output, [f32; 4]),
        finish: &Finish<'_>,
    ) -> Result<(), RenderError> {
        let region = Region {
            plan,
            output,
            paper,
            finish: Some(finish),
        };
        let buffer = self.prepare_all(core, scene, (&[region], Pages::NONE), None)?;
        core.gpu.submit([buffer]);
        Ok(())
    }

    /// The scratch targets `plan` uses that are not made yet.
    fn make_scratch(
        &mut self,
        core: &Core,
        pipelines: &RasterPipelines,
        plan: &RasterPlan,
    ) -> Result<(), RenderError> {
        for pass in &plan.passes {
            let used: Vec<Scratch> = match pass {
                Pass::Draw { target, .. } | Pass::Clear(target) => vec![*target],
                Pass::Copy { from, to } => vec![*from, *to],
                Pass::Rewrite { target, .. } => vec![*target, Scratch::Backdrop],
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

    /// Every scratch target a stroke's frame may read made now, the group's backdrop when its
    /// place lies in a group (the screen canvas's ahead, while nothing touches the screen,
    /// so no stroke's frame makes one: making one took up to 30 ms on the tablet).
    ///
    /// # Errors
    /// As the GPU.
    pub(crate) fn make_every(&mut self, core: &Core, group: bool) -> Result<(), RenderError> {
        let pipelines = core.raster_pipelines()?;
        let mut every = vec![Scratch::Chain, Scratch::Mask, Scratch::Backdrop];
        if group {
            every.push(Scratch::Group);
        }
        for at in every {
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
        if self.scratch[at.index()].is_some() {
            return Ok(());
        }
        let format = (self.capacity, pipelines.layer);
        self.scratch[at.index()] = Some(Target::of(target_texture(&core.gpu, format, DRAWN)?));
        self.reads = Reads::default();
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
        let buffers = [ops, masks, &core.textures, patches, arena, active];
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

#[path = "raster_slots.rs"]
mod slots;
pub(crate) use slots::Pages;
