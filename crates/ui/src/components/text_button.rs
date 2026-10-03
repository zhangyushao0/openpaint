//! The text button: a word inside a hairline outline, one touch target high and at least one
//! wide. Pressed, a faint wash shows under the hand; unavailable, word and outline fade to pencil
//! grey and taps go nowhere.

use egui::{Rect, Sense, Stroke, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::design::{
    BUTTON_RULE, CORNER, HAIRLINE, INK, INK_DISABLED, SPACE, TOUCH_TARGET, paint_pressed,
};

/// Shows a button reading `label`; only an `enabled` button can be clicked.
pub fn text_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let font = TextStyle::Button.resolve(ui.style());
    let (color, rule) = if enabled {
        (INK, BUTTON_RULE)
    } else {
        (INK_DISABLED, INK_DISABLED)
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
    let width = (galley.size().x + 4.0 * SPACE).max(TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let face = rect.shrink(SPACE / 2.0);
        if enabled && response.is_pointer_button_down_on() {
            paint_pressed(painter, face);
        }
        painter.rect_stroke(
            face,
            CORNER,
            Stroke::new(HAIRLINE, rule),
            egui::StrokeKind::Inside,
        );
        let text = Rect::from_center_size(rect.center(), galley.size());
        painter.galley(text.min, galley, color);
    }
    response
}
