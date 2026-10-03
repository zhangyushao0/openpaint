//! The side panel: a narrow column of chrome floating over the canvas at the left edge, centred
//! on the canvas area (Procreate's side bar). Nothing under it takes a contact that starts on
//! it. The edge it sits on is one constant, so mirroring it for the left hand is one change.

use egui::{Align2, Frame, Margin, Order, Stroke, Vec2};

use crate::components::TOP_BAR_HEIGHT;
use crate::design::{CHROME, CORNER, OUTLINE, SPACE};

/// The screen edge the side panel sits on, and its gap from that edge (towards the middle).
/// Mirrored for the left hand: `RIGHT_TOP` and `-SPACE`.
const EDGE: (Align2, f32) = (Align2::LEFT_TOP, SPACE);

/// Shows the side panel `id`, about `height` points tall, with `add_contents` laid out top to
/// bottom, one space from the screen's edge, centred on the area under the top bar; where that
/// area is too short, from one space under the top bar down, so it never covers the bar.
/// Returns its rectangle in points.
pub fn side_panel(
    ctx: &egui::Context,
    id: &str,
    height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    let (edge, inward) = EDGE;
    let screen = ctx.content_rect().height();
    let centred = (screen + TOP_BAR_HEIGHT - height) / 2.0;
    let top = centred.max(TOP_BAR_HEIGHT + SPACE);
    egui::Area::new(egui::Id::new(id))
        .order(Order::Middle)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .anchor(edge, Vec2::new(inward, top))
        .show(ctx, |ui| {
            Frame::new()
                .fill(CHROME)
                .stroke(Stroke::new(1.0, OUTLINE))
                .corner_radius(CORNER)
                .inner_margin(Margin::same(SPACE as i8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(SPACE);
                    ui.vertical_centered(add_contents);
                });
        })
        .response
        .rect
}
