//! The evaluator's compute pipeline and one dispatch of a plan into a target.

use wgpu::util::DeviceExt;

use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::plan::Plan;
use crate::protocol::{Op, Params, Record, encode, prelude};
use crate::readback::FORMAT;
use crate::scene::Scene;

const SHADER: &str = include_str!("shaders/evaluate.wgsl");

/// The protocol prelude followed by the evaluator.
pub(crate) fn shader_source() -> String {
    prelude() + SHADER
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
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("evaluate"),
            entries: &[
                buffer_entry(0, wgpu::BufferBindingType::Uniform),
                buffer_entry(1, wgpu::BufferBindingType::Storage { read_only: true }),
                buffer_entry(2, wgpu::BufferBindingType::Storage { read_only: true }),
                buffer_entry(3, wgpu::BufferBindingType::Storage { read_only: true }),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
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

    /// Uploads the scene's ops and the plan, then evaluates every plan tile into `target`.
    pub(crate) fn dispatch(
        &self,
        gpu: &Gpu,
        scene: &Scene,
        plan: &Plan,
        background: [f32; 4],
        target: &wgpu::Texture,
    ) -> Result<(), RenderError> {
        let groups = workgroups(gpu, plan.tiles.len())?;
        let params = Params {
            background,
            tiles: groups.tiles,
            row: groups.row,
            size: [target.width(), target.height()],
        };
        let ops: Vec<Op> = scene.items.iter().map(|item| *item.op()).collect();
        let buffers = [
            buffer(gpu, wgpu::BufferUsages::UNIFORM, &[params]),
            buffer(gpu, wgpu::BufferUsages::STORAGE, &ops),
            buffer(gpu, wgpu::BufferUsages::STORAGE, &plan.tiles),
            buffer(gpu, wgpu::BufferUsages::STORAGE, &plan.indices),
        ];
        let view = target.create_view(&Default::default());
        let mut entries: Vec<_> = (0..)
            .zip(&buffers)
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::TextureView(&view),
        });
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("evaluate"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(groups.row, groups.rows, 1);
        drop(pass);
        gpu.queue.submit([encoder.finish()]);
        Ok(())
    }
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
