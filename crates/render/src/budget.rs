//! The GPU memory budget (S49, S58, DECISIONS D85): the bytes the open canvas's derived caches
//! may hold on a device. On a GPU that shares the system's memory (the tablet's Adreno) it is a
//! quarter of the device's memory (about 3 GB of the tablet's 12: the app is the one the author
//! paints in, in front, and a cache made again costs seconds); on a GPU with memory of its own
//! (the development machine) a fixed 2 GiB. Above the budget the runtime lets caches go by
//! recency, never what the frame shows or the stroke being drawn uses (`runtime/src/budget.rs`);
//! it never fails for it.

use crate::core::Core;

/// The share of the device's memory the derived caches may hold on a GPU sharing it.
pub const SHARED_PART: u64 = 4;
/// The budget on a GPU sharing the system's memory when the host did not say how much the device
/// has (the harness and tests on such a GPU): S49's, measured on the tablet.
pub const SHARED_BUDGET: u64 = 384 << 20;
/// The budget on a GPU with memory of its own.
pub const OWN_BUDGET: u64 = 2 << 30;

/// What the open canvas's derived caches hold on the GPU, in bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheBytes {
    /// The canvas target itself: its texture, the in-progress stroke's two textures, the
    /// evaluator's lists and the scene's ops. Never dropped.
    pub canvas: u64,
    /// The view tiers (S32).
    pub tiers: u64,
    /// The in-progress stroke's and a preview's snapshots (S35, S48).
    pub snapshots: u64,
    /// The smudges' residuals and the filters' results (the arena), their material and kernels.
    pub arena: u64,
    /// The layer pictures being made (a canvas-sized target on their worker) and the brush
    /// strip's.
    pub pictures: u64,
}

impl CacheBytes {
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.canvas + self.tiers + self.snapshots + self.arena + self.pictures
    }
}

/// Whether a GPU shares the system's memory or has its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Shared,
    Own,
}

/// What the device's memory allocator holds (S51, the harness's long session): the bytes its
/// live allocations take, the bytes of the memory blocks it took from the driver to hold them,
/// and the allocations' bytes per label (`"smudge residual"`, `"snapshot"`), largest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GpuMemory {
    pub allocated: u64,
    pub reserved: u64,
    pub by_label: Vec<(String, u64)>,
}

impl Core {
    /// What the device's allocator holds now; `None` on a backend that does not report it.
    #[must_use]
    pub fn gpu_memory(&self) -> Option<GpuMemory> {
        let report = self.gpu.device.generate_allocator_report()?;
        let mut by_label: std::collections::HashMap<String, u64> = Default::default();
        for allocation in report.allocations {
            *by_label.entry(allocation.name).or_default() += allocation.size;
        }
        let mut by_label: Vec<(String, u64)> = by_label.into_iter().collect();
        by_label.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Some(GpuMemory {
            allocated: report.total_allocated_bytes,
            reserved: report.total_reserved_bytes,
            by_label,
        })
    }

    /// The class of this device: its own memory for a discrete GPU, else shared.
    #[must_use]
    pub fn device_class(&self) -> DeviceClass {
        match self.gpu.adapter.get_info().device_type {
            wgpu::DeviceType::DiscreteGpu => DeviceClass::Own,
            _ => DeviceClass::Shared,
        }
    }

    /// The GPU memory budget of this device: a quarter of `memory` (the device's, in bytes, as
    /// its host says) on a GPU sharing it, `SHARED_BUDGET` there without it; `OWN_BUDGET` on a
    /// GPU with memory of its own.
    #[must_use]
    pub fn memory_budget(&self, memory: Option<u64>) -> u64 {
        match (self.device_class(), memory) {
            (DeviceClass::Shared, Some(memory)) => memory / SHARED_PART,
            (DeviceClass::Shared, None) => SHARED_BUDGET,
            (DeviceClass::Own, _) => OWN_BUDGET,
        }
    }
}
