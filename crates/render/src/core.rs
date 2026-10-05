//! `Core`: the renderer's GPU state. Created once per process and shared; never stored in the
//! UI framework.

use crate::error::RenderError;
use crate::filter::FilterPipeline;
use crate::gpu::Gpu;
use crate::image::RgbaImage;
use crate::program::EvaluatePipeline;
use crate::readback::{offscreen_texture, read_rgba, wait};
use crate::screen::{blit_layout, blit_sampler};
use crate::smudge::SmudgePipeline;
use crate::thumbnail::ThumbnailPipeline;
use crate::view_shown::TierBindings;

pub struct Core {
    pub(crate) gpu: Gpu,
    /// The region evaluator's compute pipeline (`RegionEvaluator`).
    pub(crate) evaluate: EvaluatePipeline,
    /// How a canvas target binds for showing on screen, the same for every frame format.
    pub(crate) blit_layout: wgpu::BindGroupLayout,
    /// How the screen samples a canvas target: bilinear.
    pub(crate) blit_sampler: wgpu::Sampler,
    /// Reduces a canvas target to a gallery thumbnail (`Core::thumbnail`).
    pub(crate) thumbnail: ThumbnailPipeline,
    /// Filters a layer's pixels into a block of an arena (`filter`).
    pub(crate) filter: FilterPipeline,
    /// Applies smudge contacts to residuals (`smudge`).
    pub(crate) smudge: SmudgePipeline,
    /// How the blit binds the view tiers (S32), and its group without any.
    pub(crate) tiers: TierBindings,
}

/// The one device the crate's own tests share (one per test binary: drivers deadlock when
/// several open at once, STATUS).
#[cfg(test)]
pub(crate) fn test_core() -> &'static Core {
    static CORE: std::sync::OnceLock<Core> = std::sync::OnceLock::new();
    CORE.get_or_init(|| Core::headless().expect("a GPU adapter is required for render tests"))
}

impl Core {
    /// Opens the GPU device without a window and builds the pipelines. Offscreen rendering and
    /// readback work at once; platform windows attach later through `Core::create_surface`.
    pub fn headless() -> Result<Self, RenderError> {
        let gpu = Gpu::new()?;
        let evaluate = EvaluatePipeline::new(&gpu.device);
        let blit_layout = blit_layout(&gpu.device);
        let blit_sampler = blit_sampler(&gpu.device);
        let thumbnail = ThumbnailPipeline::new(&gpu.device);
        let smudge = SmudgePipeline::new(&gpu.device);
        let filter = FilterPipeline::new(&gpu.device);
        let tiers = TierBindings::new(&gpu.device);
        Ok(Self {
            gpu,
            evaluate,
            blit_layout,
            blit_sampler,
            thumbnail,
            smudge,
            filter,
            tiers,
        })
    }

    /// Adapter name and backend, for logs and harness output.
    pub fn adapter(&self) -> String {
        self.gpu.describe()
    }

    /// Blocks until the GPU has finished all submitted work, for timing a whole frame; at most
    /// `GPU_WAIT`, then `RenderError::Timeout`.
    pub fn wait_idle(&self) -> Result<(), RenderError> {
        wait(&self.gpu, "the submitted frames")
    }

    /// Clears an offscreen target of `size` pixels to the 8-bit RGBA `color` and reads it back.
    pub fn render_clear(&self, size: [u32; 2], color: [u8; 4]) -> Result<RgbaImage, RenderError> {
        let texture = offscreen_texture(&self.gpu, size)?;
        let view = texture.create_view(&Default::default());
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(clear_attachment(&view, color))],
            ..Default::default()
        });
        drop(pass);
        self.gpu.queue.submit([encoder.finish()]);
        read_rgba(&self.gpu, &texture)
    }
}

/// A colour attachment over `view` that starts by clearing it to the 8-bit RGBA `color`.
pub(crate) fn clear_attachment(
    view: &wgpu::TextureView,
    color: [u8; 4],
) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(unorm_color(color)),
            store: wgpu::StoreOp::Store,
        },
    }
}

/// `k / 255` converts back to exactly `k` under the round-to-nearest unorm rule.
fn unorm_color(color: [u8; 4]) -> wgpu::Color {
    let [r, g, b, a] = color.map(|channel| f64::from(channel) / 255.0);
    wgpu::Color { r, g, b, a }
}
