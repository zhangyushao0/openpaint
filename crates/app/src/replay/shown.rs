//! What the replay's screen reads (S163): Actions' row before it opens, then the bar, the export
//! panel and the notices of the video being made. Every time is in milliseconds of the video at
//! the speed chosen (the bar's speed is the video's), every place on the track a share of it from
//! 0 (blank paper) to 1 (the finished artwork).

use openpaint_core::CanvasSize;

use crate::projection::Availability;
use crate::replay::video::VideoError;

/// The replay as the canvas's projection carries it (`SessionProjection::with_replay`).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ReplayShown {
    /// No replay here (a mode's sheet): Actions shows no row.
    #[default]
    None,
    /// Actions' row under Share: how many strokes the artwork's record holds
    /// (`Artwork::stroke_count`), and whether it opens now (nothing to replay on an artwork
    /// without a record; refused without a change of look while a stroke is drawn).
    Offered {
        strokes: usize,
        availability: Availability,
    },
    /// The replay is open: the canvas shows its frame and its chrome only.
    Open(Box<ReplayProjection>),
}

/// The replay's screen for one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayProjection {
    /// It plays: the bar's button shows Pause (else Play).
    pub playing: bool,
    pub speed: Speed,
    /// The playhead and the whole, in ms of the video at the speed chosen ("1:24 / 3:18").
    pub at_ms: u64,
    pub total_ms: u64,
    /// The playhead on the track.
    pub at: f32,
    /// Where each sitting after the first began on the track (a small mark each).
    pub sittings: Vec<f32>,
    /// The stroke at the playhead (the last begun), none before the first.
    pub stroke: Option<StrokeLabel>,
    /// The pen's tip while a stroke is drawn on screen: its last sample drawn, canvas pixels
    /// (a ring over the canvas, through the view).
    pub tip: Option<[f32; 2]>,
    /// The export panel's choices and what each would make.
    pub export: ExportChoices,
    /// The video being made, or just made.
    pub video: VideoNotice,
    /// Back, Export video…, Play, the track, the handles, the speeds and the panel's choices:
    /// `Running` while a video is made (only the notice's Cancel then), else `Available`.
    pub controls: Availability,
}

/// The stroke the bar names over the track ("14 Nov 21:07 · stroke 412").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeLabel {
    /// Its number among the record's strokes in the drawing order, from 1.
    pub number: usize,
    /// When it began, ms since 1970 UTC; `None` for a stroke kept before format 2 (no date).
    pub began_ms: Option<u64>,
}

/// The export panel: the length, the size, the end, and what each choice makes.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportChoices {
    pub length: VideoLength,
    /// The video's whole length for each of `VideoLength::ALL`, the end held included, in ms.
    pub lengths_ms: [u64; 3],
    /// The segment's ends on the track and in ms of the bar's time; the handles show on the
    /// track while the panel shows Segment chosen.
    pub segment: [f32; 2],
    pub segment_ms: [u64; 2],
    pub size: VideoSize,
    /// The video's pixels for each of `VideoSize::ALL` (`video_size`; the canvas's own `None`
    /// beyond what the encoder takes, the choice greyed).
    pub sizes: [Option<[u32; 2]>; 2],
    /// The finished picture held at the end (`HOLD_MS`).
    pub hold: bool,
    /// The line of strokes, layers, time and dates over the held picture; offered only with the
    /// hold.
    pub caption: bool,
}

/// What the notice in the top band says of the video.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VideoNotice {
    #[default]
    None,
    /// Being made: the pill with its progress and Cancel.
    Working { percent: u8 },
    /// Saved in the album: its length in seconds, with Share… and OK.
    Saved { seconds: u32 },
    /// It could not be made: why, with OK.
    Failed(VideoError),
}

/// How fast the replay plays: the speed chosen is the video's too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Speed {
    One,
    #[default]
    Four,
    Sixteen,
}

impl Speed {
    pub const ALL: [Self; 3] = [Self::One, Self::Four, Self::Sixteen];

    /// Replay milliseconds per millisecond shown.
    #[must_use]
    pub const fn factor(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Four => 4,
            Self::Sixteen => 16,
        }
    }
}

/// How much of the replay the video holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoLength {
    /// The whole replay in `HIGHLIGHT_MS`, the end held inside it, whatever the speed.
    #[default]
    Highlight,
    /// The whole replay at the speed chosen.
    Whole,
    /// The segment between the track's two handles at the speed chosen.
    Segment,
}

impl VideoLength {
    pub const ALL: [Self; 3] = [Self::Highlight, Self::Whole, Self::Segment];
}

/// The video's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoSize {
    /// The canvas within 1920 × 1080 (1080 × 1920 standing), never larger than the canvas.
    #[default]
    Hd,
    /// The canvas's own size, where the encoder takes it.
    Canvas,
}

impl VideoSize {
    pub const ALL: [Self; 2] = [Self::Hd, Self::Canvas];
}

/// The video's pixels for `canvas` at `size`: the canvas's aspect, each side even (H.264 halves
/// the colours); `Canvas` only within H.264 level 5.1's frame (sides up to 4096, at most
/// 4096 × 2304 pixels), else `None`.
#[must_use]
pub fn video_size(canvas: CanvasSize, size: VideoSize) -> Option<[u32; 2]> {
    // S163a: the rule above.
    let _ = size;
    let even = |side: u32| (side & !1).max(2);
    Some([even(canvas.width()), even(canvas.height())])
}
