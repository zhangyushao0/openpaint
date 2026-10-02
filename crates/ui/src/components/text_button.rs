//! The text button: a word on a raised face, one touch target high and at least one wide.
//! Pressed, the face lightens under the hand; unavailable, the word dims and taps go nowhere.

use egui::{Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::design::{CORNER, ON_CHROME, ON_CHROME_DISABLED, PRESSED, RAISED, SPACE, TOUCH_TARGET};

/// Shows a button reading `label`; only an `enabled` button can be clicked.
pub fn text_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let font = TextStyle::Button.resolve(ui.style());
    let color = if enabled {
        ON_CHROME
    } else {
        ON_CHROME_DISABLED
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
    let width = (galley.size().x + 4.0 * SPACE).max(TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let face = if enabled && response.is_pointer_button_down_on() {
            PRESSED
        } else {
            RAISED
        };
        let painter = ui.painter();
        painter.rect_filled(rect.shrink(SPACE / 2.0), CORNER, face);
        let text = Rect::from_center_size(rect.center(), galley.size());
        painter.galley(text.min, galley, color);
    }
    response
}
