//! The popover: a panel that drops down over the canvas from under one end of the top bar,
//! above everything else: under the right end for the tools (colour, layers, brushes), under the
//! left end for Actions, each under its button. It closes with one tap outside it; the screen
//! that opens it decides when (ARCHITECTURE section 10).

use egui::{Align2, Frame, Margin, Order, Stroke, Vec2};

use crate::components::TOP_BAR_HEIGHT;
use crate::design::{CHROME, CORNER, OUTLINE, PANEL_WIDTH, SPACE};

/// Shows the popover `id` with `add_contents` laid out top to bottom, `PANEL_WIDTH` wide, its
/// right edge one space from the screen's and its top one space under the top bar. Returns the
/// panel's rectangle in points, which nothing under it may take a contact from.
pub fn popover(
    ctx: &egui::Context,
    id: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    panel(ctx, id, (Align2::RIGHT_TOP, -SPACE), add_contents)
}

/// As `popover`, its left edge one space from the screen's.
pub fn popover_left(
    ctx: &egui::Context,
    id: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    panel(ctx, id, (Align2::LEFT_TOP, SPACE), add_contents)
}

fn panel(
    ctx: &egui::Context,
    id: &str,
    (corner, inset): (Align2, f32),
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    egui::Area::new(egui::Id::new(id))
        .order(Order::Foreground)
        .movable(false)
        // Whole in the frame it opens: feedback within one frame.
        .fade_in(false)
        .anchor(corner, Vec2::new(inset, TOP_BAR_HEIGHT + SPACE))
        .show(ctx, |ui| {
            Frame::new()
                .fill(CHROME)
                .stroke(Stroke::new(1.0, OUTLINE))
                .corner_radius(CORNER)
                .inner_margin(Margin::same(SPACE as i8))
                .show(ui, |ui| {
                    let inner = PANEL_WIDTH - 2.0 * SPACE;
                    ui.set_width(inner);
                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
                    add_contents(ui);
                });
        })
        .response
        .rect
}
