//! The evaluator's source, its bind group layout entries (storage and uniform buffers, storage
//! textures, all for the compute stage) and how a plan's tiles lay out as workgroups.

use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::protocol::prelude;
use crate::readback::FORMAT;

const SHADER: &str = include_str!("shaders/evaluate.wgsl");
const SEGMENT: &str = include_str!("shaders/segment.wgsl");
const STAMP: &str = include_str!("shaders/stamp.wgsl");
const STAMP_TEXTURE: &str = include_str!("shaders/stamp_texture.wgsl");
const DEPOSIT: &str = include_str!("shaders/deposit.wgsl");
const SMUDGE: &str = include_str!("shaders/smudge_composite.wgsl");
const ADJUST: &str = include_str!("shaders/adjust.wgsl");
const FILL_EDGE: &str = include_str!("shaders/fill_edge.wgsl");
const FILTER: &str = include_str!("shaders/filter_composite.wgsl");

/// The protocol prelude followed by the evaluator, its curve spans, their stamps, its
/// accumulating deposits, its smudges, its adjustments, its fill edges and its filters.
pub(crate) fn shader_source() -> String {
    let source = prelude() + SHADER + SEGMENT + STAMP + STAMP_TEXTURE + DEPOSIT + SMUDGE;
    source + ADJUST + FILL_EDGE + FILTER
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

/// Tiles laid out as rows of workgroups, each row at most the device's per-dimension limit.
pub(super) struct Workgroups {
    pub(super) tiles: u32,
    pub(super) row: u32,
    pub(super) rows: u32,
}

pub(super) fn workgroups(gpu: &Gpu, tiles: usize) -> Result<Workgroups, RenderError> {
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

/// The evaluator's bindings, in the order `evaluate.wgsl` declares them.
pub(super) fn evaluate_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let read_only = wgpu::BufferBindingType::Storage { read_only: true };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("evaluate"),
        entries: &[
            buffer_entry(0, wgpu::BufferBindingType::Uniform),
            buffer_entry(1, read_only),
            buffer_entry(2, read_only),
            buffer_entry(3, read_only),
            texture_entry(4, wgpu::StorageTextureAccess::WriteOnly, FORMAT),
            buffer_entry(7, read_only),
            buffer_entry(9, read_only),
            buffer_entry(10, read_only),
            buffer_entry(11, read_only),
            buffer_entry(12, read_only),
        ],
    })
}
