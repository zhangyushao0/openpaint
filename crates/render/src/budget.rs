//! The GPU memory budget (S49, DECISIONS D76): the bytes the open canvas's derived caches may
//! hold on a device, declared per device class. On a GPU that shares the system's memory (the
//! tablet's Adreno) what the app holds there counts against everything the system keeps, so the
//! budget is measured there; on a GPU with memory of its own (the development machine) it is
//! generous. Above the budget the runtime drops caches in a fixed order, cheapest to make again
//! first: the view tiers, then the snapshots, then the arena's blocks the scene does not show,
//! then the layer pictures; it never fails for it.

use crate::core::Core;

/// The budget on a GPU sharing the system's memory, from the tablet (2026-10-12, STATUS): with
/// an artwork open its Graphics memory was 109 to 153 MB and the app's PSS 350 to 365 MB; the
/// author's A4 artwork's caches held 179 MB at opening. This leaves that, a full view tier (90
/// MB) and the snapshot of a large painted area, and stops pathological growth well below the
/// system's memory.
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

    /// The GPU memory budget of this device's class.
    #[must_use]
    pub fn memory_budget(&self) -> u64 {
        match self.device_class() {
            DeviceClass::Shared => SHARED_BUDGET,
            DeviceClass::Own => OWN_BUDGET,
        }
    }
}
