//! The swatch: a round patch of one colour inside a square touch target, ringed in the chrome's
//! text colour so that dark colours stand out on the dark chrome too. Pressed, it shows a
//! highlight under the hand; `on`, it stays highlighted (the colour panel is open, or this is
//! the current colour).

use egui::{Color32, Sense, Stroke, Vec2, WidgetInfo, WidgetType};
use openpaint_core::Color;

use crate::design::{CORNER, ON_CHROME, PRESSED, SELECTED, SPACE};

/// The colour as egui paints it: 8-bit sRGB, opaque.
#[must_use]
pub fn color32(color: Color) -> Color32 {
    let byte = |channel: f32| (channel * 255.0).round() as u8;
    Color32::from_rgb(byte(color.r()), byte(color.g()), byte(color.b()))
}

/// Shows a swatch of `color` in a square of `side` points; `label` names it for accessibility.
pub fn swatch(ui: &mut egui::Ui, color: Color, label: &str, side: f32, on: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let face = if response.is_pointer_button_down_on() {
            Some(PRESSED)
        } else if on {
            Some(SELECTED)
        } else {
            None
        };
        if let Some(face) = face {
            painter.rect_filled(rect.shrink(SPACE / 4.0), CORNER, face);
        }
        let radius = 0.3 * side;
        painter.circle(
            rect.center(),
            radius,
            color32(color),
            Stroke::new(2.0, ON_CHROME),
        );
    }
    response
}
