//! The screen's frames the GPU has not finished (S56, frame pacing). After a frame's work is
//! submitted the runtime marks it (`Core::frame_submitted`); wgpu calls back once the GPU has
//! finished everything submitted before the mark, workers' work too, and polling without waiting
//! (`Core::frames_in_flight`) collects those calls. The runtime submits no new frame while more
//! than one is in flight (`runtime/src/pacing.rs`), so the GPU's queue never holds more than a
//! frame or two of the screen's work and no wait for it (acquiring the window's next image,
//! presenting, a readback) lasts longer than that.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::Core;

#[derive(Debug, Default)]
pub(crate) struct GpuFrames {
    /// Frames marked so far.
    submitted: AtomicU64,
    /// The last frame the GPU finished, as its callback said.
    finished: Arc<AtomicU64>,
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
        if let Err(error) = self.gpu.device.poll(wgpu::PollType::Poll) {
            log::debug!("polling for finished frames: {error}");
        }
        let submitted = self.frames.submitted.load(Ordering::Relaxed);
        submitted.saturating_sub(self.frames.finished.load(Ordering::Acquire))
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
}
