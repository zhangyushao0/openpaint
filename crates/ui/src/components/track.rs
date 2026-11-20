//! The track: a scrubber along a timeline (the replay's), one touch target high under a line of
//! two quiet readouts (what is at the playhead at its left end, the time at its right). The part
//! played in ink, the rest a pencil hairline, a small upright mark where each of `marks` lies,
//! the knob at the playhead: a tap or a drag puts the playhead there, following the hand 1:1.
//! While a segment is chosen: the segment in thicker ink between two upright handles, no knob;
//! the handle nearer where the hand pressed follows it, never past the other (each handle is
//! named on its own). Unavailable, all of it in pencil grey and taps go nowhere. `progress` is
//! the line alone with its percent, for work running in a pill.

use egui::{Align2, Pos2, Rect, Sense, Stroke, StrokeKind, TextStyle, Vec2, WidgetInfo};

use crate::components::controls;
use crate::components::slider::{KNOB, TRACK, along, paint_track};
use crate::design::{
    HAIRLINE, INK, INK_DISABLED, INK_SECONDARY, MARK_LINE, PANEL, SPACE, TOUCH_TARGET, ink_line,
};
use crate::text::Words;

/// The readouts' line over the track, in points.
pub const TRACK_OVER: f32 = 3.0 * SPACE;
/// A segment's handle: an upright pill, in points.
const HANDLE: Vec2 = Vec2::new(1.25 * SPACE, 3.0 * SPACE);
/// A mark's height, in points.
const MARK: f32 = 1.5 * SPACE;

/// What one track shows.
#[derive(Debug, Clone, Copy)]
pub struct Track<'a> {
    /// Names the track, its English (the scrubber).
    pub label: Words<'a>,
    /// The readouts over its left and right ends, in the language shown.
    pub over: [&'a str; 2],
    /// The playhead, 0 to 1.
    pub at: f32,
    /// Where each mark stands, 0 to 1.
    pub marks: &'a [f32],
    /// While a segment is chosen: its ends, 0 to 1, the first before the second, and the names
    /// of their handles.
    pub segment: Option<([f32; 2], [Words<'a>; 2])>,
    pub enabled: bool,
}

/// What the hand did on the track this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackChange {
    /// The playhead to this place.
    Seek(f32),
    /// The segment's ends to these places, the first before the second.
    Segment([f32; 2]),
}

/// Shows `track`, `width` points wide, the readouts over it. Returns what the hand changed this
/// frame; what shows follows it in the same frame.
pub fn track(ui: &mut egui::Ui, track: &Track<'_>, width: f32) -> Option<TrackChange> {
    let size = Vec2::new(width, TRACK_OVER + TOUCH_TARGET);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let bar = Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - TOUCH_TARGET),
        rect.max,
    );
    let sense = if track.enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let response = ui.interact(bar, ui.id().with(track.label.id()), sense);
    let shown = track.label.shown(ui.ctx());
    response.widget_info(|| WidgetInfo::slider(track.enabled, f64::from(track.at), shown));
    controls::register(ui, track.label.id(), bar);
    let line = bar.shrink2(Vec2::new(KNOB, 0.0));
    let change = match track.segment {
        None => along(&response, line, track.at, track.enabled)
            .map(|change| TrackChange::Seek(change.value)),
        Some((ends, names)) => {
            for (end, name) in ends.into_iter().zip(names) {
                let at = Pos2::new(place(line, end), bar.center().y);
                let handle = Rect::from_center_size(at, Vec2::splat(TOUCH_TARGET));
                controls::register(ui, name.id(), handle);
            }
            held(ui.ctx(), &response, line, ends, track.enabled).map(TrackChange::Segment)
        }
    };
    if ui.is_rect_visible(rect) {
        let shown = match (change, track.segment) {
            (Some(TrackChange::Seek(at)), _) => Track { at, ..*track },
            (Some(TrackChange::Segment(ends)), Some((_, names))) => Track {
                segment: Some((ends, names)),
                ..*track
            },
            _ => *track,
        };
        paint(ui, (rect, line), &shown);
    }
    change
}

/// The line of work running (a video being made), `width` points long and inked to `share`
/// (0 to 1), then `percent`, quieter.
pub fn progress(ui: &mut egui::Ui, share: f32, (width, percent): (f32, &str)) {
    let small = TextStyle::Small.resolve(ui.style());
    let galley = (ui.painter()).layout_no_wrap(percent.to_owned(), small, INK_SECONDARY);
    let size = Vec2::new(width + SPACE + galley.size().x, TOUCH_TARGET);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let y = rect.center().y;
    let (start, end) = (Pos2::new(rect.left(), y), Pos2::new(rect.left() + width, y));
    let painter = ui.painter();
    painter.line_segment([start, end], Stroke::new(HAIRLINE, INK_DISABLED));
    let reached = Pos2::new(start.x + share.clamp(0.0, 1.0) * width, y);
    ink_line(painter, &[start, reached], Stroke::new(TRACK, INK));
    let at = Pos2::new(end.x + SPACE, y - galley.size().y / 2.0);
    painter.galley(at, galley, INK_SECONDARY);
}

/// Where `value` (0 to 1) lies along `line`, in points.
fn place(line: Rect, value: f32) -> f32 {
    line.left() + value.clamp(0.0, 1.0) * line.width()
}

/// The segment's ends the hand moves this frame: the handle nearer where it pressed follows it,
/// never past the other; it holds that handle until it lifts.
fn held(
    ctx: &egui::Context,
    response: &egui::Response,
    line: Rect,
    [start, end]: [f32; 2],
    enabled: bool,
) -> Option<[f32; 2]> {
    let id = response.id.with("held");
    let released = response.drag_stopped() || response.clicked();
    let pressed = enabled && (response.is_pointer_button_down_on() || released);
    let Some(at) = response.interact_pointer_pos().filter(|_| pressed) else {
        ctx.data_mut(|data| data.remove::<usize>(id));
        return None;
    };
    let value = ((at.x - line.left()) / line.width()).clamp(0.0, 1.0);
    let kept = ctx.data(|data| data.get_temp::<usize>(id));
    let handle = kept.unwrap_or_else(|| nearer(value, [start, end]));
    ctx.data_mut(|data| {
        if released {
            data.remove::<usize>(id);
        } else {
            data.insert_temp(id, handle);
        }
    });
    let moved = if handle == 0 {
        [value.min(end), end]
    } else {
        [start, value.max(start)]
    };
    (moved != [start, end]).then_some(moved)
}

/// Which handle (0 the start, 1 the end) lies nearer `value`; on one place, the one with room to
/// move towards it.
fn nearer(value: f32, [start, end]: [f32; 2]) -> usize {
    let (to_start, to_end) = ((value - start).abs(), (value - end).abs());
    if to_start < to_end || (to_start == to_end && (value < start || end >= 1.0)) {
        0
    } else {
        1
    }
}

fn paint(ui: &egui::Ui, (rect, line): (Rect, Rect), track: &Track<'_>) {
    let painter = ui.painter();
    let (ink, quiet) = if track.enabled {
        (INK, INK_SECONDARY)
    } else {
        (INK_DISABLED, INK_DISABLED)
    };
    let small = TextStyle::Small.resolve(ui.style());
    let over = rect.top() + TRACK_OVER / 2.0;
    let [left, right] = track.over;
    let at = Pos2::new(line.left(), over);
    painter.text(at, Align2::LEFT_CENTER, left, small.clone(), quiet);
    let at = Pos2::new(line.right(), over);
    painter.text(at, Align2::RIGHT_CENTER, right, small, quiet);
    let y = line.center().y;
    for mark in track.marks {
        let x = place(line, *mark);
        let ends = [Pos2::new(x, y - MARK / 2.0), Pos2::new(x, y + MARK / 2.0)];
        painter.line_segment(ends, Stroke::new(MARK_LINE, quiet));
    }
    let ends = [Pos2::new(line.left(), y), Pos2::new(line.right(), y)];
    let Some(([start, end], _)) = track.segment else {
        let knob = Pos2::new(place(line, track.at), y);
        paint_track(painter, ends, knob, track.enabled);
        return;
    };
    painter.line_segment(ends, Stroke::new(HAIRLINE, INK_DISABLED));
    let segment = [start, end].map(|value| Pos2::new(place(line, value), y));
    ink_line(painter, &segment, Stroke::new(2.0 * TRACK, ink));
    for at in segment {
        let handle = Rect::from_center_size(at, HANDLE);
        let stroke = Stroke::new(TRACK, ink);
        painter.rect(handle, HANDLE.x / 2.0, PANEL, stroke, StrokeKind::Middle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_handle_nearer_the_hand_moves_and_on_one_place_the_one_with_room() {
        assert_eq!(nearer(0.1, [0.2, 0.8]), 0);
        assert_eq!(nearer(0.7, [0.2, 0.8]), 1);
        assert_eq!(nearer(0.5, [0.5, 0.5]), 1, "room to the right");
        assert_eq!(nearer(0.4, [0.5, 0.5]), 0, "pressed left of both");
        assert_eq!(
            nearer(1.0, [1.0, 1.0]),
            0,
            "both at the end: the start moves"
        );
        assert_eq!(
            nearer(0.0, [0.0, 0.0]),
            1,
            "both at the start: the end moves"
        );
    }
}
