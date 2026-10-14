//! The display cache's side of the screen's canvas target (S54, DECISIONS D81): its texels copied
//! out exactly as it holds them, and a cached canvas or the gallery picture put in before
//! anything is evaluated into it, which the tiles still waiting then show instead of the paper.

use super::CanvasTarget;
use crate::core::Core;
use crate::error::{RenderError, device};
use crate::gpu::TEN_BITS;
use crate::image::RgbaImage;
use crate::readback::map_and_wait;

/// A copy of a canvas target's texels on its way to memory: submitted by
/// `CanvasTarget::copy_out` on the screen's thread, which never waits for it; read by `bytes` on
/// a worker, one GPU wait there.
pub struct CanvasCopy {
    buffer: wgpu::Buffer,
    size: [u32; 2],
    /// Bytes of a row in the buffer: the texels', padded as copies want them.
    padded: u32,
}

impl CanvasCopy {
    /// `[width, height]` in pixels.
    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Waits for the copy (and what was submitted before it) and returns the texels, 4 bytes
    /// each, rows top first, exactly as the target held them: 8-bit RGBA, or ten-bit words as
    /// the GPU packs them (red in the low ten bits, little-endian).
    ///
    /// # Errors
    /// The device's, or `Timeout` after 30 s.
    pub fn bytes(self, core: &Core) -> Result<Vec<u8>, RenderError> {
        map_and_wait(&core.gpu, &self.buffer)?;
        let view = self.buffer.get_mapped_range(..).map_err(device)?;
        let row = self.size[0] as usize * 4;
        let mut pixels = Vec::with_capacity(row * self.size[1] as usize);
        for padded in view.chunks_exact(self.padded as usize) {
            pixels.extend_from_slice(&padded[..row]);
        }
        drop(view);
        self.buffer.unmap();
        Ok(pixels)
    }
}

/// The display cache's code for a target's format: 10 for ten bits per channel, 8 for 8.
fn format_code(format: wgpu::TextureFormat) -> u32 {
    if format == TEN_BITS { 10 } else { 8 }
}

impl CanvasTarget {
    /// The display cache's code for its format: 10 for ten bits per channel, 8 for 8.
    #[must_use]
    pub fn format_code(&self) -> u32 {
        format_code(self.texture.format())
    }

    /// Copies its texels as they are once everything submitted so far ran, into memory a worker
    /// reads (`CanvasCopy::bytes`); nothing waits here.
    #[must_use]
    pub fn copy_out(&self, core: &Core) -> CanvasCopy {
        let [width, height] = self.size();
        let padded = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = core.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("canvas copy"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            self.texture.size(),
        );
        core.gpu.queue.submit([encoder.finish()]);
        CanvasCopy {
            buffer,
            size: [width, height],
            padded,
        }
    }

    /// Puts `pixels` (as `CanvasCopy::bytes` gave them from a target of this size and format)
    /// into it before anything was evaluated: what it shows from the first frame. A cache of
    /// exactly what is to be evaluated is the canvas `complete` (its caller then passes no
    /// regions, and the smudge residuals are evaluated while nothing waits, `warming`); any
    /// other shows under the tiles until they are evaluated, never the paper.
    ///
    /// # Errors
    /// `Invalid` when `pixels` do not fill the target.
    pub fn show_cached(
        &mut self,
        core: &Core,
        pixels: &[u8],
        complete: bool,
    ) -> Result<(), RenderError> {
        let [width, height] = self.size();
        if pixels.len() as u64 != u64::from(width) * u64::from(height) * 4 {
            return Err(RenderError::Invalid("cached pixels of another size"));
        }
        core.gpu.queue.write_texture(
            self.texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            self.texture.size(),
        );
        self.blank = false;
        self.warming = complete.then_some(0);
        Ok(())
    }

    /// Smudge residuals are still being evaluated after a canvas shown complete from its cache:
    /// frames are wanted until they are (S54).
    #[must_use]
    pub const fn warming(&self) -> bool {
        self.warming.is_some()
    }

    /// Puts `picture` (the gallery picture: its size and 8-bit RGBA pixels, rows top first)
    /// enlarged over the whole target before anything was evaluated into it: the tiles still
    /// waiting show it instead of the paper.
    ///
    /// # Errors
    /// `Invalid` unless it is a screen target (`new`) and the pixels fill the size; the device's.
    pub fn show_picture(
        &mut self,
        core: &Core,
        (size, pixels): ([u32; 2], &[u8]),
    ) -> Result<(), RenderError> {
        if pixels.len() as u64 != u64::from(size[0]) * u64::from(size[1]) * 4 {
            return Err(RenderError::Invalid("a picture of another size"));
        }
        core.enlarge_into(&RgbaImage::new(size, pixels.to_vec()), &self.texture)?;
        self.blank = false;
        Ok(())
    }
}

#[cfg(test)]
#[path = "evaluator_cache_tests.rs"]
mod tests;
