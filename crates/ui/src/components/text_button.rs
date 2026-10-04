//! The text button: a word inside a hairline outline, one touch target high and at least one
//! wide. Pressed, a faint wash shows under the hand; unavailable, word and outline fade to pencil
//! grey and taps go nowhere. The choice form (one of a few options, such as the selection's
//! shape) carries the active mark, a deeper ink outline, while it is the one chosen.

use egui::{Rect, Sense, Stroke, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::controls;
use crate::design::{
    BUTTON_RULE, CORNER, HAIRLINE, INK, INK_ACTIVE, INK_DISABLED, SPACE, TOUCH_TARGET,
    paint_pressed, paint_row_mark,
};

/// Shows a button reading `label`; only an `enabled` button can be clicked.
pub fn text_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    button(ui, label, (enabled, false), TOUCH_TARGET)
}

/// A button reading `label` that shows whether it is the option `chosen`.
pub fn text_choice(ui: &mut egui::Ui, label: &str, chosen: bool) -> egui::Response {
    button(ui, label, (true, chosen), TOUCH_TARGET)
}

/// As `text_choice`, at least `width` wide (options in even columns); unavailable ones grey.
pub fn text_choice_wide(
    ui: &mut egui::Ui,
    label: &str,
    (enabled, chosen): (bool, bool),
    width: f32,
) -> egui::Response {
    button(ui, label, (enabled, chosen), width)
}

fn button(
    ui: &mut egui::Ui,
    label: &str,
    (enabled, chosen): (bool, bool),
    min_width: f32,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let font = TextStyle::Button.resolve(ui.style());
    let (color, rule) = match (enabled, chosen) {
        (false, _) => (INK_DISABLED, INK_DISABLED),
        (true, true) => (INK_ACTIVE, INK_ACTIVE),
        (true, false) => (INK, BUTTON_RULE),
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
    let width = (galley.size().x + 4.0 * SPACE).max(min_width);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, chosen, label));
    controls::register(ui, label, rect);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let face = rect.shrink(SPACE / 2.0);
        if enabled && response.is_pointer_button_down_on() {
            paint_pressed(painter, face);
        }
        if chosen {
            paint_row_mark(painter, rect.shrink2(Vec2::new(SPACE / 4.0, 0.0)));
        } else {
            painter.rect_stroke(
                face,
                CORNER,
                Stroke::new(HAIRLINE, rule),
                egui::StrokeKind::Inside,
            );
        }
        let text = Rect::from_center_size(rect.center(), galley.size());
        painter.galley(text.min, galley, color);
    }
    response
}
