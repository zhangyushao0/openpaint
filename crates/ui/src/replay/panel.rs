//! The export panel, a popover under the replay's right island, wider than the studio's panels
//! (the mock's, so the canvas's own size and the end's switches keep to one line): Length as
//! choice rows with what each lasts (the highlight its length only when not 30 s, Full its
//! length, Segment its range), Size as two words in the ink pill (the canvas's own greyed beyond
//! what the encoder takes), End as two switches (the line of strokes and time only with the
//! finished picture held), then Save to album in its pill and Share…, either closing the panel.

use egui::Rect;
use openpaint_app::{
    Delivery, ExportChoices, HIGHLIGHT_MS, Intent, ReplayIntent, VideoLength, VideoSize,
};
use openpaint_core::CanvasSize;

use crate::components::{
    Opener, choice_row, choices, panel_heading, pill_button, popover_of_width, section, switch_row,
    text_button, text_choice_wide,
};
use crate::design::{SPACE, TOUCH_TARGET};
use crate::taps::push_once;
use crate::text::{self as t, Text, Words, language};

/// The panel's width, in points.
const WIDTH: f32 = 50.0 * SPACE;

/// The panel while it is open or closing, its choices `export` for a canvas of `canvas` pixels,
/// taking taps while `enabled`. Returns where the open panel lies (points) and whether a tap
/// closes it.
pub(super) fn panel(
    ctx: &egui::Context,
    (opener, open): (Opener, bool),
    (export, canvas, enabled): (&ExportChoices, Option<CanvasSize>, bool),
    taps: &mut Vec<Intent>,
) -> (Option<Rect>, bool) {
    let mut close = false;
    let mut tap = |intent| push_once(taps, Intent::Replay(intent));
    let rect = popover_of_width(ctx, "export video", (opener, open), WIDTH, |ui| {
        panel_heading(ui, t::EXPORT_VIDEO_HEADING);
        section(ui, t::LENGTH);
        if let Some(length) = length_rows(ui, export, enabled) {
            tap(ReplayIntent::Length(length));
        }
        section(ui, t::VIDEO_SIZE);
        if let Some(size) = size_row(ui, export, (canvas, enabled)) {
            tap(ReplayIntent::Size(size));
        }
        section(ui, t::VIDEO_END);
        if switch_row(ui, t::STOP_ON_FINISHED, export.hold, enabled).clicked() {
            tap(ReplayIntent::Hold(!export.hold));
        }
        let caption = export.hold && export.caption;
        if switch_row(ui, t::STROKES_AND_TIME, caption, enabled && export.hold).clicked() {
            tap(ReplayIntent::Caption(!export.caption));
        }
        ui.add_space(SPACE);
        if let Some(delivery) = deliveries(ui, enabled) {
            tap(ReplayIntent::Export(delivery));
            close = true;
        }
    });
    (rect.filter(|_| !close), close)
}

/// The lengths, the one chosen with its ring inked; the one tapped if it was not chosen.
fn length_rows(ui: &mut egui::Ui, export: &ExportChoices, enabled: bool) -> Option<VideoLength> {
    let mut tapped = None;
    for (index, length) in VideoLength::ALL.into_iter().enumerate() {
        let note = note(export, (index, length));
        let chosen = export.length == length;
        let words = (Words::from(length_name(length)), Words::made(&note));
        if choice_row(ui, words, (enabled, chosen)).clicked() && !chosen {
            tapped = Some(length);
        }
    }
    tapped
}

const fn length_name(length: VideoLength) -> Text {
    match length {
        VideoLength::Highlight => t::HIGHLIGHT,
        VideoLength::Whole => t::FULL,
        VideoLength::Segment => t::SEGMENT,
    }
}

/// What a length's row says at its right: the highlight's length when it is not 30 s, the
/// whole's length, the segment's range.
fn note(export: &ExportChoices, (index, length): (usize, VideoLength)) -> String {
    let lasts = export.lengths_ms[index];
    match length {
        VideoLength::Highlight if lasts == HIGHLIGHT_MS => String::new(),
        VideoLength::Highlight | VideoLength::Whole => t::clock(lasts),
        VideoLength::Segment => t::range(export.segment_ms),
    }
}

/// 1080p and the canvas's own size, the one chosen in the ink pill, the canvas's own greyed when
/// the encoder cannot take it; the one tapped if it was not chosen.
fn size_row(
    ui: &mut egui::Ui,
    export: &ExportChoices,
    (canvas, enabled): (Option<CanvasSize>, bool),
) -> Option<VideoSize> {
    let own = canvas.map_or([0, 0], |canvas| [canvas.width(), canvas.height()]);
    let original = t::original(language(ui.ctx()), own);
    let mut tapped = None;
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(SPACE);
        choices(ui, "video size", |ui, group| {
            for (index, size) in VideoSize::ALL.into_iter().enumerate() {
                let words = match size {
                    VideoSize::Hd => Words::made(t::HD),
                    VideoSize::Canvas => Words::named(t::ORIGINAL.id(), &original),
                };
                let offered = enabled && export.sizes[index].is_some();
                let chosen = export.size == size;
                let word = text_choice_wide(ui, group, words, (offered, chosen), TOUCH_TARGET);
                if word.clicked() && !chosen {
                    tapped = Some(size);
                }
            }
        });
    });
    tapped
}

/// Save to album in its pill, then Share…; the delivery tapped.
fn deliveries(ui: &mut egui::Ui, enabled: bool) -> Option<Delivery> {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(SPACE);
        let save = pill_button(ui, t::SAVE_TO_ALBUM, enabled).clicked();
        ui.add_space(SPACE);
        let share = text_button(ui, t::SHARE_VIDEO, enabled).clicked();
        match (save, share) {
            (true, _) => Some(Delivery::Save),
            (false, true) => Some(Delivery::Share),
            (false, false) => None,
        }
    })
    .inner
}
