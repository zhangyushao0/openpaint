//! The replay's screen and its video's last line (S163; the author's mock of six screens,
//! 2026-11-20), read from `ReplayProjection` (`SessionProjection::replay`), its taps
//! `Intent::Replay`. While the replay is open the canvas shows its frame at fit between the top
//! band and the bar, on the desk, and nothing of the studio's chrome: the left island holds Back,
//! the right island Export video… (ringed while its panel is open), the bar floats under the
//! canvas, a notice is a pill in the top band left of the right island; pen and fingers on the
//! canvas do nothing; the system's back is Back.
//!
//! The bar, left to right: Play or Pause (an icon in a ring, named by its English); over the
//! track, the stroke at the playhead (its local date and time when it has one, then its number)
//! at the left, the playhead and the whole at the right ("1:24 / 3:18"), or while Segment is
//! chosen in the open panel the segment and its length ("0:52 – 1:40 · 48 s"); the track: what
//! played in ink, a small mark where each sitting after the first began, the knob (tap or drag:
//! `Seek`), or the segment in ink between two handles (drag: `Segment`); the speeds 1×, 4×, 16×,
//! the one chosen in the ink pill. Over the canvas, while a stroke is drawn, a thin ring at the
//! pen's tip.
//!
//! The export panel, a popover under the right island: its heading; Length: 30 s highlight,
//! Full, Segment, each a choice row with what it lasts (Segment its range); Size: 1080p and the
//! canvas's own ("Original 2880 × 1800"), greyed where the encoder cannot take it; End: two
//! switches, the finished picture held and the line of strokes and time (offered with the first
//! on); Save to album, Share…. While the video is made: the pill "Exporting video", its progress
//! line and percent, Cancel; every other control greyed. Saved: "Video saved to album · 48 s",
//! Share… and OK. Failed: the failure and OK.
//!
//! Words: `text/replay.rs` (D100); no sentence says how the replay or the video is made (D118).

mod caption;

pub use caption::{caption_band, video_caption};
