//! The slider: a value from 0 to 1 along a track one touch target high. The knob jumps to where
//! the hand presses and follows it; lifting the hand settles the value.

use egui::{Pos2, Rect, Sense, Stroke, Vec2, WidgetInfo};

use crate::design::{ON_CHROME, ON_CHROME_DISABLED, SPACE, TOUCH_TARGET};

/// Radius of the knob, in points.
const KNOB: f32 = 1.5 * SPACE;
/// Thickness of the track, in points.
const TRACK: f32 = SPACE / 2.0;

/// A value the hand set this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderChange {
    /// In `0..=1`.
    pub value: f32,
    /// True when the hand lifted: the last value of this touch.
    pub settled: bool,
}

/// Shows `value` (in `0..=1`) across the available width; `label` names it for accessibility.
/// Returns the value the hand set, if it set one this frame; only an `enabled` slider moves.
pub fn slider(ui: &mut egui::Ui, label: &str, value: f32, enabled: bool) -> Option<SliderChange> {
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::slider(enabled, f64::from(value), label));
    let track = rect.shrink2(Vec2::new(KNOB, 0.0));
    let settled = response.drag_stopped() || response.clicked();
    let touched = enabled && (response.is_pointer_button_down_on() || settled);
    let change = response
        .interact_pointer_pos()
        .filter(|_| touched)
        .map(|pos| SliderChange {
            value: ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0),
            settled,
        })
        .filter(|change| change.settled || change.value != value);
    if ui.is_rect_visible(rect) {
        let shown = change.map_or(value, |change| change.value);
        paint(ui, track, shown, enabled);
    }
    change
}

fn paint(ui: &egui::Ui, track: Rect, value: f32, enabled: bool) {
    let painter = ui.painter();
    let y = track.center().y;
    let knob = Pos2::new(track.left() + value * track.width(), y);
    let color = if enabled {
        ON_CHROME
    } else {
        ON_CHROME_DISABLED
    };
    let left = Pos2::new(track.left(), y);
    let right = Pos2::new(track.right(), y);
    painter.line_segment([left, right], Stroke::new(TRACK, ON_CHROME_DISABLED));
    painter.line_segment([left, knob], Stroke::new(TRACK, color));
    painter.circle_filled(knob, KNOB, color);
}
