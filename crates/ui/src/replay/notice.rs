//! The replay's notice, a pill in the top band left of the right island: the video being made
//! ("Exporting video", its progress line and percent, Cancel), saved ("Video saved to album ·
//! 48 s", Share… and OK) or failed (OK). A cancelled video tells nothing.

use egui::Rect;
use openpaint_app::{Intent, ReplayIntent, VideoNotice};

use crate::components::{
    Place, Source, confirmation_with, divider, notice, pill, progress, text_button,
};
use crate::design::SPACE;
use crate::taps::push_once;
use crate::text::{self as t, Words, language};

/// The progress line's length, in points.
const PROGRESS: f32 = 20.0 * SPACE;

/// The pill for `video`, `from_right` points in from the screen's right edge; its taps go to
/// `taps`. Returns where it lies, in points, while there is something to tell.
pub(super) fn notice_pill(
    ctx: &egui::Context,
    video: &VideoNotice,
    from_right: f32,
    taps: &mut Vec<Intent>,
) -> Option<Rect> {
    if *video == VideoNotice::None {
        return None;
    }
    let language = language(ctx);
    let place = Place::top_right(from_right);
    // It comes out of its right end, beside the right island.
    let source = Source::Within(egui::Align::Max);
    let (tapped, rect) = pill(ctx, "replay notice", place, source, |ui| match video {
        VideoNotice::Working { percent } => {
            ui.label(t::EXPORTING_VIDEO.get(language));
            ui.add_space(SPACE);
            let share = f32::from(*percent) / 100.0;
            progress(ui, share, (PROGRESS, &t::percent(*percent)));
            ui.add_space(2.0 * SPACE);
            divider(ui);
            let cancel = text_button(ui, t::CANCEL, true).clicked();
            cancel.then_some(ReplayIntent::Cancel)
        }
        VideoNotice::Saved { seconds } => {
            let message = t::video_saved(language, *seconds);
            let (share, ok) = confirmation_with(ui, Words::made(&message), t::SHARE_VIDEO);
            match (share.clicked(), ok.clicked()) {
                (true, _) => Some(ReplayIntent::ShareSaved),
                (false, true) => Some(ReplayIntent::Dismiss),
                (false, false) => None,
            }
        }
        VideoNotice::Failed(_) => {
            let ok = notice(ui, t::VIDEO_FAILED, t::OK, true).clicked();
            ok.then_some(ReplayIntent::Dismiss)
        }
        VideoNotice::None => None,
    });
    if let Some(intent) = tapped {
        push_once(taps, Intent::Replay(intent));
    }
    Some(rect)
}
