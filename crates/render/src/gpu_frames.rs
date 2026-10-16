//! The screen's frames the GPU has not finished (S56, frame pacing), and the background work it
//! has queued (S60). After a frame's work is submitted the runtime marks it
//! (`Core::frame_submitted`); wgpu calls back once the GPU has finished everything submitted
//! before the mark, workers' work too, and polling without waiting (`Core::frames_in_flight`)
//! collects those calls. The runtime submits no new frame while more than one is in flight
//! (`runtime/src/pacing.rs`), so the GPU's queue never holds more than a frame or two of the
//! screen's work and no wait for it (acquiring the window's next image, presenting, a readback)
//! lasts longer than that.
//!
//! Background work (S60, DECISIONS D87) is marked the same way with its estimated GPU time
//! (`Core::background_submitted`): what is submitted and not finished is what a frame drawn now
//! would wait for (`Core::background_queued`), which the frame budget keeps within one idle
//! frame's share while nothing touches the screen (`pace.rs`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::Core;

#[derive(Debug, Default)]
pub(crate) struct GpuFrames {
    /// Frames marked so far.
    submitted: AtomicU64,
    /// The last frame the GPU finished, as its callback said.
    finished: Arc<AtomicU64>,
    /// Background work's estimated GPU time submitted so far and finished so far, ns, and its
    /// pieces (each mark one).
    work: AtomicU64,
    done: Arc<AtomicU64>,
    pieces: AtomicU64,
    pieces_done: Arc<AtomicU64>,
}

impl Core {
    /// Everything the screen's frame submitted is on the queue: it counts as in flight until the
    /// GPU has finished it (and everything submitted before it).
    pub fn frame_submitted(&self) {
        let frame = self.frames.submitted.fetch_add(1, Ordering::Relaxed) + 1;
        let finished = Arc::clone(&self.frames.finished);
        self.gpu.queue.on_submitted_work_done(move || {
            finished.fetch_max(frame, Ordering::Release);
        });
    }

    /// The screen's frames submitted that the GPU has not finished, after a poll that never
    /// waits.
    #[must_use]
    pub fn frames_in_flight(&self) -> u64 {
        self.poll();
        let submitted = self.frames.submitted.load(Ordering::Relaxed);
        submitted.saturating_sub(self.frames.finished.load(Ordering::Acquire))
    }

    /// Background work estimated at `ns` of GPU time is on the queue (S60): it counts as queued
    /// until the GPU has finished everything submitted so far.
    pub(crate) fn background_submitted(&self, ns: f64) {
        if ns <= 0.0 {
            return;
        }
        let ns = ns as u64;
        self.frames.work.fetch_add(ns, Ordering::Relaxed);
        self.frames.pieces.fetch_add(1, Ordering::Relaxed);
        let (done, pieces) = (
            Arc::clone(&self.frames.done),
            Arc::clone(&self.frames.pieces_done),
        );
        self.gpu.queue.on_submitted_work_done(move || {
            done.fetch_add(ns, Ordering::Release);
            pieces.fetch_add(1, Ordering::Release);
        });
    }

    /// Pieces of background work on the GPU's queue not finished yet (each `background_submitted`
    /// one), as `background_queued` last polled them (S60).
    pub(crate) fn background_pieces(&self) -> u64 {
        let pieces = self.frames.pieces.load(Ordering::Relaxed);
        pieces.saturating_sub(self.frames.pieces_done.load(Ordering::Acquire))
    }

    /// Background work on the GPU's queue not finished yet, by its estimates in ns, after a poll
    /// that never waits (S60): what a frame submitted now would wait for.
    #[must_use]
    pub fn background_queued(&self) -> f64 {
        self.poll();
        let work = self.frames.work.load(Ordering::Relaxed);
        work.saturating_sub(self.frames.done.load(Ordering::Acquire)) as f64
    }

    /// Collects the GPU's finished-work callbacks without waiting.
    fn poll(&self) {
        if let Err(error) = self.gpu.device.poll(wgpu::PollType::Poll) {
            log::debug!("polling for finished work: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::test_core;

    #[test]
    fn a_frame_is_in_flight_until_the_gpu_finished_it() {
        let core = test_core();
        let before = core.frames_in_flight();
        core.frame_submitted();
        core.frame_submitted();
        assert!(core.frames_in_flight() <= before + 2);
        core.wait_idle().expect("idle");
        assert_eq!(core.frames_in_flight(), 0, "every frame finished");
    }

    #[test]
    fn background_work_is_queued_until_the_gpu_finished_it() {
        let core = test_core();
        core.background_submitted(3_000_000.0);
        core.background_submitted(0.0);
        assert!(core.background_queued() <= 3_000_000.0);
        core.wait_idle().expect("idle");
        assert_eq!(core.background_queued(), 0.0, "every piece finished");
    }
}
