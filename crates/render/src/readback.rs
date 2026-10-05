//! Offscreen targets and GPU-to-memory readback.

use std::sync::mpsc;
use std::time::Duration;

use crate::error::{RenderError, device};
use crate::gpu::{Gpu, TEN_BITS};
use crate::image::RgbaImage;

/// The offscreen colour format: 8 bits per channel, no sRGB conversion on write or read.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A texture that can be rendered into and then read back.
pub(crate) fn offscreen_texture(gpu: &Gpu, size: [u32; 2]) -> Result<wgpu::Texture, RenderError> {
    target_texture(gpu, (size, FORMAT), wgpu::TextureUsages::RENDER_ATTACHMENT)
}

/// A texture of `size` pixels in `format` (`FORMAT` or the display's, `Gpu::display`) with
/// `usage`, readable back.
pub(crate) fn target_texture(
    gpu: &Gpu,
    (size, format): ([u32; 2], wgpu::TextureFormat),
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
        format,
        usage: usage | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }))
}

/// Copies `texture` into memory, waiting for the GPU. Strips the 256-byte row padding the copy
/// requires, so the result is tightly packed.
pub(crate) fn read_rgba(gpu: &Gpu, texture: &wgpu::Texture) -> Result<RgbaImage, RenderError> {
    read_rect(gpu, texture, [0, 0], [texture.width(), texture.height()])
}

/// As `read_rgba`, the `[width, height]` texels from `[x, y]` on (inside the texture).
pub(crate) fn read_rect(
    gpu: &Gpu,
    texture: &wgpu::Texture,
    [x, y]: [u32; 2],
    [width, height]: [u32; 2],
) -> Result<RgbaImage, RenderError> {
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
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
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
    if texture.format() == TEN_BITS {
        for texel in pixels.as_chunks_mut::<4>().0 {
            *texel = eight_bits(u32::from_le_bytes(*texel));
        }
    }
    Ok(RgbaImage::new([width, height], pixels))
}

/// Copies the RGBA texel at `[x, y]` of `texture` into memory, waiting for the GPU; `None` when
/// the texel lies outside the texture.
pub(crate) fn read_texel(
    gpu: &Gpu,
    texture: &wgpu::Texture,
    [x, y]: [u32; 2],
) -> Result<Option<[u8; 4]>, RenderError> {
    if x >= texture.width() || y >= texture.height() {
        return Ok(None);
    }
    let row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texel readback"),
        size: u64::from(row),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    map_and_wait(gpu, &buffer)?;
    let view = buffer.get_mapped_range(..).map_err(device)?;
    let mut texel = [0; 4];
    texel.copy_from_slice(&view[..4]);
    drop(view);
    buffer.unmap();
    if texture.format() == TEN_BITS {
        texel = eight_bits(u32::from_le_bytes(texel));
    }
    Ok(Some(texel))
}

/// The 8-bit RGBA a ten-bit display texel (`Rgb10a2Unorm`: red in the low ten bits) stands for.
/// The evaluator writes each channel as a ten-bit code `j` chosen among those whose nearest
/// 8-bit value, `floor(j 255 / 1023 + 1/2)`, is the one it stores in an 8-bit target (S37,
/// `coded` in `evaluate.wgsl`), so this is that value exactly; never halfway, as `510 j` is even
/// and `1023` odd. Alpha has two bits: display targets are opaque, 3 is 255.
pub(crate) fn eight_bits(word: u32) -> [u8; 4] {
    let channel = |shift: u32| ((((word >> shift) & 1023) * 510 + 1023) / 2046) as u8;
    [
        channel(0),
        channel(10),
        channel(20),
        ((word >> 30) * 85) as u8,
    ]
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

/// The longest any wait for the GPU may take. Work here finishes in milliseconds; a wait this
/// long is a stalled driver or device, reported as `RenderError::Timeout` instead of blocking
/// the caller (a test, a worker) forever.
pub(crate) const GPU_WAIT: Duration = Duration::from_secs(30);

/// Blocks until the GPU has finished every submitted command, at most `GPU_WAIT`; `what` names
/// the wait in the error.
pub(crate) fn wait(gpu: &Gpu, what: &'static str) -> Result<(), RenderError> {
    let poll = wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(GPU_WAIT),
    };
    match gpu.device.poll(poll) {
        Ok(_) => Ok(()),
        Err(wgpu::PollError::Timeout) => Err(timeout(what)),
        Err(error) => Err(device(error)),
    }
}

fn timeout(what: &'static str) -> RenderError {
    RenderError::Timeout {
        what,
        seconds: GPU_WAIT.as_secs(),
    }
}

fn map_and_wait(gpu: &Gpu, buffer: &wgpu::Buffer) -> Result<(), RenderError> {
    let (sender, receiver) = mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        // The receiver outlives the wait below; a failed send cannot happen.
        let _ = sender.send(result);
    });
    wait(gpu, "a readback")?;
    // The finished wait ran the callback; the timeout only guards a driver that never does.
    match receiver.recv_timeout(GPU_WAIT) {
        Ok(result) => result.map_err(device),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(timeout("a readback's buffer map")),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(RenderError::Device(
            "buffer map callback never ran".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `coded`'s range of ten-bit codes for the 8-bit value `k` (`evaluate.wgsl`).
    fn codes(k: u32) -> std::ops::RangeInclusive<u32> {
        let low = if k == 0 {
            0
        } else {
            ((2 * k - 1) * 1023).div_ceil(510)
        };
        low..=(((2 * k + 1) * 1023) / 510).min(1023)
    }

    #[test]
    fn ten_bit_codes_read_back_as_their_eight_bit_value() {
        let mut next = 0;
        for k in 0..=255 {
            let codes = codes(k);
            // Every code belongs to exactly one value, in order, four or five each (three at
            // the ends, which hold half a step).
            assert_eq!(*codes.start(), next);
            assert!((3..=5).contains(&codes.clone().count()), "{k}: {codes:?}");
            next = codes.end() + 1;
            for j in codes {
                let word = j | (j << 10) | (j << 20) | (3 << 30);
                let expected = u8::try_from(k).expect("8 bits");
                assert_eq!(eight_bits(word), [expected, expected, expected, 255]);
            }
        }
        assert_eq!(next, 1024);
        // Each code's value is the nearest 8-bit value to it.
        for j in 0..1024u32 {
            let nearest = (f64::from(j) * 255.0 / 1023.0).round() as u8;
            assert_eq!(eight_bits(j)[0], nearest);
        }
    }
}
