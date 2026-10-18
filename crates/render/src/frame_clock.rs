//! The screen's frames' GPU time, measured for the scenario benchmarks (S63): while it is on, a
//! frame's first submission on the frame loop's thread carries a timestamp written once the GPU
//! has finished everything submitted before it, and the frame's end another once the GPU has
//! finished the frame's own work; their difference is what the frame cost the GPU (its gaps
//! included, where the GPU waited for the frame loop to submit). Read back without waiting, a few
//! frames later. Off unless a benchmark turns it on (`Core::time_frames`).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::ThreadId;

/// Frames measured at once, at most; a frame finding none free is not measured.
const SLOTS: u32 = 8;
/// Where each slot's two timestamps are resolved: the alignment resolving needs.
const STRIDE: u64 = 256;

/// A slot's state: free, being written by a frame, on its way back, read back.
const FREE: u8 = 0;
const HELD: u8 = 1;
const FLYING: u8 = 2;
const MAPPED: u8 = 3;

struct Slot {
    read: wgpu::Buffer,
    state: Arc<AtomicU8>,
}

/// The frame being measured: its slot, the thread drawing it, and whether its start is written.
struct Frame {
    slot: u32,
    thread: ThreadId,
    started: bool,
}

pub(crate) struct FrameClock {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    /// Nanoseconds a timestamp tick.
    period: f64,
    frame: Mutex<Option<Frame>>,
}

impl FrameClock {
    /// A clock on `device`; `None` without timestamp queries.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2 * SLOTS,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame timestamps"),
            size: STRIDE * u64::from(SLOTS),
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..SLOTS)
            .map(|_| Slot {
                read: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frame timestamps read"),
                    size: 16,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(FREE)),
            })
            .collect();
        Some(Self {
            set,
            resolve,
            slots,
            period: f64::from(queue.get_timestamp_period()),
            frame: Mutex::new(None),
        })
    }

    fn frame(&self) -> std::sync::MutexGuard<'_, Option<Frame>> {
        self.frame.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A frame begins on this thread: its first submission will carry its start (a frame left
    /// unended gives its slot back).
    pub(crate) fn begin(&self) {
        let mut frame = self.frame();
        if let Some(left) = frame.take() {
            self.slots[left.slot as usize]
                .state
                .store(FREE, Ordering::Release);
        }
        let free = (self.slots.iter()).position(|slot| slot.state.load(Ordering::Acquire) == FREE);
        if let Some(at) = free {
            self.slots[at].state.store(HELD, Ordering::Release);
            *frame = Some(Frame {
                slot: at as u32,
                thread: std::thread::current().id(),
                started: false,
            });
        }
    }

    /// The commands writing the frame's start, for this thread's first submission of the frame.
    pub(crate) fn start(&self, device: &wgpu::Device) -> Option<wgpu::CommandBuffer> {
        let mut frame = self.frame();
        let held = frame.as_mut()?;
        if held.started || held.thread != std::thread::current().id() {
            return None;
        }
        held.started = true;
        Some(self.stamp(device, 2 * held.slot))
    }

    /// The frame's work is all submitted: its end written, both resolved and read back later; a
    /// frame that submitted nothing is not measured.
    pub(crate) fn end(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(frame) = self.frame().take() else {
            return;
        };
        let slot = &self.slots[frame.slot as usize];
        if !frame.started {
            slot.state.store(FREE, Ordering::Release);
            return;
        }
        let at = frame.slot;
        let mut encoder = device.create_command_encoder(&Default::default());
        stamped(&mut encoder, &self.set, 2 * at + 1);
        let offset = STRIDE * u64::from(at);
        encoder.resolve_query_set(&self.set, 2 * at..2 * at + 2, &self.resolve, offset);
        encoder.copy_buffer_to_buffer(&self.resolve, offset, &slot.read, 0, 16);
        queue.submit([encoder.finish()]);
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

    /// Every frame's GPU time read back since the last call, ns, after a poll that never waits.
    pub(crate) fn readings(&self, device: &wgpu::Device) -> Vec<f64> {
        let _ = device.poll(wgpu::PollType::Poll);
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
                readings.push((end - start) as f64 * self.period);
            }
        }
        readings
    }

    /// A command buffer writing timestamp `index` once everything before it is finished.
    fn stamp(&self, device: &wgpu::Device, index: u32) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&Default::default());
        stamped(&mut encoder, &self.set, index);
        encoder.finish()
    }
}

/// An empty compute pass writing timestamp `index` at its end: when the GPU has finished every
/// command before it.
fn stamped(encoder: &mut wgpu::CommandEncoder, set: &wgpu::QuerySet, index: u32) {
    drop(encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("frame timestamp"),
        timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
            query_set: set,
            beginning_of_pass_write_index: None,
            end_of_pass_write_index: Some(index),
        }),
    }));
}
