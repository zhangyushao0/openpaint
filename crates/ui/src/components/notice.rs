//! The notice: a short message after the alert icon, then a divider and the one action that
//! answers it as a bare word, laid out left to right inside a pill in the top band. It floats
//! over the desk at fit, so it never moves the canvas and hardly covers it.

use egui::{Sense, Vec2};

use crate::components::{divider, text_button};
use crate::design::{ICON_SIZE, INK, Icon, SPACE, paint_icon};
use crate::text::{OK, Words};

/// Shows the alert icon, `message` and a button reading `action`; only an `enabled` action can
/// be clicked. Returns the button's response.
pub fn notice<'a>(
    ui: &mut egui::Ui,
    message: impl Into<Words<'a>>,
    action: impl Into<Words<'a>>,
    enabled: bool,
) -> egui::Response {
    shown(ui, Icon::Alert, (message.into(), action.into()), enabled)
}

/// As `notice`, for work that succeeded: a check mark instead of the alert, and OK.
pub fn confirmation<'a>(ui: &mut egui::Ui, message: impl Into<Words<'a>>) -> egui::Response {
    shown(ui, Icon::Done, (message.into(), OK.into()), true)
}

fn shown(
    ui: &mut egui::Ui,
    icon: Icon,
    (message, action): (Words<'_>, Words<'_>),
    enabled: bool,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE), Sense::hover());
    paint_icon(ui.painter(), icon, rect, INK);
    ui.label(message.shown(ui.ctx()));
    // As far from the message as the word's own margin keeps it from the line.
    ui.add_space(2.0 * SPACE);
    divider(ui);
    text_button(ui, action, enabled)
}
