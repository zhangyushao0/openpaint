//! The evaluator's bind group layout entries (storage and uniform buffers, storage textures,
//! all for the compute stage) and how a plan's tiles lay out as workgroups.

use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::readback::FORMAT;

use super::COVERAGE_FORMAT;

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

/// A one-texel storage texture in `format`, bound where a target does not write (S37).
pub(super) fn unwritten(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::TextureView {
    let one = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("unwritten"),
            size: one,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
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
pub(super) fn evaluate_layout(
    device: &wgpu::Device,
    display: wgpu::TextureFormat,
) -> wgpu::BindGroupLayout {
    let read_only = wgpu::BufferBindingType::Storage { read_only: true };
    let kept = |binding| {
        texture_entry(
            binding,
            wgpu::StorageTextureAccess::ReadWrite,
            COVERAGE_FORMAT,
        )
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("evaluate"),
        entries: &[
            buffer_entry(0, wgpu::BufferBindingType::Uniform),
            buffer_entry(1, read_only),
            buffer_entry(2, read_only),
            buffer_entry(3, read_only),
            texture_entry(4, wgpu::StorageTextureAccess::WriteOnly, FORMAT),
            buffer_entry(5, read_only),
            kept(6),
            buffer_entry(7, read_only),
            kept(8),
            buffer_entry(9, read_only),
            buffer_entry(10, read_only),
            buffer_entry(11, read_only),
            buffer_entry(12, read_only),
            // The preview snapshot: written by one evaluation, read by later ones (S35).
            buffer_entry(13, wgpu::BufferBindingType::Storage { read_only: false }),
            // The display texture (S37), ten bits where the device has them.
            texture_entry(14, wgpu::StorageTextureAccess::WriteOnly, display),
        ],
    })
}
