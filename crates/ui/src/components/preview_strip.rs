//! The preview strip (S46): a strip of white paper across a panel with the panel's outline,
//! showing a picture the renderer made of it (a brush's sample stroke and what the pen drew
//! there), and Clear, a bare word at its top right corner, grey while there is nothing to clear.
//! The pen's strokes on it are routed from the raw input, not here; a finger on it does
//! nothing.

use egui::epaint::RectShape;
use egui::{Align, Color32, Layout, Rect, Sense, Stroke, StrokeKind, UiBuilder, Vec2};

use crate::components::{controls, text_button};
use crate::design::{CORNER, PANEL_RULE, PANEL_RULE_WIDTH, SPACE, TOUCH_TARGET};

/// What the strip did this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripResponse {
    /// The strip, in points.
    pub rect: Rect,
    /// The Clear word, in points: the pen's strokes start outside it.
    pub clear_rect: Rect,
    pub cleared: bool,
}

/// Shows the strip, `height` points high across the panel: `picture` once there is one, and
/// Clear, available with `strokes` to clear.
pub fn preview_strip(
    ui: &mut egui::Ui,
    (picture, strokes): (Option<egui::TextureId>, bool),
    height: f32,
) -> StripResponse {
    let size = Vec2::new(ui.available_width(), height);
    // Takes the hand, so a finger on it moves nothing under it.
    let (rect, _) = ui.allocate_exact_size(size, Sense::click_and_drag());
    controls::register(ui, "Preview", rect);
    if ui.is_rect_visible(rect) {
        let paper = RectShape::filled(rect, CORNER, Color32::WHITE);
        let shape = match picture {
            Some(texture) => {
                let whole = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                paper.with_texture(texture, whole)
            }
            None => paper,
        };
        let painter = ui.painter();
        painter.add(shape);
        let outline = Stroke::new(PANEL_RULE_WIDTH, PANEL_RULE);
        painter.rect_stroke(rect, CORNER, outline, StrokeKind::Inside);
    }
    let corner = Rect::from_min_max(
        egui::pos2(rect.right() - 12.0 * SPACE, rect.top()),
        egui::pos2(rect.right() - SPACE / 2.0, rect.top() + TOUCH_TARGET),
    );
    let mut clear = ui.new_child(
        UiBuilder::new()
            .max_rect(corner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let response = text_button(&mut clear, "Clear", strokes);
    StripResponse {
        rect,
        clear_rect: response.rect,
        cleared: response.clicked(),
    }
}
