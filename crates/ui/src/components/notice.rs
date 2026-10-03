//! The notice: a short message at the right end of the top bar, after the alert icon, with the
//! one action that answers it. It sits in the chrome, so it never covers or moves the canvas.

use egui::{Align, Layout, Sense, Vec2};

use crate::components::text_button;
use crate::design::{ICON_SIZE, Icon, ON_CHROME, paint_icon};

/// Shows `message` and a button reading `action` at the right end of the current row; only an
/// `enabled` action can be clicked. Returns the button's response.
pub fn notice(ui: &mut egui::Ui, message: &str, action: &str, enabled: bool) -> egui::Response {
    shown(ui, Icon::Alert, message, action, enabled)
}

/// As `notice`, for work that succeeded: a check mark instead of the alert, and OK.
pub fn confirmation(ui: &mut egui::Ui, message: &str) -> egui::Response {
    shown(ui, Icon::Done, message, "OK", true)
}

fn shown(
    ui: &mut egui::Ui,
    icon: Icon,
    message: &str,
    action: &str,
    enabled: bool,
) -> egui::Response {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let response = text_button(ui, action, enabled);
        ui.label(message);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE), Sense::hover());
        paint_icon(ui.painter(), icon, rect, ON_CHROME);
        response
    })
    .inner
}
