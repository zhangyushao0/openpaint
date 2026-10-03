//! The switch row: one touch target high across a panel, a name at its left and a switch at its
//! right end that shows whether the setting is on. Tapping anywhere on the row flips it;
//! unavailable, it dims and taps go nowhere.

use egui::{Align2, Pos2, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::design::{
    CHROME, CORNER, ON_CHROME, ON_CHROME_DISABLED, ON_CHROME_SECONDARY, OUTLINE, PRESSED, SPACE,
    TOUCH_TARGET,
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
        painter.rect_filled(rect.shrink2(Vec2::new(0.0, SPACE / 4.0)), CORNER, PRESSED);
    }
    let text = if enabled {
        ON_CHROME
    } else {
        ON_CHROME_DISABLED
    };
    let font = TextStyle::Body.resolve(ui.style());
    let left = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(left, Align2::LEFT_CENTER, label, font, text);
    let centre = rect.right_center() - Vec2::new(2.0 * SPACE + TRACK.x / 2.0, 0.0);
    let track = Rect::from_center_size(centre, TRACK);
    let (fill, knob, x) = match (on, enabled) {
        (true, true) => (ON_CHROME, CHROME, track.right()),
        (true, false) => (ON_CHROME_DISABLED, CHROME, track.right()),
        (false, _) => (OUTLINE, ON_CHROME_SECONDARY, track.left()),
    };
    let radius = TRACK.y / 2.0;
    painter.rect_filled(track, radius, fill);
    let inset = if on { -radius } else { radius };
    painter.circle_filled(Pos2::new(x + inset, centre.y), radius - 3.0, knob);
    response
}
