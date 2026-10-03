//! The fill bar: one line under the top bar's middle while a colour is dropped (what to do, and
//! the threshold while it is adjusted) and right after (the offer to keep filling), with at
//! most one text button. Nothing under it takes a contact that starts on it. Beside it, the
//! drop the hand carries: a disc of the colour, ringed so it shows on any canvas.

use egui::{Align, Align2, Frame, Layout, Margin, Order, Pos2, Stroke, Vec2};
use openpaint_core::Color;

use crate::components::{TOP_BAR_HEIGHT, color32, text_button};
use crate::design::{CHROME, CORNER, ON_CHROME, OUTLINE, SPACE, TOUCH_TARGET};

/// Radius of the carried drop, in points.
const DROP_RADIUS: f32 = 12.0;

/// Shows `text` and, with `action`, a button reading it, centred under the top bar. Returns the
/// bar's rectangle in points and whether the button was clicked.
pub fn fill_bar(ctx: &egui::Context, text: &str, action: Option<&str>) -> (egui::Rect, bool) {
    let shown = egui::Area::new(egui::Id::new("fill bar"))
        .order(Order::Foreground)
        .fade_in(false)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, TOP_BAR_HEIGHT + SPACE))
        .show(ctx, |ui| {
            Frame::new()
                .fill(CHROME)
                .stroke(Stroke::new(1.0, OUTLINE))
                .corner_radius(CORNER)
                .inner_margin(Margin::symmetric((2.0 * SPACE) as i8, 0))
                .show(ui, |ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.set_height(TOUCH_TARGET);
                        ui.label(text);
                        action.is_some_and(|label| text_button(ui, label, true).clicked())
                    })
                    .inner
                })
                .inner
        });
    (shown.response.rect, shown.inner)
}

/// The drop of `color` the hand carries, centred on `at` (points), over everything.
pub fn carried_drop(ctx: &egui::Context, at: Pos2, color: Color) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        Order::Tooltip,
        egui::Id::new("carried drop"),
    ));
    painter.circle(at, DROP_RADIUS, color32(color), Stroke::new(2.0, ON_CHROME));
    painter.circle_stroke(at, DROP_RADIUS + 2.0, Stroke::new(1.0, CHROME));
}
