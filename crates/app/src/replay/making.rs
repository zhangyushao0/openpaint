//! The video being made (see the module): asked for, made with its progress, cancelled, saved,
//! shared or failed, what the notice says of it, and the replay's calls that make it.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::error::AppError;
use crate::project::PortError;
use crate::projection::Availability;
use crate::replay::shown::{VideoLength, VideoNotice, video_size};
use crate::replay::video::{
    HOLD_FRAMES, VIDEO_FPS, VideoError, VideoFrames, VideoJob, VideoOrder, drawn_frames,
    highlight_rate,
};
use crate::replay::{Replay, refused};
use crate::transfer::{Delivery, HostAnswer, HostRequest};

impl Replay {
    /// An encoder is asked for and not answered yet.
    #[must_use]
    pub const fn waiting(&self) -> bool {
        matches!(self.making, Making::Asked(_))
    }

    /// The platform's answer while `waiting`: an encoder makes the video job for the video worker;
    /// a failure is the failed notice. An answer nothing waits for is dropped (an encoder asked
    /// for and cancelled at once).
    pub fn answer(&mut self, answer: HostAnswer) -> Option<VideoJob> {
        let Making::Asked(order) = &self.making else {
            return None;
        };
        let order = order.clone();
        self.making = match answer {
            HostAnswer::Video(encoder) => {
                let frames = self.frames();
                self.making = Making::working(order.clone(), frames.count());
                let caption = self.choices.hold && self.choices.caption;
                return Some(VideoJob {
                    order,
                    frames,
                    encoder,
                    caption: caption.then(|| self.timeline.facts()),
                });
            }
            HostAnswer::Cancelled => Making::Idle,
            HostAnswer::Failed(error) => Making::refused(Some(error)),
            HostAnswer::File(_) | HostAnswer::Image(_) => Making::refused(None),
        };
        None
    }

    /// The video worker made `frames` of the video's frames so far.
    pub const fn progress(&mut self, frames: u32) {
        self.making.progress(frames);
    }

    /// The video job ended: saved, shared (the share sheet next), cancelled or failed.
    pub fn finished(&mut self, result: Result<(), VideoError>) {
        let making = std::mem::take(&mut self.making);
        self.making = making.finished(result, &mut self.requests);
    }

    /// Asks the host for an encoder for a video of the choices, delivered by `delivery`; the
    /// replay pauses.
    pub(super) fn export(&mut self, delivery: Delivery) -> Result<(), AppError> {
        let size = video_size(self.canvas, self.choices.size);
        let size = size.ok_or(refused(Availability::NothingLeft))?;
        let name = self.name.clone();
        self.requests.push_back(HostRequest::Video {
            name: name.clone(),
            delivery,
        });
        self.making = Making::Asked(VideoOrder {
            name,
            size,
            fps: VIDEO_FPS,
            delivery,
        });
        self.playing = false;
        Ok(())
    }

    /// The frames of the video the choices make.
    fn frames(&self) -> VideoFrames {
        let (range, rate) = self.range_and_rate(self.choices.length);
        let made = (self.drawing.clone(), Arc::clone(&self.timeline));
        VideoFrames::new(made, range, rate, self.held())
    }

    /// The timeline's range a video of `length` shows and its rate (timeline ms per ms).
    fn range_and_rate(&self, length: VideoLength) -> ([u64; 2], f64) {
        let total = self.timeline.total_ms();
        let speed = f64::from(self.speed.factor());
        match length {
            VideoLength::Highlight => ([0, total], highlight_rate(total, self.choices.hold)),
            VideoLength::Whole => ([0, total], speed),
            VideoLength::Segment => (self.segment(), speed),
        }
    }

    /// Frames of the end held after the drawing.
    const fn held(&self) -> u32 {
        if self.choices.hold { HOLD_FRAMES } else { 0 }
    }

    /// How long a video of `length` lasts, the end held included, in ms.
    pub(super) fn video_ms(&self, length: VideoLength) -> u64 {
        let (range, rate) = self.range_and_rate(length);
        let frames = drawn_frames(range, rate).saturating_add(self.held());
        u64::from(frames) * 1000 / u64::from(VIDEO_FPS)
    }
}

/// The video's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) enum Making {
    #[default]
    Idle,
    /// Its encoder is asked of the host (`HostRequest::Video`).
    Asked(VideoOrder),
    /// The video worker makes `frames` frames; `made` so far.
    Working {
        order: VideoOrder,
        frames: u32,
        made: u32,
    },
    /// Cancelled while being made: the worker stops between frames and tells (`finished`).
    Cancelling {
        order: VideoOrder,
        frames: u32,
    },
    /// In the album: its file's name and length.
    Saved {
        name: String,
        seconds: u32,
    },
    Failed(VideoError),
}

impl Making {
    /// A video is asked for or being made.
    pub(super) const fn busy(&self) -> bool {
        matches!(
            self,
            Self::Asked(_) | Self::Working { .. } | Self::Cancelling { .. }
        )
    }

    /// What the notice in the top band says.
    pub(super) fn notice(&self) -> VideoNotice {
        match self {
            Self::Idle | Self::Cancelling { .. } => VideoNotice::None,
            Self::Asked(_) => VideoNotice::Working { percent: 0 },
            Self::Working { frames, made, .. } => {
                let percent = u64::from(*made) * 100 / u64::from((*frames).max(1));
                VideoNotice::Working {
                    percent: u8::try_from(percent.min(100)).unwrap_or(100),
                }
            }
            Self::Saved { seconds, .. } => VideoNotice::Saved { seconds: *seconds },
            Self::Failed(error) => VideoNotice::Failed(error.clone()),
        }
    }

    /// The host's encoder came for `order`: the worker makes its `frames` frames.
    pub(super) const fn working(order: VideoOrder, frames: u32) -> Self {
        Self::Working {
            order,
            frames,
            made: 0,
        }
    }

    /// The host had no encoder: `error`, or a picker's answer that is none.
    pub(super) fn refused(error: Option<PortError>) -> Self {
        let error = error.unwrap_or_else(|| PortError::Device {
            detail: "something else came back for a video's encoder".into(),
        });
        Self::Failed(VideoError::Encoder(error))
    }

    /// Cancel: one asked for is dropped at once, one being made once the worker stops.
    pub(super) fn cancelled(self) -> Self {
        match self {
            Self::Working { order, frames, .. } => Self::Cancelling { order, frames },
            Self::Asked(_) => Self::Idle,
            other => other,
        }
    }

    /// The worker made `made` frames.
    pub(super) const fn progress(&mut self, made: u32) {
        if let Self::Working {
            frames,
            made: so_far,
            ..
        } = self
        {
            *so_far = if made < *frames { made } else { *frames };
        }
    }

    /// The worker ended with `result`: saved (the notice), shared (the share sheet next, in
    /// `requests`), cancelled (nothing told) or failed (the notice). A video finished although
    /// cancelled is told as made: it is in the album or on its way to the share sheet.
    pub(super) fn finished(
        self,
        result: Result<(), VideoError>,
        requests: &mut VecDeque<HostRequest>,
    ) -> Self {
        let (order, frames, cancelled) = match self {
            Self::Working { order, frames, .. } => (order, frames, false),
            Self::Cancelling { order, frames } => (order, frames, true),
            other => return other,
        };
        match result {
            Ok(()) => match order.delivery {
                Delivery::Save => Self::Saved {
                    name: order.name,
                    seconds: (frames + VIDEO_FPS / 2) / VIDEO_FPS,
                },
                Delivery::Share => {
                    requests.push_back(HostRequest::SendVideo { name: order.name });
                    Self::Idle
                }
            },
            Err(VideoError::Cancelled) => Self::Idle,
            Err(_) if cancelled => Self::Idle,
            Err(error) => Self::Failed(error),
        }
    }
}
