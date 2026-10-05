//! The wgpu instance, adapter, device and queue. They belong to the renderer's `Core`, never to
//! the UI framework (DECISIONS D01).

use crate::error::{RenderError, device};

pub(crate) struct Gpu {
    /// Kept so platform windows can be attached as surfaces after the device exists.
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    /// What the screen's canvas and its view tiers hold (S37): `Rgb10a2Unorm` where the device
    /// writes it from compute shaders, else `readback::FORMAT`.
    pub(crate) display: wgpu::TextureFormat,
}

/// The ten-bit display format.
pub(crate) const TEN_BITS: wgpu::TextureFormat = wgpu::TextureFormat::Rgb10a2Unorm;

/// The display format `adapter` can write from compute shaders and filter: ten bits, or eight.
fn display_format(adapter: &wgpu::Adapter) -> wgpu::TextureFormat {
    use wgpu::TextureFormatFeatureFlags as Flags;
    let features = adapter.get_texture_format_features(TEN_BITS);
    let writes = features
        .allowed_usages
        .contains(wgpu::TextureUsages::STORAGE_BINDING)
        && features
            .flags
            .contains(Flags::STORAGE_WRITE_ONLY | Flags::FILTERABLE);
    if writes {
        TEN_BITS
    } else {
        crate::readback::FORMAT
    }
}

impl Gpu {
    /// Opens a device before any window exists: offscreen targets work at once, surfaces attach
    /// later and bring their own display handle (`SurfaceWindow`). `WGPU_BACKEND` and the other
    /// wgpu environment variables select the adapter.
    pub(crate) fn new() -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(device)?;
        // Canvas textures need the adapter's real maximum sizes, not the portable minimums; the
        // ten-bit display format needs the adapter's own format features (S37).
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("openpaint"),
            required_features: adapter.features()
                & wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            required_limits: adapter.limits(),
            ..Default::default()
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&descriptor)).map_err(device)?;
        let display = if device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        {
            display_format(&adapter)
        } else {
            crate::readback::FORMAT
        };
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            display,
        })
    }

    /// Adapter name and backend, for logs and harness output.
    pub(crate) fn describe(&self) -> String {
        let info = self.adapter.get_info();
        format!(
            "{} ({}), canvas shown in {:?}",
            info.name, info.backend, self.display
        )
    }
}
