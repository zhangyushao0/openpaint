//! The icon button: one touch target on the chrome with an icon. Pressed, it shows a highlight
//! under the hand; unavailable, its icon dims and taps go nowhere. The toggle form stays
//! highlighted while what it opens is open.

use egui::{Rect, Sense, Vec2, WidgetInfo, WidgetType};

use crate::design::{
    CORNER, ICON_SIZE, Icon, ON_CHROME, ON_CHROME_DISABLED, PRESSED, SELECTED, SPACE, TOUCH_TARGET,
    paint_icon,
};

/// Shows a button of one touch target with `icon`; `label` names it for accessibility. Only an
/// `enabled` button can be clicked.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    button(ui, icon, label, enabled, false)
}

/// An icon button that shows whether what it opens is `on`.
pub fn icon_toggle(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool) -> egui::Response {
    button(ui, icon, label, true, on)
}

fn button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool, on: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let face = if enabled && response.is_pointer_button_down_on() {
            Some(PRESSED)
        } else if on {
            Some(SELECTED)
        } else {
            None
        };
        if let Some(face) = face {
            painter.rect_filled(rect.shrink(SPACE / 2.0), CORNER, face);
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
