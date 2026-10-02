//! The icon button: one touch target on the chrome with an icon. Pressed, it shows a highlight
//! under the hand; unavailable, its icon dims and taps go nowhere.

use egui::{Rect, Sense, Vec2, WidgetInfo, WidgetType};

use crate::design::{
    CORNER, ICON_SIZE, Icon, ON_CHROME, ON_CHROME_DISABLED, PRESSED, SPACE, TOUCH_TARGET,
    paint_icon,
};

/// Shows a button of one touch target with `icon`; `label` names it for accessibility. Only an
/// `enabled` button can be clicked.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.is_pointer_button_down_on() {
            painter.rect_filled(rect.shrink(SPACE / 2.0), CORNER, PRESSED);
        }
        let color = if enabled {
            ON_CHROME
        } else {
            ON_CHROME_DISABLED
        };
        let square = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        paint_icon(painter, icon, square, color);
    }
    response
}
