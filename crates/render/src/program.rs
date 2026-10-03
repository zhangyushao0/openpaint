//! The evaluator's compute pipeline and one dispatch of a plan into a target.

use wgpu::util::DeviceExt;

use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::plan::Plan;
use crate::protocol::{Op, Params, Record, encode, prelude};
use crate::readback::FORMAT;
use crate::scene::Scene;

const SHADER: &str = include_str!("shaders/evaluate.wgsl");
/// The in-progress stroke's coverage texture: one float per canvas pixel.
pub(crate) const COVERAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

/// The protocol prelude followed by the evaluator.
pub(crate) fn shader_source() -> String {
    prelude() + SHADER
}

/// What one dispatch evaluates.
pub(crate) struct Dispatch<'a> {
    /// The scene's ops and its fill masks (`scene_ops`), uploaded once per scene.
    pub(crate) ops: &'a SceneOps,
    /// The in-progress stroke's ops for this evaluation.
    pub(crate) fresh: &'a [Op],
    pub(crate) plan: &'a Plan,
    pub(crate) background: [f32; 4],
    /// Clears the coverage texture first: the in-progress stroke starts over.
    pub(crate) reset: bool,
}

/// One canvas target's evaluation resources, kept between dispatches: buffers are rewritten
/// in place and the bind group is rebuilt only when a buffer grows or the scene's ops change.
/// A bind group per frame would cycle Vulkan descriptor pools, a periodic multi-millisecond
/// stall.
pub(crate) struct Bindings {
    size: [u32; 2],
    canvas: wgpu::TextureView,
    coverage: wgpu::Texture,
    coverage_view: wgpu::TextureView,
    params: wgpu::Buffer,
    /// Tiles, indices and fresh ops, in binding order 2, 3, 5.
    lists: [wgpu::Buffer; 3],
    /// Built for the scene ops buffer it names.
    group: Option<(wgpu::Buffer, wgpu::BindGroup)>,
}

/// One scene on the GPU: its ops (item index = op index) and its fill masks.
pub(crate) struct SceneOps {
    pub(crate) ops: wgpu::Buffer,
    pub(crate) masks: wgpu::Buffer,
}

impl Bindings {
    pub(crate) fn new(gpu: &Gpu, canvas: &wgpu::Texture, coverage: &wgpu::Texture) -> Self {
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        Self {
            size: [canvas.width(), canvas.height()],
            canvas: canvas.create_view(&Default::default()),
            coverage: coverage.clone(),
            coverage_view: coverage.create_view(&Default::default()),
            params: empty_buffer(gpu, uniform, size_of::<Params>() as u64),
            lists: [0; 3].map(|_| empty_buffer(gpu, storage, MIN_LIST_BYTES)),
            group: None,
        }
    }

    /// Writes `records` into list `slot`, growing it (and dropping the bind group) if needed.
    fn write<R: Record>(&mut self, gpu: &Gpu, slot: usize, records: &[R]) {
        let bytes = encode(records);
        let size = bytes.len() as u64;
        if size > self.lists[slot].size() {
            let usage = self.lists[slot].usage();
            self.lists[slot] = empty_buffer(gpu, usage, size.next_power_of_two());
            self.group = None;
        }
        if size > 0 {
            gpu.queue.write_buffer(&self.lists[slot], 0, &bytes);
        }
    }
}

/// Smallest list buffer: room for 128 ops.
const MIN_LIST_BYTES: u64 = 128 * size_of::<Op>() as u64;

fn empty_buffer(gpu: &Gpu, usage: wgpu::BufferUsages, size: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("evaluate"),
        size,
        usage,
        mapped_at_creation: false,
    })
}

pub(crate) struct EvaluatePipeline {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl EvaluatePipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("evaluate"),
            source: wgpu::ShaderSource::Wgsl(shader_source().into()),
        });
        let read_only = wgpu::BufferBindingType::Storage { read_only: true };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("evaluate"),
            entries: &[
                buffer_entry(0, wgpu::BufferBindingType::Uniform),
                buffer_entry(1, read_only),
                buffer_entry(2, read_only),
                buffer_entry(3, read_only),
                texture_entry(4, wgpu::StorageTextureAccess::WriteOnly, FORMAT),
                buffer_entry(5, read_only),
                texture_entry(6, wgpu::StorageTextureAccess::ReadWrite, COVERAGE_FORMAT),
                buffer_entry(7, read_only),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("evaluate"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("evaluate"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("evaluate"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }

    /// Writes the fresh ops and the plan, then evaluates every plan tile into the canvas.
    pub(crate) fn dispatch(
        &self,
        gpu: &Gpu,
        bindings: &mut Bindings,
        work: &Dispatch<'_>,
    ) -> Result<(), RenderError> {
        let groups = workgroups(gpu, work.plan.tiles.len())?;
        let params = Params {
            background: work.background,
            tiles: groups.tiles,
            row: groups.row,
            size: bindings.size,
        };
        gpu.queue
            .write_buffer(&bindings.params, 0, &encode(&[params]));
        bindings.write(gpu, 0, &work.plan.tiles);
        bindings.write(gpu, 1, &work.plan.indices);
        bindings.write(gpu, 2, work.fresh);
        if bindings
            .group
            .as_ref()
            .is_none_or(|(ops, _)| *ops != work.ops.ops)
        {
            let group = self.bind_group(gpu, bindings, work.ops);
            bindings.group = Some((work.ops.ops.clone(), group));
        }
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        if work.reset {
            clear_coverage(&mut encoder, &bindings.coverage);
        }
        if let Some((_, group)) = &bindings.group {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(groups.row, groups.rows, 1);
        }
        gpu.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn bind_group(&self, gpu: &Gpu, bindings: &Bindings, scene: &SceneOps) -> wgpu::BindGroup {
        let [tiles, indices, fresh] = &bindings.lists;
        let buffers = [
            (0, &bindings.params),
            (1, &scene.ops),
            (2, tiles),
            (3, indices),
            (5, fresh),
            (7, &scene.masks),
        ];
        let mut entries: Vec<_> = buffers
            .into_iter()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        for (binding, view) in [(4, &bindings.canvas), (6, &bindings.coverage_view)] {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("evaluate"),
            layout: &self.layout,
            entries: &entries,
        })
    }
}

/// The scene's ops (item index = op index) and its fill masks as GPU buffers.
pub(crate) fn scene_ops(gpu: &Gpu, scene: &Scene) -> SceneOps {
    let ops: Vec<Op> = scene.items.iter().map(|item| item.op()).collect();
    SceneOps {
        ops: buffer(gpu, wgpu::BufferUsages::STORAGE, &ops),
        masks: buffer(gpu, wgpu::BufferUsages::STORAGE, &scene.masks),
    }
}

/// Zero coverage everywhere, through a render pass: clearing a texture directly needs an
/// optional device feature.
fn clear_coverage(encoder: &mut wgpu::CommandEncoder, coverage: &wgpu::Texture) {
    let view = coverage.create_view(&Default::default());
    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear coverage"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    drop(pass);
}

fn buffer_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(
    binding: u32,
    access: wgpu::StorageTextureAccess,
    format: wgpu::TextureFormat,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

/// A buffer holding `records`. An empty list still gets one zeroed record, because a binding
/// must hold at least one element; no tile refers to it.
fn buffer<R: Record>(gpu: &Gpu, usage: wgpu::BufferUsages, records: &[R]) -> wgpu::Buffer {
    let mut contents = encode(records);
    if contents.is_empty() {
        contents.resize(size_of::<R>(), 0);
    }
    gpu.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("evaluate"),
            contents: &contents,
            usage,
        })
}

/// Tiles laid out as rows of workgroups, each row at most the device's per-dimension limit.
struct Workgroups {
    tiles: u32,
    row: u32,
    rows: u32,
}

fn workgroups(gpu: &Gpu, tiles: usize) -> Result<Workgroups, RenderError> {
    let limit = gpu.device.limits().max_compute_workgroups_per_dimension;
    let too_many = RenderError::Invalid("too many tiles for one evaluation");
    let tiles = u32::try_from(tiles).map_err(|_| too_many.clone())?;
    let row = tiles.clamp(1, limit);
    let rows = tiles.div_ceil(row);
    if rows > limit {
        return Err(too_many);
    }
    Ok(Workgroups { tiles, row, rows })
}
