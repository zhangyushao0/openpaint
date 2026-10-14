//! Background work's GPU time, measured (S53): each background evaluation's passes write GPU
//! timestamps, the first run's at its start and every run's at its end, so the last end less the
//! first start is the time the GPU took for it. They are resolved into a buffer and read back
//! frames later without waiting; the frame budget (`pace.rs`) learns from them how far its
//! estimates are off on this device. A device without timestamp queries measures nothing and the
//! estimates stand.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::gpu::Gpu;
use crate::pace::Group;

/// Evaluations measured at once, at most; one more waits until a reading came back.
const SLOTS: u32 = 16;
/// Where each slot's two timestamps are resolved: the alignment resolving needs.
const STRIDE: u64 = 256;

/// A slot's state: free, its timestamps on their way back, or read back and waiting to be taken.
const FREE: u8 = 0;
const FLYING: u8 = 1;
const MAPPED: u8 = 2;

struct Slot {
    read: wgpu::Buffer,
    state: Arc<AtomicU8>,
    /// The tiles of the evaluation and their estimates (`pace.rs`).
    group: Group,
}

pub(crate) struct GpuClock {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    /// Nanoseconds a timestamp tick.
    period: f64,
}

/// One evaluation being measured: the slot its passes write to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing(u32);

impl GpuClock {
    /// A clock on `gpu`; `None` when its device has no timestamp queries.
    pub(crate) fn new(gpu: &Gpu) -> Option<Self> {
        if !gpu
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            return None;
        }
        let set = gpu.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("background timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2 * SLOTS,
        });
        let resolve = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("background timestamps"),
            size: STRIDE * u64::from(SLOTS),
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..SLOTS)
            .map(|_| Slot {
                read: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("background timestamps read"),
                    size: 16,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(FREE)),
                group: Group::default(),
            })
            .collect();
        Some(Self {
            set,
            resolve,
            slots,
            period: f64::from(gpu.queue.get_timestamp_period()),
        })
    }

    /// A free slot for an evaluation of `group`; `None` while every slot waits.
    pub(crate) fn start(&mut self, group: Group) -> Option<Timing> {
        let at = self
            .slots
            .iter()
            .position(|slot| slot.state.load(Ordering::Acquire) == FREE)?;
        self.slots[at].group = group;
        Some(Timing(at as u32))
    }

    /// What a run's pass of `timing` writes: its start for the `first` run, its end always.
    pub(crate) fn writes(
        &self,
        timing: Timing,
        first: bool,
    ) -> wgpu::ComputePassTimestampWrites<'_> {
        wgpu::ComputePassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: first.then_some(2 * timing.0),
            end_of_pass_write_index: Some(2 * timing.0 + 1),
        }
    }

    /// As `writes`, for a render pass (S57's raster probe): the start for the `first` pass, the
    /// end for the `last`.
    pub(crate) fn render_writes(
        &self,
        timing: Timing,
        (first, last): (bool, bool),
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        (first || last).then_some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: first.then_some(2 * timing.0),
            end_of_pass_write_index: last.then_some(2 * timing.0 + 1),
        })
    }

    /// After the evaluation's last run: its two timestamps resolved and copied out, to be read
    /// back once the GPU is there.
    pub(crate) fn finish(&mut self, gpu: &Gpu, Timing(at): Timing) {
        let slot = &self.slots[at as usize];
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let offset = STRIDE * u64::from(at);
        encoder.resolve_query_set(&self.set, 2 * at..2 * at + 2, &self.resolve, offset);
        encoder.copy_buffer_to_buffer(&self.resolve, offset, &slot.read, 0, 16);
        gpu.submit([encoder.finish()]);
        slot.state.store(FLYING, Ordering::Release);
        let state = Arc::clone(&slot.state);
        slot.read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(
                    if result.is_ok() { MAPPED } else { FREE },
                    Ordering::Release,
                );
            });
    }

    /// Every reading back since the last call: each evaluation's tiles and measured time, ns.
    pub(crate) fn readings(&mut self, gpu: &Gpu) -> Vec<(Group, f64)> {
        // Nothing waits: whatever the GPU finished calls back now.
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        let mut readings = Vec::new();
        for slot in &self.slots {
            if slot.state.load(Ordering::Acquire) != MAPPED {
                continue;
            }
            let ticks: Vec<u64> = match slot.read.get_mapped_range(..) {
                Ok(range) => (range.as_chunks::<8>().0.iter())
                    .map(|bytes| u64::from_le_bytes(*bytes))
                    .collect(),
                Err(_) => Vec::new(),
            };
            slot.read.unmap();
            slot.state.store(FREE, Ordering::Release);
            if let [start, end] = ticks[..]
                && end > start
            {
                readings.push((slot.group, (end - start) as f64 * self.period));
            }
        }
        readings
    }
}
