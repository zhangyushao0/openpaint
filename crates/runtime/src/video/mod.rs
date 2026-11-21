//! The video worker (S163): the replay's video made off the frame loop on the thread
//! `openpaint-video`, one job at a time, on a GPU device of its own (`device.rs`): opened beside
//! the screen's when the video starts, made anew within it when it went as far as `VideoBounds`
//! lets one device go, let go when it ends. Every frame of the job's `VideoFrames` is drawn by a
//! `VideoTarget` on it (the moment's artwork and the stroke being drawn, scaled to the video's
//! size and read back); the end held is the finished picture or the card (`ends.rs`). Each frame
//! goes to the platform's encoder with its time (`k × 1_000_000 / fps` µs). How many frames it
//! made, the user's Cancel and the bytes its target holds on the GPU are atomics the frame loop
//! shares (`VideoShared`): it stops between frames and the encoder drops what it began (also
//! after a failure); the GPU memory budget counts the bytes while the video is made. Each frame's
//! GPU work is finished before the next is drawn (the target waits for each region and for its
//! readback), the frame read into the one image the target keeps. Every `NOTE_EVERY` frames and
//! at its end it logs what the process's resident memory gained while frames were drawn and
//! while they were encoded, with its device's allocator and wgpu's live objects.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use openpaint_app::{At, VideoEncoder, VideoError, VideoFrame, VideoFrames, VideoJob, VideoOrder};
use openpaint_render::{Core, FrameRequest, RenderError, RgbaImage, UiFrame, VideoTarget};

use crate::canvas::PAPER;
use crate::error::RuntimeError;
use crate::resident::resident;
use crate::shared::SharedCore;
use crate::worker::Queue;

mod device;
mod ends;

pub use device::VideoBounds;
use device::VideoDevice;
use ends::Ends;

/// What a video being made shares with the frame loop: the frames handed to the encoder so far,
/// the user's Cancel, and the bytes its target holds on the GPU (0 before its first frame and
/// once it ended).
#[derive(Debug, Default)]
pub struct VideoShared {
    pub made: AtomicU32,
    pub cancel: AtomicBool,
    pub held: AtomicU64,
}

/// One video for the worker: the job, its end card's line laid (with the caption on), how far its
/// devices go, and what it shares with the frame loop.
pub(crate) struct VideoWork {
    job: VideoJob,
    caption: Option<UiFrame>,
    bounds: VideoBounds,
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
            let shared = (&*work.shared, work.bounds);
            make_video(&core.get(), (work.job, work.caption), shared)
        })?;
        Ok(Self {
            worker,
            shared: Arc::default(),
        })
    }

    /// Makes `job`'s video, its end card with `caption` (the line laid for it) when given, its
    /// devices within `bounds`.
    pub(crate) fn submit(&mut self, job: VideoJob, caption: Option<UiFrame>, bounds: VideoBounds) {
        self.shared = Arc::default();
        log::info!(
            "video {:?} of {} frames asked",
            job.order,
            job.frames.count()
        );
        self.worker.submit(VideoWork {
            job,
            caption,
            bounds,
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

/// `job`'s video made on devices of its own opened beside `core`'s (`device.rs`) within
/// `bounds`: `begin`, every frame (its end card with `caption`, the line laid for it, when
/// given), `finish`; `shared` counts the frames handed over and the bytes the target holds, its
/// Cancel stops it before the next frame. The video worker's path, which tests and timings call
/// as it is. Its end is logged; a panic is a bug, logged and reported as a failure to draw. The
/// encoder drops what it began unless the video finished.
///
/// # Errors
/// `Cancelled` once Cancel is set; `Render` when the GPU fails; the encoder's.
pub fn make_video(
    core: &Core,
    (job, caption): (VideoJob, Option<UiFrame>),
    (shared, bounds): (&VideoShared, VideoBounds),
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
        spent = encode(core, (work, bounds), encoder.as_mut(), shared)?;
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
            "video {} made: {frames_made} frames of {} x {} in {} ms (drawn {} ms, encoded {} \
             ms); resident memory {:+.1} MB while drawing, {:+.1} while encoding, {:.1} now",
            order.name,
            order.size[0],
            order.size[1],
            started.elapsed().as_millis(),
            spent.drawing.as_millis(),
            spent.encoding.as_millis(),
            mb(spent.grown[0]),
            mb(spent.grown[1]),
            mb(resident().unwrap_or(0).cast_signed())
        ),
        Err(VideoError::Cancelled) => log::info!("video cancelled after {frames_made} frames"),
        Err(error) => log::error!("video failed after {frames_made} frames: {error}"),
    }
    if result.is_err() {
        encoder.cancel();
    }
    result
}

/// A memory note every this many frames.
const NOTE_EVERY: u32 = 30;

fn mb(bytes: i64) -> f64 {
    bytes as f64 / f64::from(1 << 20)
}

/// Where a job's time went, and the resident memory the process gained while frames were drawn
/// and while they were encoded (where the system says, `resident.rs`).
#[derive(Debug, Default, Clone, Copy)]
struct Spent {
    drawing: Duration,
    encoding: Duration,
    grown: [i64; 2],
}

/// An instant with the process's resident memory then.
type Mark = (Instant, Option<u64>);

/// The resident memory gained from `from` to `to` (0 where the system does not say).
fn gained((_, from): Mark, (_, to): Mark) -> i64 {
    from.zip(to)
        .map_or(0, |(from, to)| to.cast_signed() - from.cast_signed())
}

impl Spent {
    /// One frame's phases: drawn from `start` to `drawn`, encoded from `drawn` to `encoded`.
    fn add(&mut self, [start, drawn, encoded]: [Mark; 3]) {
        self.drawing += drawn.0 - start.0;
        self.encoding += encoded.0 - drawn.0;
        self.grown[0] += gained(start, drawn);
        self.grown[1] += gained(drawn, encoded);
    }
}

/// The video's memory after `frames` frames, logged: the process's resident memory and what it
/// gained while drawing and while encoding, the device's allocations and blocks, the target's
/// bytes and wgpu's live objects (what must stay flat frame after frame).
fn note(device: &VideoDevice, frames: u32, spent: &Spent) {
    let gpu = device.core.gpu_memory().unwrap_or_default();
    log::info!(
        "video memory at frame {frames}: resident {:.1} MB, {:+.1} while drawing and {:+.1} \
         while encoding since it began; its device's allocator {:.1} MB in blocks of {:.1}, the \
         target {:.1}; wgpu objects {:?}",
        mb(resident().unwrap_or(0).cast_signed()),
        mb(spent.grown[0]),
        mb(spent.grown[1]),
        mb(gpu.allocated.cast_signed()),
        mb(gpu.reserved.cast_signed()),
        mb(device.target.bytes().total().cast_signed()),
        device.core.gpu_objects().unwrap_or_default()
    );
}

/// Every frame of `frames` drawn on the video's devices within `bounds` and handed to `encoder`,
/// `shared` counting them and the target's bytes, until done or Cancel.
fn encode(
    core: &Core,
    ((order, frames, ends), bounds): ((&VideoOrder, &VideoFrames, &mut Ends), VideoBounds),
    encoder: &mut dyn VideoEncoder,
    shared: &VideoShared,
) -> Result<Spent, VideoError> {
    let canvas = frames.drawing().artwork().canvas();
    let sizes = ([canvas.width(), canvas.height()], order.size);
    let mut device = VideoDevice::open(core, sizes, bounds.arena)?;
    shared
        .held
        .store(device.target.bytes().total(), Ordering::Release);
    let fps = u64::from(order.fps.max(1));
    let mut spent = Spent::default();
    for index in 0..frames.count() {
        if shared.cancel.load(Ordering::Acquire) {
            return Err(VideoError::Cancelled);
        }
        if device.spent(&bounds) {
            device = device.made_anew(core, sizes, bounds.arena)?;
        }
        let start = (Instant::now(), resident());
        let VideoDevice {
            core: own, target, ..
        } = &mut device;
        let image = match frames.frame(index) {
            VideoFrame::At(at) => drawn_at(own, target, frames, at)?,
            VideoFrame::End => ends.end(own, target, frames.drawing())?,
        };
        let drawn = (Instant::now(), resident());
        encoder.frame(image.pixels(), u64::from(index) * 1_000_000 / fps)?;
        spent.add([start, drawn, (Instant::now(), resident())]);
        device.drew(gained(start, drawn));
        shared
            .held
            .store(device.target.bytes().total(), Ordering::Release);
        shared.made.store(index + 1, Ordering::Release);
        if (index + 1).is_multiple_of(NOTE_EVERY) {
            note(&device, index + 1, &spent);
        }
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

fn rendering(error: RenderError) -> VideoError {
    log::error!("a frame of the video could not be drawn: {error}");
    VideoError::Render
}
