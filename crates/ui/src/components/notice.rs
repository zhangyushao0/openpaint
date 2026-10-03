//! The notice: a short message after the alert icon, with the one action that answers it, laid
//! out left to right inside a pill in the top band. It floats over the desk at fit, so it never
//! moves the canvas and hardly covers it.

use egui::{Sense, Vec2};

use crate::components::text_button;
use crate::design::{ICON_SIZE, INK, Icon, paint_icon};

/// Shows the alert icon, `message` and a button reading `action`; only an `enabled` action can
/// be clicked. Returns the button's response.
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
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE), Sense::hover());
    paint_icon(ui.painter(), icon, rect, INK);
    ui.label(message);
    text_button(ui, action, enabled)
}
