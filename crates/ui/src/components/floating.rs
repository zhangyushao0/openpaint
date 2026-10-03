//! The floating window: a panel over the canvas that the hand puts where it likes (Procreate's
//! reference window). A title bar one touch target high, its name at the left and icon buttons
//! at the right end; dragging the bar anywhere but on a button moves the window. Under the bar,
//! the body. Nothing under the window takes a contact that starts on it.

use egui::{Align, Align2, Frame, Layout, Order, Pos2, Rect, Sense, Stroke, TextStyle, UiBuilder};
use egui::{Vec2, WidgetInfo, WidgetType};

use crate::design::{CHROME, CORNER, ON_CHROME, OUTLINE, SPACE, TOUCH_TARGET};

/// What the window gave this frame.
pub struct Floating<B, T> {
    /// The whole window, in points.
    pub rect: Rect,
    /// How far the hand dragged the title bar this frame, in points.
    pub moved: Vec2,
    /// What the buttons and the body returned.
    pub buttons: B,
    pub body: T,
}

/// Shows the window `id` with its top-left corner at `origin` (points), `width` points wide,
/// named `title`; `add_buttons` lays out right to left at the bar's right end and `add_body`
/// fills the rest, top to bottom.
pub fn floating<B, T>(
    ctx: &egui::Context,
    id: &str,
    (origin, width): (Pos2, f32),
    title: &str,
    add_buttons: impl FnOnce(&mut egui::Ui) -> B,
    add_body: impl FnOnce(&mut egui::Ui) -> T,
) -> Floating<B, T> {
    let shown = egui::Area::new(egui::Id::new(id))
        .order(Order::Middle)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .fixed_pos(origin)
        .show(ctx, |ui| {
            Frame::new()
                .fill(CHROME)
                .stroke(Stroke::new(1.0, OUTLINE))
                .corner_radius(CORNER)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let size = Vec2::new(width, TOUCH_TARGET);
                    let (bar, drag) = ui.allocate_exact_size(size, Sense::drag());
                    drag.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, title));
                    let font = TextStyle::Body.resolve(ui.style());
                    let left = bar.left_center() + Vec2::new(2.0 * SPACE, 0.0);
                    ui.painter()
                        .text(left, Align2::LEFT_CENTER, title, font, ON_CHROME);
                    let mut row = ui.new_child(
                        UiBuilder::new()
                            .max_rect(bar)
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    row.spacing_mut().item_spacing = Vec2::ZERO;
                    let buttons = add_buttons(&mut row);
                    let body = add_body(ui);
                    (drag.drag_delta(), buttons, body)
                })
                .inner
        });
    let (moved, buttons, body) = shown.inner;
    Floating {
        rect: shown.response.rect,
        moved,
        buttons,
        body,
    }
}
