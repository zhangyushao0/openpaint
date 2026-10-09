//! The brush row: a brush's name at the left and its stroke preview at the right, across a
//! panel. Tapping the row chooses the brush; the chosen row carries the active mark. In a set
//! the hand orders (S47) the row may be carried by the hand as a layer row is (`lifted`), and
//! rows that moved slide to their new place (settle).

use egui::{Align2, Color32, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::controls;
use crate::components::list_row::{row_offset, row_painter};
use crate::design::{BRUSH_PREVIEW, INK, INK_ACTIVE, Outline, SPACE, paint_mark, paint_pressed};

/// The height of a brush row, in points.
pub const BRUSH_ROW: f32 = BRUSH_PREVIEW.y + 2.0 * SPACE;

/// What one brush row shows.
#[derive(Debug, Clone, Copy)]
pub struct BrushRowView<'a> {
    /// Stays the same for one brush in one list from frame to frame, wherever the row moves.
    pub key: egui::Id,
    pub name: &'a str,
    /// A texture of `BRUSH_PREVIEW` points, once it is loaded.
    pub preview: Option<egui::TextureId>,
    pub selected: bool,
    /// Carried by the hand: drawn this far below its place (points), on paper over the other
    /// rows; let go, it slides from there to its place.
    pub lifted: Option<f32>,
}

/// Shows one row; its press can lift it.
pub fn brush_row(ui: &mut egui::Ui, row: &BrushRowView<'_>) -> egui::Response {
    let size = Vec2::new(ui.available_width(), BRUSH_ROW);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let selected = row.selected;
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, row.name));
    controls::register(ui, row.name, rect);
    let shown = rect.translate(Vec2::new(0.0, row_offset(ui, row.key, rect, row.lifted)));
    if !ui.is_rect_visible(shown) && row.lifted.is_none() {
        return response;
    }
    let painter = row_painter(ui, row.key, shown, row.lifted.is_some());
    if response.is_pointer_button_down_on() {
        paint_pressed(&painter, Outline::row(shown));
    }
    if selected {
        paint_mark(&painter, Outline::row(shown), 1.0);
    }
    let font = TextStyle::Body.resolve(ui.style());
    let left = shown.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    let color = if selected { INK_ACTIVE } else { INK };
    painter.text(left, Align2::LEFT_CENTER, row.name, font, color);
    if let Some(texture) = row.preview {
        let right = shown.right_center() - Vec2::new(SPACE + BRUSH_PREVIEW.x / 2.0, 0.0);
        let picture = Rect::from_center_size(right, BRUSH_PREVIEW);
        let whole = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture, picture, whole, Color32::WHITE);
    }
    response
}
