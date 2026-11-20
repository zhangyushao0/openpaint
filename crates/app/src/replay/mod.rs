//! The replay of a studio artwork and its video (S163; the author's mock of six screens,
//! 2026-11-20). `Replay` is the flow; the composition root holds one beside the open artwork
//! while it shows (the session and its history stay as they are under it), draws its frame on
//! the canvas, ticks it with the frame clock, hands its video job to the video worker and its
//! requests to the host. Pure: no clock of its own, no thread.
//!
//! The flow, which the screens, the runtime and the host keep alike:
//! - Open (Actions › Share › Replay…, `ReplayIntent::Open`, offered while the record holds a step,
//!   `EditorAction::Replay`): what an edit ends ends first (a transform committed, an
//!   adjustment applied, as leaving does); the replay opens on blank paper, playing from the
//!   start at the speed last chosen in the app's run (4× at first), the canvas at fit between
//!   the top band and the bar; the studio's islands, side bar and panels go, Back and Export
//!   video… stay. Pen and fingers on the canvas do nothing.
//! - Play and Pause: the playhead moves by the frame clock's time times the speed; it stops at
//!   the end on the finished artwork (Pause shows Play); Play at the end starts again.
//! - The track: a tap or drag moves the playhead there (`Seek`), playing or not; the steps done
//!   and the stroke drawn so far follow at once.
//! - Speeds 1×, 4×, 16×: how fast it plays, and the video's speed (Whole and Segment).
//! - Export video… opens its panel under the island: the length (the 30 s highlight, the whole,
//!   a segment: two handles on the track, from the playhead to the end when first chosen), the
//!   size (1080p, the canvas's own where the encoder takes it), the end (the finished picture
//!   held, and with it the line of strokes and time), Save to album and Share…. The choices stay
//!   while the replay is open.
//! - Making the video (`Export`): the host lends an encoder (`HostRequest::Video`, answered at
//!   once with `HostAnswer::Video`); the video worker draws and encodes every frame; the
//!   notice's pill shows the progress and Cancel; every other control waits (`Running`), the
//!   replay pauses. Cancel: the worker stops between frames and the encoder removes what it began;
//!   nothing is told. Saved: "Video saved to album · 48 s" with Share… (the share sheet for that
//!   video, `HostRequest::SendVideo`) and OK. Shared: the share sheet at once, no notice. Failed:
//!   the notice with OK.
//! - Back (and the system's back) closes it: the canvas shows the artwork as it was, the studio's
//!   chrome returns; refused while a video is made. Leaving the app pauses it.

use std::collections::VecDeque;
use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, DrawingOrder, ReplayFrame};

use crate::error::AppError;
use crate::projection::{Availability, EditorAction};
use crate::title::Title;
use crate::transfer::{Delivery, HostAnswer, HostRequest};

mod shown;
mod timeline;
mod video;

pub use shown::{
    ExportChoices, ReplayProjection, ReplayShown, Speed, StrokeLabel, VideoLength, VideoNotice,
    VideoSize, video_size,
};
pub use timeline::{At, EDIT_MS, LEAST_STROKE_MS, PAUSE_MS, ReplayFacts, SITTING_MS, Timeline};
pub use video::{
    HIGHLIGHT_MS, HOLD_MS, VIDEO_FPS, VideoEncoder, VideoError, VideoFrame, VideoFrames, VideoJob,
    VideoOrder, video_file_name,
};

/// What the user asks of the replay. `Open` is Actions' and goes through the session (what it
/// ends, whether it is offered) before the composition root makes the `Replay`; the rest are the
/// replay's own (`Replay::dispatch`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplayIntent {
    /// Actions › Replay….
    Open,
    /// Back: the replay closes.
    Close,
    /// The bar's Play (true) or Pause (false).
    Play(bool),
    /// The playhead to a place on the track (0 to 1).
    Seek(f32),
    Speed(Speed),
    /// The export panel's choices.
    Length(VideoLength),
    /// The segment's ends on the track (0 to 1, the first before the second), as the handles
    /// left them.
    Segment([f32; 2]),
    Size(VideoSize),
    Hold(bool),
    Caption(bool),
    /// Save to album (`Delivery::Save`) or Share… (`Delivery::Share`): the video is made.
    Export(Delivery),
    /// The notice's Cancel while the video is made.
    Cancel,
    /// The saved notice's Share…: the video just saved, to the share sheet.
    ShareSaved,
    /// The notice's OK.
    Dismiss,
}

/// The export panel's choices as they stand.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Choices {
    length: VideoLength,
    /// The segment's ends, ms of the timeline at 1×.
    segment: [u64; 2],
    size: VideoSize,
    hold: bool,
    caption: bool,
}

/// The video's state (S163a fills its states: asked, being made with its progress, saved,
/// failed).
#[derive(Debug, Clone, PartialEq, Default)]
struct Making {
    notice: VideoNotice,
}

/// The replay of one artwork (see the module).
#[derive(Debug)]
pub struct Replay {
    drawing: DrawingOrder,
    timeline: Arc<Timeline>,
    canvas: CanvasSize,
    /// The video file's name.
    name: String,
    /// The playhead, ms of the timeline at 1×.
    at_ms: u64,
    playing: bool,
    speed: Speed,
    /// The frame clock's ms at the last tick.
    ticked: Option<u64>,
    /// What the canvas shows at the playhead, made again only when it moves past a sample.
    shown: ReplayFrame,
    choices: Choices,
    making: Making,
    requests: VecDeque<HostRequest>,
}

impl Replay {
    /// The replay of `artwork` (the open session's current version) titled `title`, playing
    /// from blank paper at `speed` (the one last chosen in the app's run).
    #[must_use]
    pub fn open(artwork: &Artwork, title: &Title, speed: Speed) -> Self {
        let drawing = artwork.drawing_order();
        let timeline = Arc::new(Timeline::of(&drawing));
        // S163a: blank paper at first (`drawing.frame(0, 0)`).
        let shown = drawing.frame(drawing.steps().len(), 0);
        let end = timeline.total_ms();
        Self {
            canvas: artwork.canvas(),
            name: video_file_name(title),
            drawing,
            timeline,
            at_ms: 0,
            playing: true,
            speed,
            ticked: None,
            shown,
            choices: Choices {
                length: VideoLength::Highlight,
                segment: [0, end],
                size: VideoSize::Hd,
                hold: true,
                caption: true,
            },
            making: Making::default(),
            requests: VecDeque::new(),
        }
    }

    /// One of the replay's own intents (all but `Open`). `Close` answers whether it may close:
    /// the composition root then drops the replay.
    ///
    /// # Errors
    /// `Unavailable` (`EditorAction::Replay`, `Running`) for any but `Cancel` while a video is
    /// made; `NothingLeft` for a choice not offered (the canvas's own size beyond the encoder's,
    /// the caption without the hold).
    pub fn dispatch(&mut self, intent: &ReplayIntent) -> Result<(), AppError> {
        // S163a: every intent by the module's flow.
        if self.making() && *intent != ReplayIntent::Cancel {
            return Err(AppError::Unavailable {
                action: EditorAction::Replay,
                availability: Availability::Running,
            });
        }
        Ok(())
    }

    /// The frame clock at `now_ms`: playing moves the playhead by the time since the last tick
    /// times the speed, stopping at the end.
    pub fn tick(&mut self, now_ms: u64) {
        // S163a: the playhead and the frame at it.
        self.ticked = Some(now_ms);
    }

    /// It plays: the composition root wants a frame every vsync meanwhile.
    #[must_use]
    pub const fn playing(&self) -> bool {
        self.playing
    }

    /// A video is being made (or its encoder asked for): Back is refused and frames keep coming
    /// for its progress.
    #[must_use]
    pub fn making(&self) -> bool {
        matches!(self.making.notice, VideoNotice::Working { .. })
    }

    /// What the canvas shows now; the same version while the playhead stays within a sample.
    #[must_use]
    pub const fn frame(&self) -> &ReplayFrame {
        &self.shown
    }

    /// The replay's screen for this frame.
    #[must_use]
    pub fn projection(&self) -> ReplayProjection {
        // S163a: every field from the playhead, the timeline, the choices and the video.
        let speed = u64::from(self.speed.factor());
        let total_ms = self.timeline.total_ms() / speed;
        ReplayProjection {
            playing: self.playing,
            speed: self.speed,
            at_ms: self.at_ms / speed,
            total_ms,
            at: 0.0,
            sittings: Vec::new(),
            stroke: None,
            tip: None,
            export: ExportChoices {
                length: self.choices.length,
                lengths_ms: [HIGHLIGHT_MS, total_ms, total_ms],
                segment: [0.0, 1.0],
                segment_ms: self.choices.segment.map(|ms| ms / speed),
                size: self.choices.size,
                sizes: VideoSize::ALL.map(|size| video_size(self.canvas, size)),
                hold: self.choices.hold,
                caption: self.choices.caption,
            },
            video: self.making.notice.clone(),
            controls: if self.making() {
                Availability::Running
            } else {
                Availability::Available
            },
        }
    }

    /// The next thing to ask of the platform (an encoder for the video, the share sheet), once.
    pub fn request(&mut self) -> Option<HostRequest> {
        self.requests.pop_front()
    }

    /// An encoder is asked for and not answered yet.
    #[must_use]
    pub const fn waiting(&self) -> bool {
        // S163a: while the encoder is asked for.
        false
    }

    /// The platform's answer while `waiting`: an encoder makes the video job for the video worker;
    /// a failure is the failed notice.
    pub fn answer(&mut self, answer: HostAnswer) -> Option<VideoJob> {
        // S163a: the job of the choices, `VideoFrames` over the timeline.
        let _ = (answer, &self.drawing, &self.name, self.ticked);
        None
    }

    /// The video worker made `frames` of the video's frames so far.
    pub fn progress(&mut self, frames: u32) {
        // S163a: the pill's percent.
        let _ = frames;
    }

    /// The video job ended: saved, shared (the share sheet next), cancelled or failed.
    pub fn finished(&mut self, result: Result<(), VideoError>) {
        // S163a: the notice by the result and the delivery.
        let _ = result;
    }
}
