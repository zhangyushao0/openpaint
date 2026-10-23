//! The status: a turning spinner and a short message, left to right inside a pill in the top
//! band, while something runs in the background (an export being written, an image being
//! opened). It asks nothing of the user, so drawing goes on.

use egui::Vec2;

use crate::design::{ICON_SIZE, INK};
use crate::text::Words;

/// Shows the spinner and `message`.
pub fn status<'a>(ui: &mut egui::Ui, message: impl Into<Words<'a>>) {
    ui.add_sized(
        Vec2::splat(ICON_SIZE),
        egui::Spinner::new().size(ICON_SIZE).color(INK),
    );
    ui.label(message.into().shown(ui.ctx()));
}
