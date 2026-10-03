//! The fill bar: a pill in the middle of the top band while a colour is dropped (what to do, and
//! the threshold while it is adjusted) and right after (the offer to keep filling), with at
//! most one text button. At fit it lies on the desk above the canvas, between the islands.
//! Nothing under it takes a contact that starts on it. Beside it, the drop the hand carries: a
//! disc of the colour, ringed so it shows on any canvas.

use egui::{Order, Pos2, Stroke};
use openpaint_core::Color;

use crate::components::{Place, UNDER_ISLANDS, color32, pill, text_button};
use crate::design::{INK, PANEL};

/// Radius of the carried drop, in points.
const DROP_RADIUS: f32 = 12.0;

/// Shows `text` and, with `action`, a button reading it, in the middle of the top band, or just
/// under the islands while the band's middle is `taken` (by the selection bar). Returns the
/// bar's rectangle in points and whether the button was clicked.
pub fn fill_bar(
    ctx: &egui::Context,
    text: &str,
    action: Option<&str>,
    taken: bool,
) -> (egui::Rect, bool) {
    let mut place = Place::top_middle();
    if taken {
        place.offset.y = UNDER_ISLANDS;
    }
    let (clicked, rect) = pill(ctx, "fill bar", place, |ui| {
        ui.label(text);
        action.is_some_and(|label| text_button(ui, label, true).clicked())
    });
    (rect, clicked)
}

/// The drop of `color` the hand carries, centred on `at` (points), over everything.
pub fn carried_drop(ctx: &egui::Context, at: Pos2, color: Color) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        Order::Tooltip,
        egui::Id::new("carried drop"),
    ));
    painter.circle(at, DROP_RADIUS, color32(color), Stroke::new(1.5, INK));
    painter.circle_stroke(at, DROP_RADIUS + 1.5, Stroke::new(1.5, PANEL));
}
