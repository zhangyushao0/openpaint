//! One output region of the raster probe on the GPU (S57): its targets (the scratch atlas, the
//! float layer, the 8-bit result), the plan's instances and the values, bound once, and the
//! render passes of one evaluation: per batch the coverage into the atlas and the composites onto
//! the layer, each batch its own command buffer, then the layer onto the paper.

use wgpu::util::DeviceExt;

use crate::core::Core;
use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::program::SceneOps;
use crate::raster_gpu::{INSTANCE_BYTES, RasterPipelines};
use crate::raster_plan::{Batch, Instance, RasterPlan};
use crate::readback::{FORMAT, target_texture};

pub(super) struct Frame {
    instances: wgpu::Buffer,
    tiles: wgpu::Buffer,
    /// The atlas, the layer, the result.
    views: [wgpu::TextureView; 3],
    pub(super) out: wgpu::Texture,
    /// The scene's buffers; the atlas or the layer as what a composite or the finish reads.
    scene: wgpu::BindGroup,
    reads: [wgpu::BindGroup; 2],
    bytes: [u64; 2],
    /// CPU time to encode and upload the instances and the values, ns.
    pub(super) upload_ns: f64,
}

impl Frame {
    /// The targets, buffers and bind groups of `plan` over `ops`, a target of `size` at
    /// `output`, an atlas of `atlas` pixels.
    pub(super) fn new(
        core: &Core,
        pipelines: &RasterPipelines,
        (plan, ops): (&RasterPlan, &SceneOps),
        (output, size, atlas): (Output, [u32; 2], [u32; 2]),
    ) -> Result<Self, RenderError> {
        let gpu = &core.gpu;
        let device = &gpu.device;
        let drawn = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let atlas_texture = target_texture(gpu, (atlas, pipelines.atlas), drawn)?;
        let layer = target_texture(gpu, (size, pipelines.layer), drawn)?;
        let out = target_texture(gpu, (size, FORMAT), wgpu::TextureUsages::RENDER_ATTACHMENT)?;
        let views =
            [&atlas_texture, &layer, &out].map(|texture| texture.create_view(&Default::default()));
        let started = std::time::Instant::now();
        let values = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("raster probe values"),
            contents: &values(output, size, atlas, plan.opacity),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let [instances, tiles] = [&plan.instances, &plan.tiles].map(|list| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raster probe instances"),
                contents: &instance_bytes(list),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        let upload_ns = started.elapsed().as_secs_f64() * 1e9;
        let buffers = [&values, &ops.ops(), &ops.masks, &core.evaluate.textures];
        let scene = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster probe scene"),
            layout: &pipelines.layouts[0],
            entries: &(0..)
                .zip(buffers)
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding,
                    resource: buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });
        let reads = [&views[0], &views[1]].map(|view| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raster probe source"),
                layout: &pipelines.layouts[1],
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                }],
            })
        });
        let texels = |[w, h]: [u32; 2]| u64::from(w) * u64::from(h);
        let bytes = [
            texels(atlas) * u64::from(pipelines.atlas.block_copy_size(None).unwrap_or(4)),
            texels(size) * u64::from(pipelines.layer.block_copy_size(None).unwrap_or(16)),
        ];
        Ok(Self {
            instances,
            tiles,
            views,
            out,
            scene,
            reads,
            bytes,
            upload_ns,
        })
    }

    /// The atlas's and the layer's bytes.
    pub(super) const fn bytes(&self) -> [u64; 2] {
        self.bytes
    }

    /// Render passes one evaluation of `plan` takes.
    pub(super) fn passes(&self, plan: &RasterPlan) -> usize {
        2 * plan.batches.len() + 1 + usize::from(plan.batches.is_empty())
    }

    /// One evaluation's command buffers, timed by `timing` from the first pass's start to the
    /// last's end.
    pub(super) fn encode(
        &self,
        gpu: &Gpu,
        pipelines: &RasterPipelines,
        plan: &RasterPlan,
        timing: Option<(&GpuClock, Timing)>,
    ) -> Vec<wgpu::CommandBuffer> {
        let writes = |ends| timing.and_then(|(clock, timing)| clock.render_writes(timing, ends));
        let mut buffers = Vec::with_capacity(plan.batches.len() + 1);
        for (index, batch) in plan.batches.iter().enumerate() {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            self.batch(
                &mut encoder,
                pipelines,
                batch,
                (index == 0, writes((index == 0, false))),
            );
            buffers.push(encoder.finish());
        }
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let first = plan.batches.is_empty();
        if first {
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            drop(begin(
                &mut encoder,
                &self.views[1],
                clear,
                writes((true, false)),
            ));
        }
        {
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            let mut pass = begin(&mut encoder, &self.views[2], clear, writes((false, true)));
            pass.set_bind_group(0, &self.scene, &[]);
            pass.set_bind_group(1, &self.reads[1], &[]);
            pass.set_pipeline(&pipelines.finish);
            pass.draw(0..3, 0..1);
        }
        buffers.push(encoder.finish());
        buffers
    }
}

impl Frame {
    /// One batch's two passes: its strokes' coverage into the atlas (the `first` batch's
    /// timed from its start by `writes`), then their composites onto the layer (cleared first by
    /// the `first` batch).
    fn batch<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        pipelines: &RasterPipelines,
        batch: &Batch,
        (first, writes): (bool, Option<wgpu::RenderPassTimestampWrites<'a>>),
    ) {
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        {
            let mut pass = begin(encoder, &self.views[0], clear, writes);
            pass.set_bind_group(0, &self.scene, &[]);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            for draw in (batch.coverage.iter()).filter(|draw| !draw.instances.is_empty()) {
                let [x, y, width, height] = draw.scissor;
                pass.set_pipeline(pipelines.coverage(draw.coverage));
                pass.set_scissor_rect(x, y, width, height);
                pass.draw(0..4, draw.instances.clone());
            }
        }
        let load = if first { clear } else { wgpu::LoadOp::Load };
        let mut pass = begin(encoder, &self.views[1], load, None);
        pass.set_bind_group(0, &self.scene, &[]);
        pass.set_bind_group(1, &self.reads[0], &[]);
        pass.set_vertex_buffer(0, self.tiles.slice(..));
        for draw in (batch.composite.iter()).filter(|draw| !draw.tiles.is_empty()) {
            pass.set_pipeline(pipelines.composite(draw.blend));
            pass.draw(0..4, draw.tiles.clone());
        }
    }
}

/// A render pass over `view`, starting by `load`, storing what it draws.
fn begin<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
    timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'a>>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("raster probe"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        timestamp_writes,
        ..Default::default()
    })
}

/// The values raster.wgsl's `Raster` holds, as GPU bytes.
fn values(output: Output, size: [u32; 2], atlas: [u32; 2], opacity: f32) -> Vec<u8> {
    let (scale, inverse) = output.operands();
    let floats = [scale, inverse, scale.log2(), opacity, 1.0, 1.0, 1.0, 1.0];
    let mut bytes: Vec<u8> = floats.iter().flat_map(|v| v.to_le_bytes()).collect();
    for origin in output.origin {
        bytes.extend(i32::try_from(origin).unwrap_or(i32::MAX).to_le_bytes());
    }
    let sizes = [size[0], size[1], atlas[0], atlas[1]].map(|side| side as f32);
    bytes.extend(
        sizes
            .iter()
            .chain(&[0.0, 0.0])
            .flat_map(|v| v.to_le_bytes()),
    );
    bytes
}

/// The instances as GPU bytes; one zeroed instance for none (a buffer holds something).
fn instance_bytes(list: &[Instance]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((list.len().max(1)) * INSTANCE_BYTES as usize);
    for instance in list {
        bytes.extend(instance.a.iter().flat_map(|v| v.to_le_bytes()));
        bytes.extend(instance.b.iter().flat_map(|v| v.to_le_bytes()));
    }
    if bytes.is_empty() {
        bytes.resize(INSTANCE_BYTES as usize, 0);
    }
    bytes
}
