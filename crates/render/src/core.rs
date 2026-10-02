//! `Core`: the renderer's GPU state. Created once per process and shared; never stored in the
//! UI framework.

use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::image::RgbaImage;
use crate::readback::{offscreen_texture, read_rgba};

pub struct Core {
    gpu: Gpu,
}

impl Core {
    /// A core without a window: offscreen rendering and readback only.
    pub fn headless() -> Result<Self, RenderError> {
        Ok(Self {
            gpu: Gpu::headless()?,
        })
    }

    /// Adapter name and backend, for logs and harness output.
    pub fn adapter(&self) -> String {
        self.gpu.describe()
    }

    /// Clears an offscreen target of `size` pixels to the 8-bit RGBA `color` and reads it back.
    pub fn render_clear(&self, size: [u32; 2], color: [u8; 4]) -> Result<RgbaImage, RenderError> {
        let texture = offscreen_texture(&self.gpu, size)?;
        let view = texture.create_view(&Default::default());
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(unorm_color(color)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        drop(pass);
        self.gpu.queue.submit([encoder.finish()]);
        read_rgba(&self.gpu, &texture)
    }
}

/// `k / 255` converts back to exactly `k` under the round-to-nearest unorm rule.
fn unorm_color(color: [u8; 4]) -> wgpu::Color {
    let [r, g, b, a] = color.map(|channel| f64::from(channel) / 255.0);
    wgpu::Color { r, g, b, a }
}
