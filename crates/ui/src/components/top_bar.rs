//! The top bar: one strip of chrome across the top of the screen, above the canvas.

use egui::{Frame, Margin};

use crate::design::{CHROME, SPACE, TOUCH_TARGET};

/// Top bar height in points: exactly one touch target.
pub const TOP_BAR_HEIGHT: f32 = TOUCH_TARGET;

/// Shows the top bar with `add_contents` laid out left to right, centred vertically.
pub fn top_bar(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let side = (2.0 * SPACE) as i8;
    egui::Panel::top("top_bar")
        .exact_size(TOP_BAR_HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(CHROME)
                .inner_margin(Margin::symmetric(side, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(add_contents);
        });
}
