//! Background work's GPU time, measured: each raster region drawn as background work writes GPU
//! timestamps, its first pass's at its start and its last pass's at its end, so the end less the
//! start is the time the GPU took for it. They are read back frames later without waiting
//! (`timestamps.rs`); the frame budget (`pace/`) learns from them how far its estimates are off
//! on this device. A device without timestamp queries measures nothing and the estimates stand.

use crate::gpu::Gpu;
use crate::timestamps::Timestamps;

/// Regions measured at once, at most; one more waits until a reading came back.
const SLOTS: u32 = 16;

pub(crate) struct GpuClock {
    stamps: Timestamps,
    /// Per slot, the estimate of the region it measures, ns (`pace/`).
    costs: Vec<f64>,
}

/// One region being measured: the slot its passes write to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing(u32);

/// A raster region's timestamps: the clock's query set and the slot, held apart from the clock
/// (the raster worker encodes the region while the frame loop uses the clock).
#[derive(Debug, Clone)]
pub(crate) struct Stamps {
    set: wgpu::QuerySet,
    pub(crate) timing: Timing,
}

impl Stamps {
    /// What a render pass writes: the start for the `first` pass, the end for the `last`.
    pub(crate) fn render_writes(
        &self,
        (first, last): (bool, bool),
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        (first || last).then_some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: first.then_some(2 * self.timing.0),
            end_of_pass_write_index: last.then_some(2 * self.timing.0 + 1),
        })
    }
}

impl GpuClock {
    /// A clock on `gpu`; `None` when its device has no timestamp queries.
    pub(crate) fn new(gpu: &Gpu) -> Option<Self> {
        let stamps = Timestamps::new(&gpu.device, &gpu.queue, SLOTS, "background timestamps")?;
        Some(Self {
            stamps,
            costs: vec![0.0; SLOTS as usize],
        })
    }

    /// A free slot for a region estimated at `cost` ns, and its timestamps; `None` while every
    /// slot waits.
    pub(crate) fn start(&mut self, cost: f64) -> Option<Stamps> {
        let at = self.stamps.claim()?;
        self.costs[at as usize] = cost;
        Some(Stamps {
            set: self.stamps.set.clone(),
            timing: Timing(at),
        })
    }

    /// A region started is not submitted after all: its slot is free again.
    pub(crate) fn abandon(&mut self, Timing(at): Timing) {
        self.stamps.free(at);
    }

    /// After the region's last pass: its two timestamps resolved and copied out, to be read back
    /// once the GPU is there.
    pub(crate) fn finish(&mut self, gpu: &Gpu, Timing(at): Timing) {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        self.stamps.resolve(&mut encoder, at);
        gpu.submit([encoder.finish()]);
        self.stamps.read_back(at);
    }

    /// Every reading back since the last call: each region's estimate and measured time, ns.
    pub(crate) fn readings(&mut self, gpu: &Gpu) -> Vec<(f64, f64)> {
        (self.stamps.readings(&gpu.device).into_iter())
            .map(|(at, measured)| (self.costs[at as usize], measured))
            .collect()
    }
}
