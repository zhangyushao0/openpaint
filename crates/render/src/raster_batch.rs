//! A batch of raster regions in one command buffer (`raster.rs`): each
//! region's values at an offset of their own in one uniform buffer (bound by a dynamic offset),
//! its instances one after another in the instance buffer, its passes encoded in order, so a
//! frame of a stroke's drawing (its kept coverage laid, the snapshots it takes, its regions) is
//! one submission; the regions share the targets, which every pass of a region sets before it
//! reads them.

use super::frame::{instance_bytes, values};
use super::{Finish, Pages, RasterTargets, SceneBuffers};
use crate::core::Core;
use crate::error::RenderError;
use crate::gpu_clock::Stamps;
use crate::output::Output;
use crate::raster_plan::{Pass, RasterPlan};

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
        let bounds = self.upload(core, regions);
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
            let at = region.finish.map_or([0, 0], |finish| finish.at);
            instances.extend(instance_bytes(&region.plan.instances, at));
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
}
