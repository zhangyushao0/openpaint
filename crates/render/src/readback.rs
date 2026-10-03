//! Offscreen targets and GPU-to-memory readback.

use std::sync::mpsc;

use crate::error::{RenderError, device};
use crate::gpu::Gpu;
use crate::image::RgbaImage;

/// The offscreen colour format: 8 bits per channel, no sRGB conversion on write or read.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A texture that can be rendered into and then read back.
pub(crate) fn offscreen_texture(gpu: &Gpu, size: [u32; 2]) -> Result<wgpu::Texture, RenderError> {
    target_texture(gpu, size, wgpu::TextureUsages::RENDER_ATTACHMENT)
}

/// A `FORMAT` texture of `size` pixels with `usage`, readable back.
pub(crate) fn target_texture(
    gpu: &Gpu,
    size: [u32; 2],
    usage: wgpu::TextureUsages,
) -> Result<wgpu::Texture, RenderError> {
    let max = gpu.device.limits().max_texture_dimension_2d;
    if size.contains(&0) {
        return Err(RenderError::Invalid("offscreen target has a zero side"));
    }
    if size.iter().any(|side| *side > max) {
        return Err(RenderError::Invalid(
            "offscreen target exceeds the device's texture limit",
        ));
    }
    Ok(gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: usage | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }))
}

/// Copies `texture` into memory, waiting for the GPU. Strips the 256-byte row padding the copy
/// requires, so the result is tightly packed.
pub(crate) fn read_rgba(gpu: &Gpu, texture: &wgpu::Texture) -> Result<RgbaImage, RenderError> {
    let [width, height] = [texture.width(), texture.height()];
    let row = width * 4;
    let padded_row = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    map_and_wait(gpu, &buffer)?;
    let view = buffer.get_mapped_range(..).map_err(device)?;
    let mut pixels = Vec::with_capacity(row as usize * height as usize);
    for padded in view.chunks_exact(padded_row as usize) {
        pixels.extend_from_slice(&padded[..row as usize]);
    }
    drop(view);
    buffer.unmap();
    Ok(RgbaImage::new([width, height], pixels))
}

/// Copies the first `bytes` of a `COPY_SRC` buffer into memory, waiting for the GPU.
pub(crate) fn read_buffer(
    gpu: &Gpu,
    source: &wgpu::Buffer,
    bytes: u64,
) -> Result<Vec<u8>, RenderError> {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, bytes);
    gpu.queue.submit([encoder.finish()]);
    map_and_wait(gpu, &buffer)?;
    let view = buffer.get_mapped_range(..).map_err(device)?;
    let data = view.to_vec();
    drop(view);
    buffer.unmap();
    Ok(data)
}

fn map_and_wait(gpu: &Gpu, buffer: &wgpu::Buffer) -> Result<(), RenderError> {
    let (sender, receiver) = mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        // The receiver outlives the wait below; a failed send cannot happen.
        let _ = sender.send(result);
    });
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(device)?;
    match receiver.recv() {
        Ok(result) => result.map_err(device),
        Err(_) => Err(RenderError::Device(
            "buffer map callback never ran".to_string(),
        )),
    }
}
