//! The status: a short message at the right end of the top bar while something runs in the
//! background (an export being written, an image being opened), after a turning spinner. It
//! asks nothing of the user and sits in the chrome, so drawing goes on.

use egui::{Align, Layout, Vec2};

use crate::design::{ICON_SIZE, ON_CHROME, SPACE};

/// Shows the spinner and `message` at the right end of the current row.
pub fn status(ui: &mut egui::Ui, message: &str) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_space(SPACE);
        ui.label(message);
        ui.add_sized(
            Vec2::splat(ICON_SIZE),
            egui::Spinner::new().size(ICON_SIZE).color(ON_CHROME),
        );
    });
}
