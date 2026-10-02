//! The wgpu instance, adapter, device and queue. They belong to the renderer's `Core`, never to
//! the UI framework (DECISIONS D01).

use crate::error::{RenderError, device};

pub(crate) struct Gpu {
    /// Kept so platform windows can be attached as surfaces after the device exists.
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl Gpu {
    /// Opens a device before any window exists: offscreen targets work at once, surfaces attach
    /// later. No display handle is needed on Android or for offscreen work. `WGPU_BACKEND` and
    /// the other wgpu environment variables select the adapter.
    pub(crate) fn new() -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(device)?;
        // Canvas textures need the adapter's real maximum sizes, not the portable minimums.
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("openpaint"),
            required_limits: adapter.limits(),
            ..Default::default()
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&descriptor)).map_err(device)?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Adapter name and backend, for logs and harness output.
    pub(crate) fn describe(&self) -> String {
        let info = self.adapter.get_info();
        format!("{} ({})", info.name, info.backend)
    }
}
