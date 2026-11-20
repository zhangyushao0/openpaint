//! The replay as a video file (S163): the encoder the platform lends (a capability port, as the
//! store is), the order it is made to, and the frames the video worker draws. An H.264 MP4 at
//! `VIDEO_FPS`, saved in the album (Android: Movies/OpenPaint) or made for the share sheet.
//!
//! The frames (`VideoFrames`): frame `k` of the drawing shows the replay at
//! `from + k × rate × 1000 / VIDEO_FPS` ms of the timeline, the last drawing frame exactly at `to`
//! (`rate`: the speed chosen, or for the highlight the whole fitted in `HIGHLIGHT_MS` less the
//! hold, never slower than 1×); then, with the hold, `HOLD_MS` of frames of the end
//! (`VideoFrame::End`): the finished picture (a segment's too, as "Stop on the finished picture"
//! says), or with the caption the card: the desk, the picture fitted above the caption band, the
//! line under it (the worker draws it with the caption the UI lays, `openpaint_ui::video_caption`).
//!
//! The encoder's calls come from the video worker's thread, in order: `begin`, `frame` for each
//! frame, then `finish`, or `cancel` at any point after `begin` (also after a failed call): what a
//! cancelled or failed video began is removed, nothing half made stays in the album.

use std::fmt;
use std::sync::Arc;

use openpaint_core::DrawingOrder;

use crate::project::PortError;
use crate::replay::timeline::{At, ReplayFacts, Timeline};
use crate::title::Title;
use crate::transfer::Delivery;

/// Frames per second of every video.
pub const VIDEO_FPS: u32 = 30;
/// How long the end is held when the hold is on.
pub const HOLD_MS: u64 = 3_000;
/// The highlight's whole length, the hold inside it.
pub const HIGHLIGHT_MS: u64 = 30_000;

/// The platform's H.264 encoder writing one MP4 file (Android: `MediaCodec` and `MediaMuxer`;
/// the harness keeps what it is given, `openpaint_devtools`).
///
/// # Errors
/// `Encoder` when the encoder refuses the size or fails; `Write` when the file cannot be
/// written (storage full).
pub trait VideoEncoder: Send {
    /// Opens the file for `order` (`Delivery::Save`: the album's Movies/OpenPaint, pending until
    /// finished; `Share`: the platform's place for files it shares) and the encoder for frames of
    /// `order.size` at `order.fps`.
    fn begin(&mut self, order: &VideoOrder) -> Result<(), VideoError>;
    /// One frame: opaque sRGB RGBA8 rows of `order.size`, top first; `at_us` its time from the
    /// video's start (frame `k`: `k × 1_000_000 / fps`).
    fn frame(&mut self, rgba: &[u8], at_us: u64) -> Result<(), VideoError>;
    /// Ends the video: the file complete and, in the album, shown there.
    fn finish(&mut self) -> Result<(), VideoError>;
    /// Drops the video: nothing of it stays.
    fn cancel(&mut self);
}

/// What a video is made to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoOrder {
    /// The file's name: the artwork's title as a file name, `.mp4` (`video_file_name`).
    pub name: String,
    /// Pixels, each side even (`video_size`).
    pub size: [u32; 2],
    pub fps: u32,
    pub delivery: Delivery,
}

/// Why a video was not made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoError {
    /// The user's Cancel: nothing is told.
    Cancelled,
    /// The GPU could not draw a frame.
    Render,
    /// The platform's encoder refused the size or failed.
    Encoder(PortError),
    /// The file could not be written.
    Write(PortError),
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("the video was cancelled"),
            Self::Render => f.write_str("a frame of the video could not be drawn"),
            Self::Encoder(error) => write!(f, "the video encoder failed: {error}"),
            Self::Write(error) => write!(f, "the video could not be written: {error}"),
        }
    }
}

/// One video for the video worker: what it shows, where it goes and the encoder that makes it.
pub struct VideoJob {
    pub order: VideoOrder,
    pub frames: VideoFrames,
    pub encoder: Box<dyn VideoEncoder>,
    /// The numbers of the end card's line, when the caption is on: the composition root has the
    /// UI word and lay them (`openpaint_ui::video_caption`) before the job runs.
    pub caption: Option<ReplayFacts>,
}

impl fmt::Debug for VideoJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoJob")
            .field("order", &self.order)
            .field("frames", &self.frames.count())
            .finish_non_exhaustive()
    }
}

/// What every frame of a video shows (see the module).
#[derive(Debug, Clone)]
pub struct VideoFrames {
    drawing: DrawingOrder,
    timeline: Arc<Timeline>,
    /// The timeline's range shown, ms at 1×.
    range: [u64; 2],
    /// Timeline ms per ms of video.
    rate: f64,
    /// Frames of the end held after the drawing.
    held: u32,
}

/// What one frame of a video shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFrame {
    /// The replay at one moment (`DrawingOrder::frame(done, samples)`).
    At(At),
    /// The end held: the finished picture (a segment's too), or the card with the caption.
    End,
}

impl VideoFrames {
    /// The frames of `range` of `timeline` (ms at 1×) at `rate` timeline ms per ms of video,
    /// then `held` frames of the end.
    #[must_use]
    pub fn new(
        (drawing, timeline): (DrawingOrder, Arc<Timeline>),
        range: [u64; 2],
        rate: f64,
        held: u32,
    ) -> Self {
        Self {
            drawing,
            timeline,
            range,
            rate,
            held,
        }
    }

    /// How many frames the video has, the end's included.
    #[must_use]
    pub fn count(&self) -> u32 {
        drawn_frames(self.range, self.rate).saturating_add(self.held)
    }

    /// What frame `index` shows.
    #[must_use]
    pub fn frame(&self, index: u32) -> VideoFrame {
        let drawn = drawn_frames(self.range, self.rate);
        if index >= drawn {
            return VideoFrame::End;
        }
        let [from, to] = self.range;
        let ms = if index + 1 == drawn {
            to
        } else {
            let since = f64::from(index) * frame_ms(self.rate);
            (from + since.round() as u64).min(to)
        };
        VideoFrame::At(self.timeline.at(&self.drawing, ms))
    }

    /// The drawing order the frames show.
    #[must_use]
    pub const fn drawing(&self) -> &DrawingOrder {
        &self.drawing
    }
}

/// The video file's name for `title`: the title as `file_name` makes file names, `.mp4`.
#[must_use]
pub fn video_file_name(title: &Title) -> String {
    crate::export::named(title, "mp4")
}

/// Frames of the end held, with the hold on.
pub(super) const HOLD_FRAMES: u32 = (HOLD_MS * VIDEO_FPS as u64 / 1000) as u32;

/// Timeline ms between two frames at `rate` (a rate that is no number or not above 0 is 1×).
fn frame_ms(rate: f64) -> f64 {
    let rate = if rate > 0.0 { rate } else { 1.0 };
    rate * 1000.0 / f64::from(VIDEO_FPS)
}

/// How many frames draw `range` (timeline ms) at `rate`: one every `frame_ms`, the last exactly
/// at its end (one frame for an empty range); a millionth of a gap is rounding, not a frame.
pub(super) fn drawn_frames([from, to]: [u64; 2], rate: f64) -> u32 {
    let gaps = (to.saturating_sub(from) as f64 / frame_ms(rate) - 1e-6).ceil();
    (gaps.max(0.0) as u32).saturating_add(1)
}

/// The rate fitting the whole of `total_ms` (timeline ms) into the highlight, the end held
/// inside it when `hold`: never slower than 1×.
pub(super) fn highlight_rate(total_ms: u64, hold: bool) -> f64 {
    let held = if hold { HOLD_FRAMES } else { 0 };
    let room = (HIGHLIGHT_MS * u64::from(VIDEO_FPS) / 1000) as u32 - held;
    let room_ms = f64::from(room.saturating_sub(1)) * 1000.0 / f64::from(VIDEO_FPS);
    (total_ms as f64 / room_ms).max(1.0)
}
