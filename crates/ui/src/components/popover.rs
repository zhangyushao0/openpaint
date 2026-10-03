//! The popover: a panel over the canvas that grows out of its island button (D44, the approved
//! demo): under the right island for the tools (colour, layers, brushes), under the left one for
//! Actions. It shares one ink outline with its island while it grows, finishes as paper under
//! the islands, and goes back into its button when it closes; a switch to another panel lets the
//! old one retract into its own button first. It closes with one tap outside it; the screen
//! that opens it decides when (ARCHITECTURE section 10). The screen only says whether it is
//! open; it is drawn while it closes too, taking no contacts then.

use egui::{Align2, Id, Margin, Order, Rect, Vec2};

use crate::components::UNDER_ISLANDS;
use crate::components::chrome::{self, Source};
use crate::design::{PANEL_WIDTH, SPACE, surface_frame};

/// The button a popover grows out of: the island it lies on and its rectangle, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opener {
    pub island: &'static str,
    pub button: Rect,
}

/// Shows the popover `id`, grown from `opener` while `open` and going back into it once closed,
/// with `add_contents` laid out top to bottom, `PANEL_WIDTH` wide, its right edge one space from
/// the screen's and its top one space under the islands. Returns the open panel's rectangle in
/// points, which nothing under it may take a contact from; `None` while it is closed.
pub fn popover(
    ctx: &egui::Context,
    id: &str,
    (opener, open): (Opener, bool),
    add_contents: impl FnOnce(&mut egui::Ui),
) -> Option<Rect> {
    panel(
        ctx,
        id,
        (Align2::RIGHT_TOP, -SPACE),
        (opener, open),
        add_contents,
    )
}

/// As `popover`, its left edge one space from the screen's.
pub fn popover_left(
    ctx: &egui::Context,
    id: &str,
    (opener, open): (Opener, bool),
    add_contents: impl FnOnce(&mut egui::Ui),
) -> Option<Rect> {
    panel(
        ctx,
        id,
        (Align2::LEFT_TOP, SPACE),
        (opener, open),
        add_contents,
    )
}

fn panel(
    ctx: &egui::Context,
    id: &str,
    (corner, inset): (Align2, f32),
    (opener, open): (Opener, bool),
    add_contents: impl FnOnce(&mut egui::Ui),
) -> Option<Rect> {
    let id = Id::new(id);
    let source = Source::Button {
        island: Id::new(opener.island),
        button: opener.button,
    };
    let showing = chrome::declare(ctx, id, source, Order::Foreground, open)?;
    let shown = egui::Area::new(id)
        .order(Order::Foreground)
        .movable(false)
        .interactable(open)
        .fade_in(false)
        .anchor(corner, Vec2::new(inset, UNDER_ISLANDS))
        .show(ctx, |ui| {
            surface_frame(Margin::same(SPACE as i8)).show(ui, |ui| {
                ui.multiply_opacity(showing.reveal);
                let inner = PANEL_WIDTH - 2.0 * SPACE;
                ui.set_width(inner);
                ui.spacing_mut().item_spacing = Vec2::splat(0.0);
                add_contents(ui);
            });
        });
    let rect = shown.response.rect;
    chrome::laid_out(ctx, id, rect, shown.response.layer_id, None);
    open.then_some(rect)
}
