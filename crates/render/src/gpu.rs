//! The wgpu device and queue. They belong to the renderer's `Core`, never to the UI framework
//! (DECISIONS D01).

use crate::error::{RenderError, device};

pub(crate) struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
}

impl Gpu {
    /// Opens a device without a window or surface: offscreen targets and readback only.
    /// `WGPU_BACKEND` and the other wgpu environment variables select the adapter.
    pub(crate) fn headless() -> Result<Self, RenderError> {
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
            device,
            queue,
            info: adapter.get_info(),
        })
    }

    /// Adapter name and backend, for logs and harness output.
    pub(crate) fn describe(&self) -> String {
        format!("{} ({})", self.info.name, self.info.backend)
    }
}
