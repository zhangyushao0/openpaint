//! The video worker (S163): the replay's video made off the frame loop on the thread
//! `openpaint-video`, one job at a time. Every frame of the job's `VideoFrames` is drawn by a
//! `VideoTarget` of its own (the moment's artwork and the stroke being drawn, scaled to the
//! video's size and read back); the end held is the finished picture (a segment's too) or, with
//! the caption, the end card (the line the composition root laid, painted by a painter of its own
//! into a `ScreenTarget` of the band's size, `openpaint_ui::video_caption`). Each frame goes to
//! the platform's encoder with its time (`k × 1_000_000 / fps` µs). How many frames it made, the
//! user's Cancel and the bytes its target holds on the GPU are atomics the frame loop shares
//! (`VideoShared`): it stops between frames and the encoder drops what it began (also after a
//! failure); the GPU memory budget counts the bytes while the video is made. Each frame's GPU
//! work is finished before the next is drawn (the target waits for each region and for its
//! readback), the frame read into the one image the target keeps. It holds only an `Arc<Core>`
//! and the job's values.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use openpaint_app::{At, VideoEncoder, VideoError, VideoFrame, VideoFrames, VideoJob, VideoOrder};
use openpaint_core::DrawingOrder;
use openpaint_render::{
    Core, FrameRequest, RenderError, RgbaImage, ScreenFrame, ScreenPainter, ScreenTarget, UiFrame,
    VideoTarget,
};
use openpaint_ui::caption_band;
use openpaint_ui::design::SURROUND;

use crate::canvas::PAPER;
use crate::error::RuntimeError;
use crate::shared::SharedCore;
use crate::worker::Queue;

/// What a video being made shares with the frame loop: the frames handed to the encoder so far,
/// the user's Cancel, and the bytes its target holds on the GPU (0 before its first frame and
/// once it ended).
#[derive(Debug, Default)]
pub struct VideoShared {
    pub made: AtomicU32,
    pub cancel: AtomicBool,
    pub held: AtomicU64,
}

/// One video for the worker: the job, its end card's line laid (with the caption on), the room
/// its target's arena keeps within, and what it shares with the frame loop.
pub(crate) struct VideoWork {
    job: VideoJob,
    caption: Option<UiFrame>,
    arena: u64,
    shared: Arc<VideoShared>,
}

/// The worker and what the job handed to it last shares.
pub(crate) struct VideoWorker {
    worker: Queue<VideoWork, Result<(), VideoError>>,
    shared: Arc<VideoShared>,
}

impl VideoWorker {
    pub(crate) fn start(core: SharedCore) -> Result<Self, RuntimeError> {
        let worker = Queue::spawn("openpaint-video", move |work: VideoWork| {
            let shared = (&*work.shared, work.arena);
            make_video(&core.get(), (work.job, work.caption), shared)
        })?;
        Ok(Self {
            worker,
            shared: Arc::default(),
        })
    }

    /// Makes `job`'s video, its end card with `caption` (the line laid for it) when given, its
    /// target's arena within `arena` bytes.
    pub(crate) fn submit(&mut self, job: VideoJob, caption: Option<UiFrame>, arena: u64) {
        self.shared = Arc::default();
        log::info!(
            "video {:?} of {} frames asked",
            job.order,
            job.frames.count()
        );
        self.worker.submit(VideoWork {
            job,
            caption,
            arena,
            shared: Arc::clone(&self.shared),
        });
    }

    /// The user's Cancel: the job stops before its next frame.
    pub(crate) fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::Release);
    }

    /// How many frames of the last job went to its encoder so far.
    pub(crate) fn made(&self) -> u32 {
        self.shared.made.load(Ordering::Acquire)
    }

    /// The bytes the last job's target holds on the GPU now (0 while none is drawn).
    pub(crate) fn held(&self) -> u64 {
        self.shared.held.load(Ordering::Acquire)
    }

    /// The last job's end, once it ended.
    pub(crate) fn take(&mut self) -> Option<Result<(), VideoError>> {
        self.worker.take()
    }

    /// A job is queued or running, or its end waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }
}

impl Drop for VideoWorker {
    /// The app ends: a video under way stops at its next frame and drops what it began, then the
    /// thread is joined.
    fn drop(&mut self) {
        self.cancel();
    }
}

/// `job`'s video made on `core`: `begin`, every frame (its end card with `caption`, the line
/// laid for it, when given), `finish`; `shared` counts the frames handed over and the bytes the
/// target holds, its Cancel stops it before the next frame; the target's arena keeps within
/// `arena` bytes. The video worker's path, which tests and timings call as it is. Its end is
/// logged; a panic is a bug, logged and reported as a failure to draw. The encoder drops what it
/// began unless the video finished.
///
/// # Errors
/// `Cancelled` once Cancel is set; `Render` when the GPU fails; the encoder's.
pub fn make_video(
    core: &Core,
    (job, caption): (VideoJob, Option<UiFrame>),
    (shared, arena): (&VideoShared, u64),
) -> Result<(), VideoError> {
    let made = &shared.made;
    let VideoJob {
        order,
        frames,
        mut encoder,
        ..
    } = job;
    let started = Instant::now();
    let mut spent = Spent::default();
    let mut ends = Ends {
        caption,
        finished: None,
        card: None,
    };
    let made_whole = catch_unwind(AssertUnwindSafe(|| {
        encoder.begin(&order)?;
        let work = (&order, &frames, &mut ends);
        spent = encode(core, (work, arena), encoder.as_mut(), shared)?;
        encoder.finish()
    }));
    shared.held.store(0, Ordering::Release);
    let result = made_whole.unwrap_or_else(|_| {
        log::error!("making the video panicked");
        Err(VideoError::Render)
    });
    let frames_made = made.load(Ordering::Acquire);
    match &result {
        Ok(()) => log::info!(
            "video {} made: {frames_made} frames of {} x {} in {} ms (drawn {} ms, encoded {} ms)",
            order.name,
            order.size[0],
            order.size[1],
            started.elapsed().as_millis(),
            spent.drawing.as_millis(),
            spent.encoding.as_millis()
        ),
        Err(VideoError::Cancelled) => log::info!("video cancelled after {frames_made} frames"),
        Err(error) => log::error!("video failed after {frames_made} frames: {error}"),
    }
    if result.is_err() {
        encoder.cancel();
    }
    result
}

/// Where a job's time went.
#[derive(Debug, Default, Clone, Copy)]
struct Spent {
    drawing: Duration,
    encoding: Duration,
}

/// Every frame of `frames` drawn (the target's arena within `arena` bytes) and handed to
/// `encoder`, `shared` counting them and the target's bytes, until done or Cancel.
fn encode(
    core: &Core,
    ((order, frames, ends), arena): ((&VideoOrder, &VideoFrames, &mut Ends), u64),
    encoder: &mut dyn VideoEncoder,
    shared: &VideoShared,
) -> Result<Spent, VideoError> {
    let canvas = frames.drawing().artwork().canvas();
    let canvas = [canvas.width(), canvas.height()];
    let mut target = VideoTarget::new(core, canvas, (order.size, arena)).map_err(rendering)?;
    shared.held.store(target.bytes().total(), Ordering::Release);
    let fps = u64::from(order.fps.max(1));
    let mut spent = Spent::default();
    for index in 0..frames.count() {
        if shared.cancel.load(Ordering::Acquire) {
            return Err(VideoError::Cancelled);
        }
        let drawn = Instant::now();
        let image = match frames.frame(index) {
            VideoFrame::At(at) => drawn_at(core, &mut target, frames, at)?,
            VideoFrame::End => ends.end(core, &mut target, frames.drawing())?,
        };
        let encoded = Instant::now();
        spent.drawing += encoded - drawn;
        encoder.frame(image.pixels(), u64::from(index) * 1_000_000 / fps)?;
        spent.encoding += encoded.elapsed();
        shared.held.store(target.bytes().total(), Ordering::Release);
        shared.made.store(index + 1, Ordering::Release);
    }
    Ok(spent)
}

/// The replay at `at`, drawn and read back into the target's image.
fn drawn_at<'a>(
    core: &Core,
    target: &'a mut VideoTarget,
    frames: &VideoFrames,
    at: At,
) -> Result<&'a RgbaImage, VideoError> {
    let frame = frames.drawing().frame(at.done, at.samples);
    let request = FrameRequest {
        artwork: &frame.artwork,
        active: frame.active.as_ref(),
        background: PAPER,
    };
    target.frame(core, &request).map_err(rendering)
}

/// The end held: the finished picture, and the card once made.
struct Ends {
    /// The line laid for the card, until it is painted.
    caption: Option<UiFrame>,
    finished: Option<RgbaImage>,
    card: Option<RgbaImage>,
}

impl Ends {
    /// A frame of the end: the card with the line, else the finished picture ("Stop on the
    /// finished picture": a segment's video too ends on the whole drawing, whose line the card
    /// tells). The card holds the picture the target drew last: the finished one.
    fn end(
        &mut self,
        core: &Core,
        target: &mut VideoTarget,
        drawing: &DrawingOrder,
    ) -> Result<&RgbaImage, VideoError> {
        if self.finished.is_none() {
            let finished = drawing.frame(drawing.steps().len(), 0);
            let request = FrameRequest {
                artwork: &finished.artwork,
                active: None,
                background: PAPER,
            };
            self.finished = Some(target.frame(core, &request).map_err(rendering)?.clone());
        }
        if let Some(caption) = self.caption.take() {
            let band = band(core, caption, caption_band(target.size())).map_err(rendering)?;
            self.card = Some(target.card(core, (SURROUND, &band)).map_err(rendering)?);
        }
        self.card
            .as_ref()
            .or(self.finished.as_ref())
            .ok_or(VideoError::Render)
    }
}

impl Drop for Ends {
    /// A line never painted (the video cancelled or failed before its end) is let go: its
    /// textures are its own context's, which is gone, and epaint asserts every delta is handled.
    fn drop(&mut self) {
        if let Some(caption) = &mut self.caption {
            caption.textures.clear();
        }
    }
}

/// The caption's band of `size` pixels: the desk with the line painted over it.
fn band(core: &Core, caption: UiFrame, size: [u32; 2]) -> Result<RgbaImage, RenderError> {
    let mut painter = ScreenPainter::offscreen(core);
    let target = ScreenTarget::new(core, size)?;
    let frame = ScreenFrame {
        surround: SURROUND,
        canvas: None,
        ui: caption,
    };
    core.render_screen(&mut painter, &target, frame)?;
    target.read(core)
}

fn rendering(error: RenderError) -> VideoError {
    log::error!("a frame of the video could not be drawn: {error}");
    VideoError::Render
}
