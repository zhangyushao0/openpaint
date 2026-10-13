//! The wgpu instance, adapter, device and queue. They belong to the renderer's `Core`, never to
//! the UI framework (DECISIONS D01). The device reports its loss to `Health` (S49): the driver's
//! lost callback, a reset, a test destroying it.

use std::sync::{Arc, Mutex, PoisonError};

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
    /// Whether ten bits were asked for, so a device opened again after a loss is alike.
    pub(crate) ten_bits: bool,
    pub(crate) health: Arc<Health>,
}

/// Whether the device is lost, and why: set once, by wgpu's lost callback or by whoever finds it
/// lost first.
#[derive(Debug, Default)]
pub(crate) struct Health {
    lost: Mutex<Option<String>>,
}

impl Health {
    /// The device is lost for `why`; the first reason stays.
    pub(crate) fn lose(&self, why: String) {
        let mut lost = self.lost.lock().unwrap_or_else(PoisonError::into_inner);
        if lost.is_none() {
            log::error!("the GPU device was lost: {why}");
            *lost = Some(why);
        }
    }

    /// Why the device was lost, once it was.
    pub(crate) fn lost(&self) -> Option<String> {
        self.lost
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
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
    /// wgpu environment variables select the adapter. Without `ten_bits` the display format is
    /// `readback::FORMAT` whatever the adapter offers.
    pub(crate) fn new(ten_bits: bool) -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(device)?;
        // Canvas textures need the adapter's real maximum sizes, not the portable minimums; the
        // ten-bit display format needs the adapter's own format features (S37).
        // S51: on a GPU sharing the system's memory (the tablet) the allocator takes blocks of 8
        // to 64 MiB instead of 128 to 256, so what it holds stays near what is allocated.
        let shared = adapter.get_info().device_type != wgpu::DeviceType::DiscreteGpu;
        // S53: GPU timestamps measure background work where the adapter has them.
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("openpaint"),
            required_features: adapter.features()
                & (wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::TIMESTAMP_QUERY),
            required_limits: adapter.limits(),
            memory_hints: if shared {
                wgpu::MemoryHints::MemoryUsage
            } else {
                wgpu::MemoryHints::Performance
            },
            ..Default::default()
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&descriptor)).map_err(device)?;
        let display = if ten_bits
            && device
                .features()
                .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        {
            display_format(&adapter)
        } else {
            crate::readback::FORMAT
        };
        let health = Arc::new(Health::default());
        let reported = Arc::clone(&health);
        device.set_device_lost_callback(move |reason, message| {
            reported.lose(format!("{reason:?} {message}").trim_end().to_owned());
        });
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            display,
            ten_bits,
            health,
        })
    }

    /// Adapter name and backend, for logs and harness output.
    pub(crate) fn describe(&self) -> String {
        let info = self.adapter.get_info();
        let timestamps = self
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY);
        format!(
            "{} ({}), canvas shown in {:?}, GPU timestamps {}",
            info.name,
            info.backend,
            self.display,
            if timestamps { "measured" } else { "absent" }
        )
    }
}
