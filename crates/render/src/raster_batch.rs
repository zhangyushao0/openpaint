//! A batch of raster regions in one command buffer (`raster.rs`): each
//! region's values at an offset of their own in one uniform buffer (bound by a dynamic offset),
//! its instances one after another in the instance buffer, its passes encoded in order, so a
//! frame of a stroke's drawing (its kept coverage laid, the snapshots it takes, its regions) is
//! one submission; the regions share the targets, which every pass of a region sets before it
//! reads them.
//!
//! A region's finish rounds the canvas target into the destination's pixels at each place the
//! region goes to. A stroke's frame (a finish naming its destination's texture) finishes into a
//! target of the destination's format as large as the region targets, and each place's pixels
//! are copied from it: a render pass over the destination itself loads and stores the whole of
//! it where the GPU draws in tiles (wgpu's render area is the whole attachment), the canvas's 19
//! MiB or a view tier's atlas of 25 to 45 MiB every frame, for a region of a few thousand pixels
//! (S134). The copies move the rounded texels as they are: every pixel is the one a pass into
//! the destination writes.

use super::frame::{begin, clear, instance_bytes, region, values};
use super::reads::CANVAS;
use super::{Finish, Pages, Placed, RasterTargets, SceneBuffers, Target};
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu_clock::Stamps;
use crate::output::Output;
use crate::raster_plan::{Pass, RasterPlan};
use crate::readback::{extent, place, target_texture, texture_bytes};

/// The bytes of one region's values (raster.wgsl's `Raster`), and how far apart regions' values
/// lie (the largest uniform offset alignment a device asks for).
pub(super) const VALUES_BYTES: u64 = 128;
pub(super) const VALUES_STRIDE: u64 = 256;

/// One region of a batch: its plan, the output it lies at on its paper, and its finish (none: a
/// snapshot taken, a kept coverage laid).
pub(crate) struct Region<'a> {
    pub(crate) plan: &'a RasterPlan,
    pub(crate) output: Output,
    pub(crate) paper: [f32; 4],
    pub(crate) finish: Option<&'a Finish<'a>>,
}

/// Where the region being encoded finds its values (a dynamic offset) and its instances (bytes).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Bound {
    pub(super) values: u32,
    pub(super) instances: u64,
}

/// The first and the last render pass of `regions` (region, pass; a region's finish is the pass
/// after its last): where a batch's timestamps go (a copy writes none).
pub(super) fn timed(regions: &[Region<'_>]) -> ((usize, usize), (usize, usize)) {
    let renders = |pass: &Pass| {
        !matches!(
            pass,
            Pass::Restore(_) | Pass::Keep(_) | Pass::Copy { .. } | Pass::Zero(_)
        )
    };
    let mut all = Vec::new();
    for (at, region) in regions.iter().enumerate() {
        let passes = &region.plan.passes;
        all.extend(
            (passes.iter().enumerate())
                .filter_map(|(index, pass)| renders(pass).then_some((at, index))),
        );
        if region.finish.is_some() {
            all.push((at, passes.len()));
        }
    }
    let none = (usize::MAX, usize::MAX);
    (
        all.first().copied().unwrap_or(none),
        all.last().copied().unwrap_or(none),
    )
}

impl RasterTargets {
    /// `regions` of the scene in `scene` encoded into one command buffer, their slot passes
    /// reading and writing `pages`, timed from the first pass to the last by `stamps`; their
    /// values and instances land before the next submission.
    ///
    /// # Errors
    /// `Invalid` for a region larger than the targets' capacity or a destination format the
    /// raster core does not finish into; as the GPU.
    pub(crate) fn prepare_all(
        &mut self,
        core: &Core,
        scene: &SceneBuffers,
        (regions, pages): (&[Region<'_>], Pages<'_>),
        stamps: Option<&Stamps>,
    ) -> Result<wgpu::CommandBuffer, RenderError> {
        for region in regions {
            let [width, height] = region.plan.size;
            if width > self.capacity[0] || height > self.capacity[1] || width * height == 0 {
                return Err(RenderError::Invalid("a raster region outside its targets"));
            }
        }
        let pipelines = core.raster_pipelines()?;
        for region in regions {
            self.make_scratch(core, pipelines, region.plan)?;
        }
        self.make_reads(core, pipelines, regions);
        self.make_finished(core, regions)?;
        let started = std::time::Instant::now();
        let bounds = self.upload(core, regions);
        self.uploaded_ns = crate::profile::since(started);
        self.bind_scene(core, pipelines, scene);
        self.encode(core, (pipelines, pages), (regions, &bounds), stamps)
    }

    /// Every region's values and instances written, the buffers grown first when they do not
    /// hold them (group 0 is made again then); where each region's start.
    fn upload(&mut self, core: &Core, regions: &[Region<'_>]) -> Vec<Bound> {
        let atlas = self.atlas();
        let mut bytes = Vec::with_capacity(regions.len() * VALUES_STRIDE as usize);
        let mut instances = Vec::new();
        let mut bounds = Vec::with_capacity(regions.len());
        for region in regions {
            bounds.push(Bound {
                values: bytes.len() as u32,
                instances: instances.len() as u64,
            });
            let start = bytes.len();
            let at = (region.plan.size, region.plan.base);
            bytes.extend(values(region.output, at, atlas, region.paper));
            bytes.resize(start + VALUES_STRIDE as usize, 0);
            // A finish through the finishing target draws once, at its top-left.
            let places: Vec<[i32; 2]> = match region.finish {
                Some(finish) if finish.texture.is_none() => {
                    finish.into.iter().map(|placed| placed.at).collect()
                }
                _ => vec![[0, 0]],
            };
            instances.extend(instance_bytes(&region.plan.instances, &places));
        }
        let gpu = &core.gpu;
        let grown = |label, size: u64, usage| {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.next_power_of_two(),
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        if bytes.len() as u64 > self.values.size() {
            let usage = wgpu::BufferUsages::UNIFORM;
            self.values = grown("raster values", bytes.len() as u64, usage);
            self.scene = None;
        }
        if instances.len() as u64 > self.instances.size() {
            let usage = wgpu::BufferUsages::VERTEX;
            self.instances = grown("raster instances", instances.len() as u64, usage);
        }
        gpu.queue.write_buffer(&self.values, 0, &bytes);
        gpu.queue.write_buffer(&self.instances, 0, &instances);
        bounds
    }

    /// The target the finishes of `regions` that copy go through, made in their destination's
    /// format when there is none in it.
    ///
    /// # Errors
    /// As the GPU.
    fn make_finished(&mut self, core: &Core, regions: &[Region<'_>]) -> Result<(), RenderError> {
        let copied = (regions.iter().filter_map(|region| region.finish))
            .find(|finish| finish.texture.is_some());
        let Some(format) = copied.map(|finish| finish.format) else {
            return Ok(());
        };
        if (self.finished.as_ref()).is_some_and(|made| made.texture.format() == format) {
            return Ok(());
        }
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let texture = target_texture(&core.gpu, (self.capacity, format), usage)?;
        self.finished = Some(Target::of(texture));
        Ok(())
    }

    /// The canvas of a region of `size` rounded into the destination's pixels at each of
    /// `finish`'s places (only those a place names when it names any) by `pipeline`, the
    /// instances from `finished` on naming the places in turn; through the finishing target and
    /// copies when the finish names its destination's texture (its one instance at the target's
    /// top-left), else in one pass over the destination.
    ///
    /// # Errors
    /// `Invalid` when the finishing target is not in the destination's format.
    pub(super) fn finish(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        (pipeline, finish, finished): (&wgpu::RenderPipeline, &Finish<'_>, u32),
        size: [u32; 2],
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) -> Result<(), RenderError> {
        let Some(destination) = finish.texture else {
            self.attached.pass(0, false);
            let mut pass = begin(encoder, finish.view, wgpu::LoadOp::Load, timestamps);
            self.bind(&mut pass, self.reads.get(CANVAS));
            pass.set_pipeline(pipeline);
            for (instance, placed) in (finished..).zip(&finish.into) {
                for part in parts(placed, size) {
                    // The triangle covers the part alone; each pixel reads its own region pixel.
                    region(&mut pass, part.min.map(i32::unsigned_abs), part.size());
                    pass.draw(0..3, instance..instance + 1);
                }
            }
            return Ok(());
        };
        let finishing = (self.finished.as_ref())
            .filter(|made| made.texture.format() == finish.format)
            .ok_or(RenderError::Invalid(
                "no finishing target in the destination's format",
            ))?;
        let mut pass = begin(encoder, &finishing.view, clear(None), timestamps);
        self.bind(&mut pass, self.reads.get(CANVAS));
        pass.set_pipeline(pipeline);
        region(&mut pass, [0, 0], size);
        pass.draw(0..3, finished..finished + 1);
        drop(pass);
        let bounds = PixelRect::full([destination.width(), destination.height()]);
        for placed in &finish.into {
            for part in parts(placed, size) {
                let part = part.intersect(&bounds);
                if part.is_empty() {
                    continue;
                }
                let from = [0, 1].map(|axis| (part.min[axis] - placed.at[axis]).unsigned_abs());
                encoder.copy_texture_to_texture(
                    place(&finishing.texture, from),
                    place(destination, part.min.map(i32::unsigned_abs)),
                    extent(part.size()),
                );
            }
        }
        let bytes = texture_bytes(&finishing.texture);
        self.attached.pass(bytes, true);
        Ok(())
    }
}

/// The destination's pixels a region of `size` writes at `placed`: the whole region there, or
/// the parts of it the place names.
fn parts(placed: &Placed, size: [u32; 2]) -> Vec<PixelRect> {
    let [x, y] = placed.at;
    let whole = PixelRect {
        min: [x, y],
        max: [x + size[0] as i32, y + size[1] as i32],
    };
    if placed.only.is_empty() {
        return vec![whole];
    }
    (placed.only.iter())
        .map(|rect| rect.intersect(&whole))
        .filter(|part| !part.is_empty())
        .collect()
}
