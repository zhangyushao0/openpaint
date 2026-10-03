//! The brush row: a brush's name at the left and its stroke preview at the right, across a
//! panel. Tapping the row chooses the brush; the chosen row is highlighted.

use egui::{Align2, Color32, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::design::{BRUSH_PREVIEW, CORNER, ON_CHROME, PRESSED, SELECTED, SPACE};

/// Shows one row: `name`, and `preview` (a texture of `BRUSH_PREVIEW` points) when it is loaded.
pub fn brush_row(
    ui: &mut egui::Ui,
    name: &str,
    preview: Option<egui::TextureId>,
    selected: bool,
) -> egui::Response {
    let size = Vec2::new(ui.available_width(), BRUSH_PREVIEW.y + 2.0 * SPACE);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, name));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let face = if response.is_pointer_button_down_on() {
        Some(PRESSED)
    } else if selected {
        Some(SELECTED)
    } else {
        None
    };
    if let Some(face) = face {
        painter.rect_filled(rect.shrink2(Vec2::new(0.0, SPACE / 4.0)), CORNER, face);
    }
    let font = TextStyle::Body.resolve(ui.style());
    let left = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(left, Align2::LEFT_CENTER, name, font, ON_CHROME);
    if let Some(texture) = preview {
        let right = rect.right_center() - Vec2::new(SPACE + BRUSH_PREVIEW.x / 2.0, 0.0);
        let picture = Rect::from_center_size(right, BRUSH_PREVIEW);
        let whole = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture, picture, whole, Color32::WHITE);
    }
    response
}
