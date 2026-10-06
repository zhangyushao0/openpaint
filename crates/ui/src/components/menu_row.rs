//! The menu row: one touch target high across a panel, an action's name at its left and an
//! optional note at its right end, dimmer. Tapping it does the action; pressed, a faint wash
//! fills the row's outline under the hand; unavailable, it dims and taps go nowhere.

use egui::{Align2, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::controls;
use crate::design::{
    ICON_SIZE, INK, INK_DISABLED, INK_SECONDARY, Icon, Outline, SPACE, TOUCH_TARGET, paint_icon,
    paint_pressed,
};

/// Shows a row reading `label`, with `note` (empty for none) at its right end.
pub fn menu_row(ui: &mut egui::Ui, label: &str, note: &str, enabled: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    controls::register(ui, label, rect);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if enabled && response.is_pointer_button_down_on() {
        paint_pressed(painter, Outline::row(rect));
    }
    let (text, quiet) = if enabled {
        (INK, INK_SECONDARY)
    } else {
        (INK_DISABLED, INK_DISABLED)
    };
    let font = TextStyle::Body.resolve(ui.style());
    let left = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(left, Align2::LEFT_CENTER, label, font, text);
    let small = TextStyle::Small.resolve(ui.style());
    let right = rect.right_center() - Vec2::new(2.0 * SPACE, 0.0);
    painter.text(right, Align2::RIGHT_CENTER, note, small, quiet);
    response
}

/// A command row (Add Mask, Merge Down): `icon` leading the words `label`, both in ink, and at
/// its right end `note` (empty for none: why it cannot act now, small and in the quieter ink),
/// so it reads as a thing to tap among the switches and labels around it. Pressed and
/// unavailable as `menu_row`.
pub fn command_row(
    ui: &mut egui::Ui,
    (icon, label, note): (Icon, &str, &str),
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    controls::register(ui, label, rect);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if enabled && response.is_pointer_button_down_on() {
        paint_pressed(painter, Outline::row(rect));
    }
    let color = if enabled { INK } else { INK_DISABLED };
    let lead = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    let square = Rect::from_min_size(
        lead - Vec2::new(0.0, ICON_SIZE / 2.0),
        Vec2::splat(ICON_SIZE),
    );
    paint_icon(painter, icon, square, color);
    let font = TextStyle::Body.resolve(ui.style());
    let at = lead + Vec2::new(ICON_SIZE + SPACE, 0.0);
    painter.text(at, Align2::LEFT_CENTER, label, font, color);
    let small = TextStyle::Small.resolve(ui.style());
    let right = rect.right_center() - Vec2::new(2.0 * SPACE, 0.0);
    painter.text(right, Align2::RIGHT_CENTER, note, small, INK_SECONDARY);
    response
}

/// A hint across a panel, one touch target high: what something does, in the quieter ink, so the
/// bare words that are commands stay what reads as tappable.
pub fn hint(ui: &mut egui::Ui, text: &str) {
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let font = TextStyle::Body.resolve(ui.style());
    let at = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    ui.painter()
        .text(at, Align2::LEFT_CENTER, text, font, INK_SECONDARY);
}

/// A section's name over its rows: small and quiet, so the rows stay what the eye reads.
pub fn section(ui: &mut egui::Ui, title: &str) {
    let size = Vec2::new(ui.available_width(), 4.0 * SPACE);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let font = TextStyle::Small.resolve(ui.style());
    let at = rect.left_bottom() + Vec2::new(2.0 * SPACE, -SPACE);
    ui.painter()
        .text(at, Align2::LEFT_BOTTOM, title, font, INK_SECONDARY);
}
