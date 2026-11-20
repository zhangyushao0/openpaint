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
//!
//! The parts: `timeline.rs` the time, `shown.rs` the screen's projection, `video.rs` the video's
//! port and frames, `making.rs` the video being made.

use std::collections::VecDeque;
use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, DrawingOrder, ReplayFrame, StepKind};

use crate::error::AppError;
use crate::projection::{Availability, EditorAction};
use crate::title::Title;
use crate::transfer::{Delivery, HostRequest};

mod making;
mod shown;
mod timeline;
mod video;

use making::Making;

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
    /// The segment's ends, ms of the timeline at 1×, once the handles were shown.
    segment: Option<[u64; 2]>,
    size: VideoSize,
    hold: bool,
    caption: bool,
}

/// The replay of one artwork (see the module).
#[derive(Debug)]
pub struct Replay {
    drawing: DrawingOrder,
    timeline: Arc<Timeline>,
    canvas: CanvasSize,
    /// The video file's name.
    name: String,
    /// The steps that are strokes, in order: a stroke's number is its place among them.
    strokes: Vec<usize>,
    /// The playhead, ms of the timeline at 1×.
    at_ms: u64,
    playing: bool,
    speed: Speed,
    /// The frame clock's ms at the last tick.
    ticked: Option<u64>,
    /// The moment shown, and what the canvas shows then: made again only when it changes.
    at: At,
    shown: ReplayFrame,
    choices: Choices,
    making: Making,
    requests: VecDeque<HostRequest>,
}

/// A replay intent refused for `availability`.
const fn refused(availability: Availability) -> AppError {
    AppError::Unavailable {
        action: EditorAction::Replay,
        availability,
    }
}

impl Replay {
    /// The replay of `artwork` (the open session's current version) titled `title`, playing
    /// from blank paper at `speed` (the one last chosen in the app's run).
    #[must_use]
    pub fn open(artwork: &Artwork, title: &Title, speed: Speed) -> Self {
        let drawing = artwork.drawing_order();
        let timeline = Arc::new(Timeline::of(&drawing));
        let strokes = (drawing.steps().iter().enumerate())
            .filter(|(_, step)| matches!(step.kind, StepKind::Stroke { .. }))
            .map(|(index, _)| index)
            .collect();
        Self {
            canvas: artwork.canvas(),
            name: video_file_name(title),
            shown: drawing.frame(0, 0),
            drawing,
            timeline,
            strokes,
            at_ms: 0,
            playing: true,
            speed,
            ticked: None,
            at: At::default(),
            choices: Choices {
                length: VideoLength::Highlight,
                segment: None,
                size: VideoSize::Hd,
                hold: true,
                caption: true,
            },
            making: Making::Idle,
            requests: VecDeque::new(),
        }
    }

    /// One of the replay's own intents (all but `Open`). `Close` answers whether it may close:
    /// the composition root then drops the replay.
    ///
    /// # Errors
    /// `Unavailable` (`EditorAction::Replay`, `Running`) for any but `Cancel` while a video is
    /// made; `NothingLeft` for a choice not offered (the canvas's own size beyond the encoder's,
    /// the caption without the hold) and for Cancel, Share… and OK with nothing to act on.
    pub fn dispatch(&mut self, intent: &ReplayIntent) -> Result<(), AppError> {
        if self.making() && *intent != ReplayIntent::Cancel {
            return Err(refused(Availability::Running));
        }
        let none = refused(Availability::NothingLeft);
        match *intent {
            ReplayIntent::Open | ReplayIntent::Close => {}
            ReplayIntent::Play(play) => {
                if play && self.at_ms >= self.timeline.total_ms() {
                    self.at_ms = 0;
                    self.show();
                }
                self.playing = play;
            }
            ReplayIntent::Seek(place) => {
                self.at_ms = self.ms_at(place);
                self.show();
            }
            ReplayIntent::Speed(speed) => self.speed = speed,
            ReplayIntent::Length(length) => {
                if length == VideoLength::Segment {
                    self.choices.segment = Some(self.segment());
                }
                self.choices.length = length;
            }
            ReplayIntent::Segment([start, end]) => {
                let [start, end] = [start.min(end), start.max(end)].map(|end| self.ms_at(end));
                self.choices.segment = Some([start, end]);
            }
            ReplayIntent::Size(size) => {
                video_size(self.canvas, size).ok_or(none)?;
                self.choices.size = size;
            }
            ReplayIntent::Hold(hold) => self.choices.hold = hold,
            ReplayIntent::Caption(caption) if caption && !self.choices.hold => return Err(none),
            ReplayIntent::Caption(caption) => self.choices.caption = caption,
            ReplayIntent::Export(delivery) => self.export(delivery)?,
            ReplayIntent::Cancel if !self.making() => return Err(none),
            ReplayIntent::Cancel => self.making = std::mem::take(&mut self.making).cancelled(),
            ReplayIntent::ShareSaved => {
                let Making::Saved { name, .. } = &self.making else {
                    return Err(none);
                };
                let name = name.clone();
                self.requests.push_back(HostRequest::SendVideo { name });
                self.making = Making::Idle;
            }
            ReplayIntent::Dismiss => match self.making {
                Making::Saved { .. } | Making::Failed(_) => self.making = Making::Idle,
                _ => return Err(none),
            },
        }
        Ok(())
    }

    /// The frame clock at `now_ms`: playing moves the playhead by the time since the last tick
    /// times the speed, stopping at the end.
    pub fn tick(&mut self, now_ms: u64) {
        let last = self.ticked.replace(now_ms);
        let Some(last) = last.filter(|_| self.playing) else {
            return;
        };
        let total = self.timeline.total_ms();
        let moved = now_ms.saturating_sub(last) * u64::from(self.speed.factor());
        self.at_ms = self.at_ms.saturating_add(moved).min(total);
        self.playing = self.at_ms < total;
        self.show();
    }

    /// It plays: the composition root wants a frame every vsync meanwhile.
    #[must_use]
    pub const fn playing(&self) -> bool {
        self.playing
    }

    /// A video is being made (or its encoder asked for): Back is refused and frames keep coming
    /// for its progress.
    #[must_use]
    pub const fn making(&self) -> bool {
        self.making.busy()
    }

    /// What the canvas shows now; the same version while the playhead stays within a sample.
    #[must_use]
    pub const fn frame(&self) -> &ReplayFrame {
        &self.shown
    }

    /// The replay's screen for this frame.
    #[must_use]
    pub fn projection(&self) -> ReplayProjection {
        let speed = u64::from(self.speed.factor());
        let total = self.timeline.total_ms();
        let share = |ms: u64| {
            if total == 0 {
                1.0
            } else {
                (ms as f64 / total as f64) as f32
            }
        };
        let segment = self.segment();
        let tip = (self.shown.active.as_ref())
            .and_then(|active| active.samples().last())
            .map(|sample| [sample.position().x(), sample.position().y()]);
        ReplayProjection {
            playing: self.playing,
            speed: self.speed,
            at_ms: self.at_ms / speed,
            total_ms: total / speed,
            at: share(self.at_ms),
            sittings: self
                .timeline
                .sittings()
                .iter()
                .map(|ms| share(*ms))
                .collect(),
            stroke: self.stroke(),
            tip,
            export: ExportChoices {
                length: self.choices.length,
                lengths_ms: VideoLength::ALL.map(|length| self.video_ms(length)),
                segment: segment.map(share),
                segment_ms: segment.map(|ms| ms / speed),
                size: self.choices.size,
                sizes: VideoSize::ALL.map(|size| video_size(self.canvas, size)),
                hold: self.choices.hold,
                caption: self.choices.caption,
            },
            video: self.making.notice(),
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

    /// The segment's ends: as the handles left them, else from the playhead to the end (the
    /// whole from the end).
    fn segment(&self) -> [u64; 2] {
        let total = self.timeline.total_ms();
        let first = if self.at_ms < total { self.at_ms } else { 0 };
        self.choices.segment.unwrap_or([first, total])
    }

    /// The timeline's ms at `place` on the track (0 to 1).
    fn ms_at(&self, place: f32) -> u64 {
        let place = if place.is_nan() {
            0.0
        } else {
            place.clamp(0.0, 1.0)
        };
        (f64::from(place) * self.timeline.total_ms() as f64).round() as u64
    }

    /// The canvas at the playhead, made again when the moment shown changes.
    fn show(&mut self) {
        let at = self.timeline.at(&self.drawing, self.at_ms);
        if at != self.at {
            self.at = at;
            self.shown = self.drawing.frame(at.done, at.samples);
        }
    }

    /// The stroke at the playhead: the last begun, by its number among the record's strokes.
    fn stroke(&self) -> Option<StrokeLabel> {
        let begun = self.at.done + usize::from(self.at.samples > 0);
        let number = self.strokes.partition_point(|step| *step < begun);
        let step = self.strokes.get(number.checked_sub(1)?)?;
        let began_ms = match self.drawing.steps()[*step].kind {
            StepKind::Stroke { began_ms, .. } => began_ms,
            StepKind::Edit(_) => None,
        };
        Some(StrokeLabel { number, began_ms })
    }
}
