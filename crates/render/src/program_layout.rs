//! The evaluator's bind group layout entries (storage and uniform buffers, storage textures,
//! all for the compute stage) and how a plan's tiles lay out as workgroups.

use crate::error::RenderError;
use crate::gpu::Gpu;

pub(super) fn buffer_entry(
    binding: u32,
    ty: wgpu::BufferBindingType,
) -> wgpu::BindGroupLayoutEntry {
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

pub(super) fn texture_entry(
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
