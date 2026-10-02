//! The notice: a short message at the right end of the top bar, after the alert icon, with the
//! one action that answers it. It sits in the chrome, so it never covers or moves the canvas.

use egui::{Align, Layout, Sense, Vec2};

use crate::components::text_button;
use crate::design::{ICON_SIZE, Icon, ON_CHROME, paint_icon};

/// Shows `message` and a button reading `action` at the right end of the current row; only an
/// `enabled` action can be clicked. Returns the button's response.
pub fn notice(ui: &mut egui::Ui, message: &str, action: &str, enabled: bool) -> egui::Response {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let response = text_button(ui, action, enabled);
        ui.label(message);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE), Sense::hover());
        paint_icon(ui.painter(), Icon::Alert, rect, ON_CHROME);
        response
    })
    .inner
}
