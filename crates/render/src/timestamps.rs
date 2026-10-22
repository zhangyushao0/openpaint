//! GPU timestamp pairs read back without waiting: a query set of slots, each a start and an end
//! timestamp, resolved and copied into a buffer of its own once written, mapped, and read back
//! frames later by a poll that never waits. The background work's clock (`gpu_clock.rs`) and the
//! frames' clock (`frame_clock.rs`) each hold one. A device without timestamp queries has none.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// Where each slot's two timestamps are resolved: the alignment resolving needs.
const STRIDE: u64 = 256;

/// A slot's state: free, held by what is being encoded, its timestamps on their way back, or read
/// back and waiting to be taken.
const FREE: u8 = 0;
const HELD: u8 = 1;
const FLYING: u8 = 2;
const MAPPED: u8 = 3;

pub(crate) struct Timestamps {
    pub(crate) set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    /// Per slot, the buffer its pair is read back through and its state.
    reads: Vec<(wgpu::Buffer, Arc<AtomicU8>)>,
    /// Nanoseconds a timestamp tick.
    period: f64,
}

impl Timestamps {
    /// `slots` pairs on `device`, named `label`; `None` without timestamp queries.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slots: u32,
        label: &str,
    ) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some(label),
            ty: wgpu::QueryType::Timestamp,
            count: 2 * slots,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: STRIDE * u64::from(slots),
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let reads = (0..slots)
            .map(|_| {
                let read = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: 16,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                (read, Arc::new(AtomicU8::new(FREE)))
            })
            .collect();
        Some(Self {
            set,
            resolve,
            reads,
            period: f64::from(queue.get_timestamp_period()),
        })
    }

    /// A free slot, held from now on; `None` while every slot is busy.
    pub(crate) fn claim(&self) -> Option<u32> {
        let at =
            (self.reads.iter()).position(|(_, state)| state.load(Ordering::Acquire) == FREE)?;
        self.reads[at].1.store(HELD, Ordering::Release);
        Some(at as u32)
    }

    /// Slot `slot` is free again (what held it was not submitted).
    pub(crate) fn free(&self, slot: u32) {
        self.reads[slot as usize].1.store(FREE, Ordering::Release);
    }

    /// Slot `slot`'s pair resolved and copied out by `encoder`, after its last write.
    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder, slot: u32) {
        let offset = STRIDE * u64::from(slot);
        let read = &self.reads[slot as usize].0;
        encoder.resolve_query_set(&self.set, 2 * slot..2 * slot + 2, &self.resolve, offset);
        encoder.copy_buffer_to_buffer(&self.resolve, offset, read, 0, 16);
    }

    /// Slot `slot`'s resolve is submitted: its pair is read back once the GPU is there.
    pub(crate) fn read_back(&self, slot: u32) {
        let (read, state) = &self.reads[slot as usize];
        state.store(FLYING, Ordering::Release);
        let state = Arc::clone(state);
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(
                    if result.is_ok() { MAPPED } else { FREE },
                    Ordering::Release,
                );
            });
    }

    /// Every pair read back since the last call, after a poll that never waits: each slot and the
    /// time between its two timestamps, ns. Their slots are free again.
    pub(crate) fn readings(&self, device: &wgpu::Device) -> Vec<(u32, f64)> {
        // Nothing waits: whatever the GPU finished calls back now.
        let _ = device.poll(wgpu::PollType::Poll);
        let mut readings = Vec::new();
        for (slot, (read, state)) in (0..).zip(&self.reads) {
            if state.load(Ordering::Acquire) != MAPPED {
                continue;
            }
            let ticks: Vec<u64> = match read.get_mapped_range(..) {
                Ok(range) => (range.as_chunks::<8>().0.iter())
                    .map(|bytes| u64::from_le_bytes(*bytes))
                    .collect(),
                Err(_) => Vec::new(),
            };
            read.unmap();
            state.store(FREE, Ordering::Release);
            if let [start, end] = ticks[..]
                && end > start
            {
                readings.push((slot, (end - start) as f64 * self.period));
            }
        }
        readings
    }
}
