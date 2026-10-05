//! `Surface`: a platform window the renderer presents into. The host owns the window's
//! lifecycle; the renderer only configures, acquires and presents.

use crate::core::Core;
use crate::error::{RenderError, device};

/// A platform window the renderer can present into. Hosts implement `raw-window-handle`'s
/// `HasWindowHandle` and `HasDisplayHandle` for their window type (on Android the display is
/// the handle-less `DisplayHandle::android()`). The instance is created before any window, so
/// wgpu takes the display from the window: without one every surface fails to create.
pub trait SurfaceWindow: wgpu::DisplayAndWindowHandle + 'static {}

impl<T: wgpu::DisplayAndWindowHandle + 'static> SurfaceWindow for T {}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// Every format the window offered with its colour spaces, as the adapter reported them.
    offered: Vec<wgpu::SurfaceFormatCapabilities>,
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
        let target = wgpu::SurfaceTarget::from(window);
        let surface = self.gpu.instance.create_surface(target).map_err(device)?;
        let caps = surface.get_capabilities(&self.gpu.adapter);
        let format = preferred_format(&caps.format_capabilities).ok_or(RenderError::Invalid(
            "the window cannot be presented in sRGB by this GPU adapter",
        ))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            // The artwork and the UI are sRGB-encoded (D50, S27): declared as such, so a
            // wide-gamut display's compositor maps them instead of stretching them to its gamut.
            color_space: wgpu::SurfaceColorSpace::Srgb,
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
            offered: caps.format_capabilities,
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

    /// The format and colour space presented in, then what the window offered, for the log:
    /// `Rgba8Unorm in SRGB; offered Rgba8Unorm: SRGB | DISPLAY_P3, ...`.
    pub fn report(&self) -> String {
        let offered: Vec<String> = self
            .offered
            .iter()
            .map(|offer| format!("{:?}: {:?}", offer.format, offer.color_spaces))
            .collect();
        format!(
            "{:?} in {:?}; offered {}",
            self.config.format,
            self.config.color_space,
            offered.join(", ")
        )
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

/// The format the window presents in the sRGB colour space: ten bits per channel where it
/// offers them (S37: the canvas's ten bits reach the display), else the first non-sRGB one (the
/// UI painter blends in gamma space, so such a format shows its colours unchanged), else the
/// first. On the tablet the compositor keeps a ten-bit layer as it is (STATUS).
fn preferred_format(offered: &[wgpu::SurfaceFormatCapabilities]) -> Option<wgpu::TextureFormat> {
    let formats: Vec<wgpu::TextureFormat> = offered
        .iter()
        .filter(|offer| offer.color_spaces.contains(wgpu::SurfaceColorSpaces::SRGB))
        .map(|offer| offer.format)
        .collect();
    let first = |wanted: fn(&wgpu::TextureFormat) -> bool| formats.iter().copied().find(wanted);
    first(|format| *format == crate::gpu::TEN_BITS)
        .or(first(|format| !format.is_srgb()))
        .or(formats.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_formats_presented_in_srgb_are_chosen() {
        use wgpu::{
            SurfaceColorSpaces as Spaces, SurfaceFormatCapabilities as Offer, TextureFormat,
        };
        let offer = |format, color_spaces| Offer {
            format,
            color_spaces,
        };
        let offered = [
            offer(TextureFormat::Rgba16Float, Spaces::EXTENDED_SRGB_LINEAR),
            offer(TextureFormat::Rgba8UnormSrgb, Spaces::SRGB),
            offer(TextureFormat::Rgb10a2Unorm, Spaces::DISPLAY_P3),
            offer(TextureFormat::Rgba8Unorm, Spaces::SRGB | Spaces::DISPLAY_P3),
        ];
        assert_eq!(preferred_format(&offered), Some(TextureFormat::Rgba8Unorm));
        assert_eq!(
            preferred_format(&offered[..3]),
            Some(TextureFormat::Rgba8UnormSrgb)
        );
        assert_eq!(preferred_format(&offered[..1]), None);
        let ten = [
            offered[3],
            offer(
                TextureFormat::Rgb10a2Unorm,
                Spaces::SRGB | Spaces::BT2100_PQ,
            ),
        ];
        assert_eq!(preferred_format(&ten), Some(TextureFormat::Rgb10a2Unorm));
    }
}
