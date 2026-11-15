//! The screen's frames the GPU has not finished (frame pacing), and the background work it
//! has queued. After a frame's work is submitted the runtime marks it
//! (`Core::frame_submitted`); wgpu calls back once the GPU has finished everything submitted
//! before the mark, workers' work too, and polling without waiting (`Core::frames_in_flight`)
//! collects those calls. The runtime submits no new frame while more than one is in flight
//! (`runtime/src/pacing.rs`), so the GPU's queue never holds more than a frame or two of the
//! screen's work and no wait for it (acquiring the window's next image, presenting, a readback)
//! lasts longer than that.
//!
//! Background work is marked the same way with its estimated GPU time
//! (`Core::background_submitted`): what is submitted and not finished is what a frame drawn now
//! would wait for (`Core::background_queued`), which the frame budget keeps within one idle
//! frame's share while nothing touches the screen (`pace/`).

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

    /// Background work estimated at `ns` of GPU time is on the queue: it counts as queued
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
    /// one), as `background_queued` last polled them.
    pub(crate) fn background_pieces(&self) -> u64 {
        let pieces = self.frames.pieces.load(Ordering::Relaxed);
        pieces.saturating_sub(self.frames.pieces_done.load(Ordering::Acquire))
    }

    /// Background work on the GPU's queue not finished yet, by its estimates in ns, after a poll
    /// that never waits: what a frame submitted now would wait for.
    #[must_use]
    pub fn background_queued(&self) -> f64 {
        self.poll();
        let work = self.frames.work.load(Ordering::Relaxed);
        work.saturating_sub(self.frames.done.load(Ordering::Acquire)) as f64
    }

    /// From now on (`on`) the screen's frames are timed on the GPU, or no longer (the
    /// scenario benchmarks; `frame_clock.rs`). A device without timestamps times nothing.
    pub fn time_frames(&self, on: bool) {
        self.gpu.timing.store(on, Ordering::Relaxed);
    }

    /// A frame of the screen begins on this thread (the runtime calls it each frame): while
    /// frames are timed, its first submission writes its start.
    pub fn frame_begun(&self) {
        if let Some(clock) = self.timed() {
            clock.begin();
        }
    }

    /// The frame's work is all submitted (after its present): while frames are timed, its end.
    pub fn frame_ended(&self) {
        if let Some(clock) = self.timed() {
            clock.end(&self.gpu.device, &self.gpu.queue);
        }
    }

    /// The GPU time of each timed frame read back since the last call, ns, without waiting.
    #[must_use]
    pub fn frame_times(&self) -> Vec<f64> {
        (self.gpu.frames.as_ref()).map_or_else(Vec::new, |clock| clock.readings(&self.gpu.device))
    }

    fn timed(&self) -> Option<&crate::frame_clock::FrameClock> {
        (self.gpu.frames.as_ref()).filter(|_| self.gpu.timing.load(Ordering::Relaxed))
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
    use crate::core::{Core, clear_attachment, test_core};
    use crate::readback::{FORMAT, target_texture};

    /// A frame's work on this thread: an offscreen target of `size` pixels cleared.
    fn clear(core: &Core, size: [u32; 2]) {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let texture = target_texture(&core.gpu, (size, FORMAT), usage).expect("a target");
        let view = texture.create_view(&Default::default());
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(clear_attachment(&view, [1, 2, 3, 255]))],
            ..Default::default()
        }));
        core.gpu.submit([encoder.finish()]);
    }

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
    fn a_timed_frame_reads_back_its_gpu_time_and_an_empty_one_none() {
        let core = test_core();
        let _ = core.frame_times();
        core.time_frames(true);
        core.frame_begun();
        core.frame_ended();
        core.frame_begun();
        clear(core, [64, 64]);
        core.frame_ended();
        core.time_frames(false);
        core.wait_idle().expect("idle");
        let times = core.frame_times();
        if core.adapter().contains("timestamps measured") {
            assert_eq!(times.len(), 1, "the frame that submitted: {times:?}");
            assert!(times[0] > 0.0);
        }
        core.frame_begun();
        clear(core, [8, 8]);
        core.frame_ended();
        core.wait_idle().expect("idle");
        assert!(core.frame_times().is_empty(), "not timed");
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
