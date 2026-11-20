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
//! The parts: `bar.rs`, `panel.rs`, `notice.rs`, the video's last line `caption.rs`.

use egui::Rect;
use openpaint_app::{CanvasView, Intent, ReplayIntent, ReplayProjection, VideoLength, ViewMapping};
use openpaint_core::CanvasSize;
use openpaint_render::BrushCursor;

use crate::components::{
    CanvasArea, Opener, Place, ScreenArea, TOP_BAND, choices, island, text_button, text_choice_wide,
};
use crate::dates::Date;
use crate::design::{SPACE, TOUCH_TARGET};
use crate::taps::push_once;
use crate::text::{self as t, language};

mod bar;
mod caption;
#[cfg(test)]
mod export_tests;
mod notice;
mod panel;
#[cfg(test)]
mod screen_tests;

use bar::BAR_ROOM;
pub use caption::{caption_band, video_caption};

/// The replay's islands, which its panel grows out of.
const LEFT: &str = "replay left island";
const RIGHT: &str = "replay right island";
/// Radius and line of the ring at the pen's tip, in points.
const TIP_RADIUS: f32 = 0.75 * SPACE;
const TIP_LINE: f32 = 1.0;

/// The replay screen's UI-local state between passes.
#[derive(Debug, Default)]
pub(crate) struct ReplayScreen {
    /// Shown in this pass, and in the one before.
    open: bool,
    was: bool,
    /// The export panel is open.
    panel: bool,
    /// Where Export video… lies, in points, from this pass.
    button: Option<Rect>,
    /// Where the islands, the pill, the bar and the open panel lie, in pixels, from this pass:
    /// the canvas takes no contact there.
    areas: Vec<ScreenArea>,
    /// The canvas at fit in its room, and the pen's tip (canvas pixels), from this pass.
    view: Option<ViewMapping>,
    tip: Option<[f32; 2]>,
}

impl ReplayScreen {
    /// A pass begins: nothing of the replay shows until `show`.
    pub(crate) fn begin(&mut self) {
        self.was = self.open;
        self.open = false;
        self.view = None;
        self.tip = None;
    }

    /// The replay's chrome for `replay` over a canvas of `canvas` pixels shown through `view`, its
    /// dates in the device's zone (`utc_offset`, minutes); its taps go to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        (replay, canvas, utc_offset): (&ReplayProjection, Option<CanvasSize>, i32),
        view: Option<ViewMapping>,
        taps: &mut Vec<Intent>,
    ) {
        if !self.was {
            // Each opening starts with its panel closed.
            self.panel = false;
        }
        (self.open, self.view, self.tip) = (true, view, replay.tip);
        let enabled = replay.controls.is_available();
        self.panel &= enabled;
        let left = self.left_island(ctx, enabled, taps);
        let right = self.right_island(ctx, enabled);
        let from_right = right.width() + SPACE;
        let pill = notice::notice_pill(ctx, &replay.video, from_right, taps);
        let segment = self.panel && replay.export.length == VideoLength::Segment;
        let stroke = stroke_words(ctx, replay, utc_offset);
        let bar = bar::bar(ctx, replay, (segment, &stroke), taps);
        let opener = Opener {
            island: RIGHT,
            button: self.button.unwrap_or(Rect::ZERO),
        };
        let laid = (&replay.export, canvas, enabled);
        let (panel, close) = panel::panel(ctx, (opener, self.panel), laid, taps);
        self.panel &= !close;
        let ppp = ctx.pixels_per_point();
        self.areas = [Some(left), Some(right), pill, Some(bar), panel]
            .into_iter()
            .flatten()
            .map(|rect| ScreenArea::from_points(rect, ppp))
            .collect();
    }

    /// Back, greyed while a video is made.
    fn left_island(&self, ctx: &egui::Context, enabled: bool, taps: &mut Vec<Intent>) -> Rect {
        let (back, rect) = island(ctx, LEFT, Place::top_left(), |ui| {
            text_button(ui, t::BACK, enabled).clicked()
        });
        if back {
            push_once(taps, Intent::Replay(ReplayIntent::Close));
        }
        rect
    }

    /// Export video…, in the ink pill while its panel is open; a tap opens or closes the panel.
    fn right_island(&mut self, ctx: &egui::Context, enabled: bool) -> Rect {
        let open = self.panel;
        let (tapped, rect) = island(ctx, RIGHT, Place::top_right(0.0), |ui| {
            choices(ui, "export video", |ui, group| {
                let shown = (enabled, open);
                let word = text_choice_wide(ui, group, t::EXPORT_VIDEO, shown, TOUCH_TARGET);
                self.button = Some(word.rect);
                word.clicked()
            })
        });
        if tapped {
            self.panel = !self.panel;
        }
        rect
    }

    /// Whether the replay showed in this pass.
    pub(crate) const fn showing(&self) -> bool {
        self.open
    }

    /// Whether it showed in the pass before this one.
    pub(crate) const fn was_showing(&self) -> bool {
        self.was
    }

    /// The canvas at fit in its room, while the replay shows.
    pub(crate) const fn view(&self) -> Option<ViewMapping> {
        self.view
    }

    /// Where the replay's controls lie, from this pass.
    pub(crate) fn areas(&self) -> &[ScreenArea] {
        &self.areas
    }

    /// A contact on the canvas outside closes the export panel.
    pub(crate) fn close_panel(&mut self) {
        self.panel = false;
    }

    /// The thin ring at the pen's tip while a stroke is drawn, in screen pixels.
    pub(crate) fn tip_cursor(&self, pixels_per_point: f32) -> Option<BrushCursor> {
        let (tip, view) = (self.tip?, self.view?);
        Some(BrushCursor {
            centre: view.to_screen(tip),
            radius: TIP_RADIUS * pixels_per_point,
            width: TIP_LINE * pixels_per_point,
        })
    }
}

/// Where the canvas shows and fits while the replay is open: the whole screen (`rect`, points),
/// fitting between the top band and the bar.
pub(crate) fn replay_area(rect: Rect, pixels_per_point: f32) -> CanvasArea {
    let mut fit = rect;
    fit.min.y += TOP_BAND;
    fit.max.y -= BAR_ROOM;
    CanvasArea {
        screen: ScreenArea::from_points(rect, pixels_per_point),
        fit: ScreenArea::from_points(fit, pixels_per_point),
    }
}

/// `canvas` at fit in `area`.
pub(crate) fn fit_view(
    canvas: CanvasSize,
    area: &CanvasArea,
    pixels_per_point: f32,
) -> ViewMapping {
    CanvasView::FIT.mapping(canvas, &area.viewport(pixels_per_point))
}

/// The stroke at the playhead in the pass's language, nothing before the first.
fn stroke_words(ctx: &egui::Context, replay: &ReplayProjection, utc_offset: i32) -> String {
    replay.stroke.map_or_else(String::new, |stroke| {
        let began = stroke.began_ms.map(|ms| local(ms, utc_offset));
        t::stroke_label(language(ctx), stroke.number, began)
    })
}

/// `ms` since 1970 UTC as the device's clock showed it, `offset` minutes from UTC: the day and
/// the hour and minute.
pub(crate) fn local(ms: u64, offset: i32) -> (Date, (i64, i64)) {
    let seconds = i64::try_from(ms / 1000).unwrap_or(i64::MAX) + i64::from(offset) * 60;
    let second = seconds.rem_euclid(86_400);
    (
        Date::of(seconds.div_euclid(86_400)),
        (second / 3600, second / 60 % 60),
    )
}
