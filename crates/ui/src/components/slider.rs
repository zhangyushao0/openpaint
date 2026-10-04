//! The slider: a value from 0 to 1 along a track one touch target high. The knob jumps to where
//! the hand presses and follows it; lifting the hand settles the value.

use egui::{Pos2, Rect, Sense, Stroke, Vec2, WidgetInfo};

use crate::components::controls;
use crate::design::{HAIRLINE, INK, INK_DISABLED, PANEL, SPACE, TOUCH_TARGET, ink_line};

/// Radius of the knob, in points.
const KNOB: f32 = 1.25 * SPACE;
/// Thickness of the inked part of the track and of the knob's ring, in points.
const TRACK: f32 = 2.0;

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
    controls::register(ui, label, rect);
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

/// The row over a slider: what it sets at the left, its value now at the right.
pub fn value_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0 * SPACE);
            ui.label(value);
        });
    });
}

fn paint(ui: &egui::Ui, track: Rect, value: f32, enabled: bool) {
    let y = track.center().y;
    let knob = Pos2::new(track.left() + value * track.width(), y);
    let ends = [Pos2::new(track.left(), y), Pos2::new(track.right(), y)];
    paint_track(ui.painter(), ends, knob, enabled);
}

/// A slider drawn as a line drawing: the whole track a pencil hairline, the part up to the knob
/// in ink, the knob an inked ring on paper. Shared with the side slider.
pub(crate) fn paint_track(painter: &egui::Painter, [start, end]: [Pos2; 2], knob: Pos2, on: bool) {
    let ink = if on { INK } else { INK_DISABLED };
    painter.line_segment([start, end], Stroke::new(HAIRLINE, INK_DISABLED));
    ink_line(painter, &[start, knob], Stroke::new(TRACK, ink));
    painter.circle(knob, KNOB, PANEL, Stroke::new(TRACK, ink));
}
