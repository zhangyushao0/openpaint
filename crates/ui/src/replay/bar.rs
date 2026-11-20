//! The replay's bar, floating under the canvas, three quarters of the screen wide (the mock's):
//! Play or Pause in its ring, the track with its readouts (the stroke at the playhead, the time;
//! while Segment is chosen in the open panel the segment's range and length, its handles on the
//! track), the speeds with the one chosen in the ink pill. Greyed while a video is made.

use egui::{Align2, Rect, Vec2};
use openpaint_app::{Intent, ReplayIntent, ReplayProjection, Speed};

use crate::components::{
    Place, TRACK_OVER, Track, TrackChange, choice_width, choices, icon_ringed, island_of,
    text_choice_wide, track,
};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::taps::push_once;
use crate::text::{self as t, Words, language};

/// The bar's height, its readouts over its track, in points.
const BAR_HEIGHT: f32 = TRACK_OVER + TOUCH_TARGET;
/// The room the bar takes at the screen's foot, under the canvas's fit, in points.
pub(super) const BAR_ROOM: f32 = BAR_HEIGHT + 2.0 * SPACE;
/// The bar's share of the screen's width.
const BAR_SHARE: f32 = 0.75;

/// The bar for `replay`, the track showing the segment while `segment`, the stroke at the
/// playhead in `stroke`; its taps go to `taps`. Returns where it lies, in points.
pub(super) fn bar(
    ctx: &egui::Context,
    replay: &ReplayProjection,
    (segment, stroke): (bool, &str),
    taps: &mut Vec<Intent>,
) -> Rect {
    let screen = ctx.content_rect();
    let width = (screen.width() * BAR_SHARE).min(screen.width() - 2.0 * SPACE);
    let place = Place {
        anchor: Align2::CENTER_BOTTOM,
        offset: Vec2::new(0.0, -SPACE),
    };
    let enabled = replay.controls.is_available();
    let ((), rect) = island_of(ctx, "replay bar", (place, BAR_HEIGHT), |ui| {
        let (icon, label) = if replay.playing {
            (Icon::Pause, t::REPLAY_PAUSE)
        } else {
            (Icon::Play, t::REPLAY_PLAY)
        };
        if icon_ringed(ui, icon, label, enabled).clicked() {
            push_once(taps, Intent::Replay(ReplayIntent::Play(!replay.playing)));
        }
        let speeds = Speed::ALL.map(|speed| choice_width(ui, Words::made(t::speed(speed))));
        let gaps = 6.0 * ui.spacing().item_spacing.x + 3.0 * SPACE;
        let room = width - TOUCH_TARGET - speeds.iter().sum::<f32>() - gaps;
        ui.add_space(SPACE);
        let change = scrubber(ui, replay, (segment, stroke), room.max(TOUCH_TARGET));
        if let Some(change) = change {
            let intent = match change {
                TrackChange::Seek(at) => ReplayIntent::Seek(at),
                TrackChange::Segment(ends) => ReplayIntent::Segment(ends),
            };
            push_once(taps, Intent::Replay(intent));
        }
        ui.add_space(SPACE);
        speed_choices(ui, (replay.speed, enabled), taps);
    });
    rect
}

/// The track, `width` points wide, and its readouts; what the hand changed on it.
fn scrubber(
    ui: &mut egui::Ui,
    replay: &ReplayProjection,
    (segment, stroke): (bool, &str),
    width: f32,
) -> Option<TrackChange> {
    let export = &replay.export;
    let time = if segment {
        t::segment_time(language(ui.ctx()), export.segment_ms)
    } else {
        t::playhead(replay.at_ms, replay.total_ms)
    };
    let handles = [t::SEGMENT_START, t::SEGMENT_END].map(Words::from);
    let shown = Track {
        label: t::REPLAY_TRACK.into(),
        over: [stroke, &time],
        at: replay.at,
        marks: &replay.sittings,
        segment: segment.then_some((export.segment, handles)),
        enabled: replay.controls.is_available(),
    };
    track(ui, &shown, width)
}

/// The speeds, `chosen` in the ink pill; a tap on another plays at it.
fn speed_choices(ui: &mut egui::Ui, (chosen, enabled): (Speed, bool), taps: &mut Vec<Intent>) {
    choices(ui, "replay speed", |ui, group| {
        for speed in Speed::ALL {
            let words = Words::made(t::speed(speed));
            let on = speed == chosen;
            let word = text_choice_wide(ui, group, words, (enabled, on), TOUCH_TARGET);
            if word.clicked() && !on {
                push_once(taps, Intent::Replay(ReplayIntent::Speed(speed)));
            }
        }
    });
}
