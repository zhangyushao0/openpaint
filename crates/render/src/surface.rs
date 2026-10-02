//! `Surface`: a platform window the renderer presents into. The host owns the window's
//! lifecycle; the renderer only configures, acquires and presents.

use crate::core::Core;
use crate::error::{RenderError, device};

/// A platform window the renderer can present into. Hosts implement `raw-window-handle`'s
/// `HasWindowHandle` for their window type; no display handle is needed on Android.
pub trait SurfaceWindow: wgpu::WindowHandle + 'static {}

impl<T: wgpu::WindowHandle + 'static> SurfaceWindow for T {}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// False while either side is zero; such a surface skips frames.
    configured: bool,
}

impl Core {
    /// Attaches `window` as a presentable surface of `size` pixels. The surface keeps the window
    /// alive until it is dropped; drop it before the platform destroys the window.
    pub fn create_surface(
        &self,
        window: impl SurfaceWindow,
        size: [u32; 2],
    ) -> Result<Surface, RenderError> {
        let target = wgpu::SurfaceTarget::from_window_without_display(window);
        let surface = self.gpu.instance.create_surface(target).map_err(device)?;
        let caps = surface.get_capabilities(&self.gpu.adapter);
        let format = preferred_format(&caps.formats).ok_or(RenderError::Invalid(
            "the window cannot be presented by this GPU adapter",
        ))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size[0],
            height: size[1],
            desired_maximum_frame_latency: 2,
            // Fifo is the only mode every platform guarantees.
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        let mut surface = Surface {
            surface,
            config,
            configured: false,
        };
        surface.resize(self, size);
        Ok(surface)
    }
}

impl Surface {
    /// `[width, height]` in pixels.
    pub fn size(&self) -> [u32; 2] {
        [self.config.width, self.config.height]
    }

    pub(crate) fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Reconfigures for a new window size, after rotation or a split-screen change.
    pub fn resize(&mut self, core: &Core, size: [u32; 2]) {
        self.config.width = size[0];
        self.config.height = size[1];
        self.configured = !size.contains(&0);
        if self.configured {
            self.surface.configure(&core.gpu.device, &self.config);
        }
    }

    /// The next texture to draw into, or `None` when this frame should be skipped (zero size,
    /// timeout, occluded window). An outdated surface is reconfigured and tried once more.
    pub(crate) fn acquire(
        &mut self,
        core: &Core,
    ) -> Result<Option<wgpu::SurfaceTexture>, RenderError> {
        if !self.configured {
            return Ok(None);
        }
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&core.gpu.device, &self.config);
                acquired(self.surface.get_current_texture())
            }
            other => acquired(other),
        }
    }
}

fn acquired(
    result: wgpu::CurrentSurfaceTexture,
) -> Result<Option<wgpu::SurfaceTexture>, RenderError> {
    use wgpu::CurrentSurfaceTexture as Current;
    match result {
        Current::Success(texture) | Current::Suboptimal(texture) => Ok(Some(texture)),
        Current::Timeout | Current::Occluded | Current::Outdated => Ok(None),
        Current::Lost => Err(RenderError::Device(
            "the window surface was lost".to_string(),
        )),
        Current::Validation => Err(RenderError::Device(
            "acquiring the window surface failed validation".to_string(),
        )),
    }
}

/// The UI painter blends in gamma space, so a non-sRGB format shows its colours unchanged.
fn preferred_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .find(|format| !format.is_srgb())
        .or(formats.first())
        .copied()
}
