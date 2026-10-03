//! The island: a small piece of paper floating over the desk, outlined in ink, holding one row
//! of controls or a short message. Nothing spans the screen's width (D40): the canvas's controls
//! are islands at the top corners (Gallery and Actions at the left; Brush, Eraser, Layers and
//! the colour at the right), what comes and goes (a notice, the fill bar, the view's zoom) is a
//! pill in the band between them, and the side bar is an island at the left edge. At fit the
//! top islands lie on the desk, above the canvas.

use egui::{Align, Align2, Layout, Margin, Order, Rect, Vec2};

use crate::design::{SPACE, TOUCH_TARGET, panel_frame};

/// The islands' top edge, in points from the screen's top.
pub const ISLAND_TOP: f32 = SPACE;
/// The band along the screen's top that the islands float in, in points: the canvas's fit
/// leaves it to the desk.
pub const TOP_BAND: f32 = TOUCH_TARGET;
/// Under the top islands, in points: where popovers and dialogs begin.
pub const UNDER_ISLANDS: f32 = ISLAND_TOP + TOUCH_TARGET + SPACE;

/// Where an island sits: the screen corner or edge it is anchored to and its offset from there,
/// in points.
#[derive(Debug, Clone, Copy)]
pub struct Place {
    pub anchor: Align2,
    pub offset: Vec2,
}

impl Place {
    /// At the top left corner.
    #[must_use]
    pub fn top_left() -> Self {
        Self {
            anchor: Align2::LEFT_TOP,
            offset: Vec2::new(SPACE, ISLAND_TOP),
        }
    }

    /// At the top right corner, `from_right` points in from the right edge.
    #[must_use]
    pub fn top_right(from_right: f32) -> Self {
        Self {
            anchor: Align2::RIGHT_TOP,
            offset: Vec2::new(-(SPACE + from_right), ISLAND_TOP),
        }
    }

    /// In the middle of the top band.
    #[must_use]
    pub fn top_middle() -> Self {
        Self {
            anchor: Align2::CENTER_TOP,
            offset: Vec2::new(0.0, ISLAND_TOP),
        }
    }
}

/// Shows the island `id` at `place`, one touch target high, with `add_contents` laid out left
/// to right. Returns what the contents returned and the island's rectangle in points, which
/// nothing under it may take a contact from.
pub fn island<R>(
    ctx: &egui::Context,
    id: &str,
    place: Place,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, Rect) {
    let shown = egui::Area::new(egui::Id::new(id))
        .order(Order::Middle)
        .movable(false)
        // Whole in the frame it opens: feedback within one frame.
        .fade_in(false)
        .anchor(place.anchor, place.offset)
        .show(ctx, |ui| {
            panel_frame(Margin::symmetric((SPACE / 2.0) as i8, 0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(SPACE / 2.0, 0.0);
                    ui.set_height(TOUCH_TARGET);
                    ui.with_layout(Layout::left_to_right(Align::Center), add_contents)
                        .inner
                })
                .inner
        });
    (shown.inner, shown.response.rect)
}

/// A pill: an island with a message and at most one button, its text two spaces from its ends.
pub fn pill<R>(
    ctx: &egui::Context,
    id: &str,
    place: Place,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, Rect) {
    island(ctx, id, place, |ui| {
        ui.add_space(1.5 * SPACE);
        let inner = add_contents(ui);
        ui.add_space(SPACE);
        inner
    })
}
