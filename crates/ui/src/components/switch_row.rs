//! The switch row: one touch target high across a panel, a name at its left and a switch at its
//! right end that shows whether the setting is on. Tapping anywhere on the row flips it;
//! unavailable, it dims and taps go nowhere.

use egui::{Align2, Pos2, Rect, Sense, Stroke, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::design::{
    INK, INK_ACTIVE, INK_DISABLED, INK_SECONDARY, SPACE, TOUCH_TARGET, paint_pressed,
};

/// The switch's track, in points.
const TRACK: Vec2 = Vec2::new(5.0 * SPACE, 3.0 * SPACE);

pub fn switch_row(ui: &mut egui::Ui, label: &str, on: bool, enabled: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, label));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if enabled && response.is_pointer_button_down_on() {
        paint_pressed(painter, rect.shrink2(Vec2::new(0.0, SPACE / 4.0)));
    }
    let text = if enabled { INK } else { INK_DISABLED };
    let font = TextStyle::Body.resolve(ui.style());
    let left = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(left, Align2::LEFT_CENTER, label, font, text);
    let centre = rect.right_center() - Vec2::new(2.0 * SPACE + TRACK.x / 2.0, 0.0);
    let track = Rect::from_center_size(centre, TRACK);
    // Off: an empty outline with a hollow knob at the left. On: the knob inked at the right.
    let (line, x) = match (on, enabled) {
        (_, false) => (INK_DISABLED, if on { track.right() } else { track.left() }),
        (true, true) => (INK_ACTIVE, track.right()),
        (false, true) => (INK_SECONDARY, track.left()),
    };
    let radius = TRACK.y / 2.0;
    painter.rect_stroke(
        track,
        radius,
        Stroke::new(1.5, line),
        egui::StrokeKind::Inside,
    );
    let inset = if on { -radius } else { radius };
    let knob = Pos2::new(x + inset, centre.y);
    if on {
        painter.circle_filled(knob, radius - 4.0, line);
    } else {
        painter.circle_stroke(knob, radius - 4.0, Stroke::new(1.5, line));
    }
    response
}
