//! The dialog: a panel over a dimmed screen that must be answered before anything else. It is
//! used only to confirm what cannot be undone and to type (ARCHITECTURE section 10). A tap on
//! the dimmed screen around it cancels.

use egui::{Align2, Frame, Margin, Order, Stroke, Vec2};

use crate::components::TOP_BAR_HEIGHT;
use crate::design::{BACKDROP, CHROME, CORNER, DIALOG_WIDTH, OUTLINE, SPACE};

/// Where a dialog sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// In the middle of the screen.
    Centre,
    /// Under the top bar, so the soft keyboard rising from the bottom never covers it.
    Top,
}

/// Shows the dialog `id` with `add_contents` laid out top to bottom, `DIALOG_WIDTH` wide.
/// Returns what the contents returned and whether the user tapped outside, which cancels.
pub fn dialog<T>(
    ctx: &egui::Context,
    id: &str,
    placement: Placement,
    add_contents: impl FnOnce(&mut egui::Ui) -> T,
) -> (T, bool) {
    let (align, offset) = match placement {
        Placement::Centre => (Align2::CENTER_CENTER, Vec2::ZERO),
        Placement::Top => (
            Align2::CENTER_TOP,
            Vec2::new(0.0, TOP_BAR_HEIGHT + 2.0 * SPACE),
        ),
    };
    let area = egui::Modal::default_area(egui::Id::new(id))
        .order(Order::Foreground)
        // Whole in the frame it opens: feedback within one frame.
        .fade_in(false)
        .anchor(align, offset);
    let frame = Frame::new()
        .fill(CHROME)
        .stroke(Stroke::new(1.0, OUTLINE))
        .corner_radius(CORNER)
        .inner_margin(Margin::same((2.0 * SPACE) as i8));
    let response = egui::Modal::new(egui::Id::new(id))
        .area(area)
        .frame(frame)
        .backdrop_color(BACKDROP)
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH - 4.0 * SPACE);
            ui.spacing_mut().item_spacing = Vec2::splat(SPACE);
            add_contents(ui)
        });
    let outside = response.backdrop_response.clicked();
    (response.inner, outside)
}
