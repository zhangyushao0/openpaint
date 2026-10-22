//! The screen's frames' GPU time, measured for the scenario benchmarks: while it is on, a frame's
//! first submission on the frame loop's thread carries a timestamp written once the GPU has
//! finished everything submitted before it, and the frame's end another once the GPU has finished
//! the frame's own work; their difference is what the frame cost the GPU (its gaps included, where
//! the GPU waited for the frame loop to submit). Read back without waiting, a few frames later
//! (`timestamps.rs`). Off unless a benchmark turns it on (`Core::time_frames`).

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::ThreadId;

use crate::timestamps::Timestamps;

/// Frames measured at once, at most; a frame finding none free is not measured.
const SLOTS: u32 = 8;

/// The frame being measured: its slot, the thread drawing it, and whether its start is written.
struct Frame {
    slot: u32,
    thread: ThreadId,
    started: bool,
}

pub(crate) struct FrameClock {
    stamps: Timestamps,
    frame: Mutex<Option<Frame>>,
}

impl FrameClock {
    /// A clock on `device`; `None` without timestamp queries.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        Some(Self {
            stamps: Timestamps::new(device, queue, SLOTS, "frame timestamps")?,
            frame: Mutex::new(None),
        })
    }

    fn frame(&self) -> MutexGuard<'_, Option<Frame>> {
        self.frame.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A frame begins on this thread: its first submission will carry its start (a frame left
    /// unended gives its slot back).
    pub(crate) fn begin(&self) {
        let mut frame = self.frame();
        if let Some(left) = frame.take() {
            self.stamps.free(left.slot);
        }
        *frame = self.stamps.claim().map(|slot| Frame {
            slot,
            thread: std::thread::current().id(),
            started: false,
        });
    }

    /// The commands writing the frame's start, for this thread's first submission of the frame.
    pub(crate) fn start(&self, device: &wgpu::Device) -> Option<wgpu::CommandBuffer> {
        let mut frame = self.frame();
        let held = frame.as_mut()?;
        if held.started || held.thread != std::thread::current().id() {
            return None;
        }
        held.started = true;
        let mut encoder = device.create_command_encoder(&Default::default());
        stamped(&mut encoder, &self.stamps.set, 2 * held.slot);
        Some(encoder.finish())
    }

    /// The frame's work is all submitted: its end written, both resolved and read back later; a
    /// frame that submitted nothing is not measured.
    pub(crate) fn end(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(frame) = self.frame().take() else {
            return;
        };
        if !frame.started {
            self.stamps.free(frame.slot);
            return;
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        stamped(&mut encoder, &self.stamps.set, 2 * frame.slot + 1);
        self.stamps.resolve(&mut encoder, frame.slot);
        queue.submit([encoder.finish()]);
        self.stamps.read_back(frame.slot);
    }

    /// Every frame's GPU time read back since the last call, ns, after a poll that never waits.
    pub(crate) fn readings(&self, device: &wgpu::Device) -> Vec<f64> {
        (self.stamps.readings(device).into_iter())
            .map(|(_, measured)| measured)
            .collect()
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
