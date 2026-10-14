//! `Core`: the renderer's GPU state. Created once per process and shared; never stored in the
//! UI framework. When its device is lost, the runtime replaces it whole (`reopened`, S49).

use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use crate::error::RenderError;
use crate::filter::{FILTER_SHADER, FilterPipeline};
use crate::gpu::Gpu;
use crate::gpu_frames::GpuFrames;
use crate::image::RgbaImage;
use crate::pace::{Background, Pace};
use crate::program::{EvaluatePipeline, shader_source};
use crate::raster_gpu::RasterPipelines;
use crate::readback::{offscreen_texture, read_rgba, wait};
use crate::screen::{blit_layout, blit_sampler};
use crate::smudge::{SMUDGE_SHADER, SmudgePipeline};
use crate::tally::Counts;
use crate::thumbnail::ThumbnailPipeline;
use crate::underlay::UnderlayPipeline;
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
    /// The screen's frame budget for background work (S53).
    pub(crate) pace: Mutex<Pace>,
    /// Enlarges the gallery picture into a canvas target opening without a display cache
    /// (S54), made on first use.
    pub(crate) underlay: OnceLock<UnderlayPipeline>,
    /// The raster core probe's pipelines (S57, the benchmark only), made on first use.
    pub(crate) raster: OnceLock<RasterPipelines>,
    /// What the pixels it evaluates depend on, hashed (`renderer_identity`).
    identity: u64,
    /// The screen's frames the GPU has not finished (S56).
    pub(crate) frames: GpuFrames,
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
        Self::with_gpu(Gpu::new(true)?)
    }

    /// As `headless`, the screen's canvas and view tiers in 8 bits per channel even where the
    /// device has ten (S37's fallback): what the scenario runner uses, whose checks read 8-bit
    /// values and whose ten-bit reads decoded every texel on the CPU (S38).
    pub fn headless_eight_bits() -> Result<Self, RenderError> {
        Self::with_gpu(Gpu::new(false)?)
    }

    fn with_gpu(gpu: Gpu) -> Result<Self, RenderError> {
        let evaluate = EvaluatePipeline::new(&gpu.device, gpu.display);
        let blit_layout = blit_layout(&gpu.device);
        let blit_sampler = blit_sampler(&gpu.device);
        let thumbnail = ThumbnailPipeline::new(&gpu.device);
        let smudge = SmudgePipeline::new(&gpu.device);
        let filter = FilterPipeline::new(&gpu.device);
        let tiers = TierBindings::new(&gpu.device);
        let pace = Mutex::new(Pace::new(&gpu));
        Ok(Self {
            pace,
            underlay: OnceLock::new(),
            raster: OnceLock::new(),
            identity: identity(gpu.display),
            frames: GpuFrames::default(),
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

    /// The renderer's identity (S54): a hash of the build's version (a patch per slice) and of
    /// the evaluator's, the smudge kernels' and the filters' WGSL as this device compiles them,
    /// so a build whose evaluated pixels may differ never takes a display cache another wrote
    /// as complete.
    #[must_use]
    pub const fn renderer_identity(&self) -> u64 {
        self.identity
    }

    /// A new device on the same kind of adapter, with every pipeline built anew: what replaces
    /// this one once it is lost (S49). Nothing made on this one can be used with it.
    ///
    /// # Errors
    /// As `headless`: no adapter or device now (the driver may still be resetting).
    pub fn reopened(&self) -> Result<Self, RenderError> {
        Self::with_gpu(Gpu::new(self.gpu.ten_bits)?)
    }

    /// Why the device was lost (the driver's lost callback, an error once it was lost, out of
    /// memory), once it was; `None` while it works.
    #[must_use]
    pub fn lost(&self) -> Option<String> {
        self.gpu.health.lost()
    }

    /// From now on wgpu's errors on this device no longer end the process while it is lost:
    /// once the driver reported the loss, every call on the device fails and each failure is
    /// only logged; running out of GPU memory or an internal driver failure counts as a loss. A
    /// validation error on a working device is a bug and panics, as wgpu's own handler does.
    /// The runtime calls this; tests keep wgpu's handler.
    pub fn catch_errors(&self) {
        let health = Arc::clone(&self.gpu.health);
        let handler = move |error: wgpu::Error| match error {
            wgpu::Error::OutOfMemory { .. } | wgpu::Error::Internal { .. } => {
                health.lose(error.to_string());
            }
            wgpu::Error::Validation { .. } if health.lost().is_some() => {
                log::debug!("the lost device refused a call: {error}");
            }
            wgpu::Error::Validation { .. } => panic!("wgpu error: {error}"),
        };
        self.gpu.device.on_uncaptured_error(Arc::new(handler));
    }

    /// Destroys the device as a driver reset would lose it, for tests and the scenario runner,
    /// then waits (at most `GPU_WAIT`) until wgpu reports the loss through its callback. Returns
    /// whether the callback reported it; if not, the loss is marked here.
    pub fn destroy_device(&self) -> bool {
        self.gpu.device.destroy();
        let poll = wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(crate::readback::GPU_WAIT),
        };
        if let Err(error) = self.gpu.device.poll(poll) {
            log::info!("polling the destroyed device: {error}");
        }
        let reported = self.lost().is_some();
        if !reported {
            self.gpu.health.lose("destroyed".to_owned());
        }
        reported
    }

    /// Sets the most work one evaluation dispatch takes (`bounded.rs`: op entries listed in its
    /// tiles, one more a tile; 12,000 at first, about 100 ms on the tablet), so tests can compare
    /// a bounded evaluation with one dispatch.
    pub fn set_dispatch_work(&self, entries: u64) {
        self.evaluate.bound_to(entries);
    }

    /// Evaluation dispatches submitted on this device so far (S49).
    #[must_use]
    pub fn dispatches(&self) -> u64 {
        self.evaluate.runs()
    }

    /// What evaluations on this device did so far, counted (S53).
    #[must_use]
    pub fn counts(&self) -> Counts {
        self.evaluate.tally.counts(self.dispatches())
    }

    /// A frame of the screen begins (S53): the background work's budget is whole again, and the
    /// GPU's measured times since correct its estimates. The runtime calls this once a frame.
    pub fn begin_frame(&self) {
        self.pace().begin(&self.gpu);
    }

    /// As `begin_frame`, giving background work `IDLE_BACKGROUND_NS` of GPU time a frame from
    /// this one on while nothing touches the screen (`idle`), else `BACKGROUND_NS` (S54), less
    /// what earlier frames spent beyond theirs; both are the one `set_frame_budget` gave.
    pub fn begin_frame_idle(&self, idle: bool) {
        let mut pace = self.pace();
        pace.aim(idle);
        pace.begin(&self.gpu);
    }

    /// What the frame before took for background work (S53).
    #[must_use]
    pub fn background(&self) -> Background {
        self.pace().last()
    }

    /// The GPU time a frame gives background work from now on, in ns, touched or idle
    /// (`BACKGROUND_NS` and `IDLE_BACKGROUND_NS` at first), so tests can make a frame's share
    /// small.
    pub fn set_frame_budget(&self, ns: f64) {
        self.pace().set_budget(ns);
    }

    pub(crate) fn pace(&self) -> MutexGuard<'_, Pace> {
        self.pace.lock().unwrap_or_else(PoisonError::into_inner)
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
        self.gpu.submit([encoder.finish()]);
        read_rgba(&self.gpu, &texture)
    }
}

/// FNV-1a over the build's version and the shaders evaluated pixels come from (S54): stable
/// from run to run of one build, unlike `std`'s hashers, which may change between releases.
fn identity(display: wgpu::TextureFormat) -> u64 {
    let sources = [
        env!("CARGO_PKG_VERSION"),
        &shader_source(display),
        SMUDGE_SHADER,
        FILTER_SHADER,
    ];
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in sources.iter().flat_map(|source| source.bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
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
