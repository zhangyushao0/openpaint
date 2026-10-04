//! The swatch: a round patch of one colour inside a square touch target, ringed with a fine ink
//! line so white shows on the paper too. Pressed, a faint wash fills its ring; `on`, it
//! carries the active mark (the colour panel is open, or this is the current colour).

use egui::{Color32, Sense, Stroke, Vec2, WidgetInfo, WidgetType};
use openpaint_core::Color;

use crate::components::{animate, controls};
use crate::design::motion::ACKNOWLEDGE;
use crate::design::{INK, Outline, paint_mark, paint_pressed};

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
    controls::register(ui, label, rect);
    let target = f32::from(u8::from(on));
    let mark = animate(ui.ctx(), response.id.with("mark"), target, ACKNOWLEDGE);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.is_pointer_button_down_on() {
            paint_pressed(painter, Outline::ring(rect));
        }
        paint_mark(painter, Outline::ring(rect), mark);
        let radius = 0.3 * side;
        painter.circle(rect.center(), radius, color32(color), Stroke::new(1.0, INK));
    }
    response.on_hover_text(label)
}
