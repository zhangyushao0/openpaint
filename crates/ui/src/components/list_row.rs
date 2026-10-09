//! The list row: one touch target high across a panel, a name at its left, an optional detail
//! before an icon button at its right end. Tapping the row chooses it; the chosen row carries
//! the active mark. The icon button acts on the row without choosing it. A row may show a small
//! picture before its name (a layer's, S38) and may be carried by the hand (`lifted`).

use egui::{Align2, Color32, Order, Rect, Sense, Stroke, StrokeKind, TextStyle, Vec2, WidgetInfo};

use crate::components::animate::{animate_from, hold_at, settle};
use crate::components::controls;
use crate::design::motion::{ACKNOWLEDGE, SETTLE};
use crate::design::{
    CHECKER, DIVIDER, ICON_SIZE, INK, INK_ACTIVE, INK_DISABLED, INK_SECONDARY, Icon, Outline,
    PANEL, SPACE, TOUCH_TARGET, paint_icon, paint_mark, paint_pressed,
};

/// What one row shows.
#[derive(Debug, Clone, Copy)]
pub struct ListRow<'a> {
    /// Stays the same for one row's subject from frame to frame, wherever the row moves.
    pub key: egui::Id,
    pub label: &'a str,
    /// Shown dimmer, right-aligned before the icon; empty for none.
    pub detail: &'a str,
    /// How far the row's content starts right of its usual place (a layer in a group); a row
    /// whose indent changes slides there (settle).
    pub indent: f32,
    /// A small icon before the label (a group's chevron, a Clipping Mask's arrow).
    pub lead: Option<Icon>,
    /// A picture before the label, after the lead.
    pub picture: Option<RowPicture>,
    pub selected: bool,
    pub icon: Icon,
    /// Names the icon button for accessibility.
    pub icon_label: &'a str,
    pub enabled: bool,
    /// Carried by the hand: drawn this far below its place (points), on paper over the other
    /// rows, following the hand 1:1; let go, it slides from there to its place (settle).
    pub lifted: Option<f32>,
}

/// A row's picture: `size` points (inside `design::LAYER_PICTURE`) on a light checker, the
/// texture over it once there is one.
#[derive(Debug, Clone, Copy)]
pub struct RowPicture {
    pub texture: Option<egui::TextureId>,
    pub size: Vec2,
}

/// The row's two touch targets.
pub struct RowResponse {
    pub row: egui::Response,
    pub icon: egui::Response,
}

pub fn list_row(ui: &mut egui::Ui, row: &ListRow<'_>) -> RowResponse {
    let sense = if row.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let offset = row_offset(ui, row.key, rect, row.lifted);
    let indent = row.indent + settle(ui.ctx(), row.key.with("indent"), row.indent, SETTLE);
    let button = Rect::from_min_size(
        rect.right_top() - Vec2::new(TOUCH_TARGET, 0.0),
        Vec2::splat(TOUCH_TARGET),
    );
    // Added after the row, so it takes the contacts that land on it.
    let icon = ui.interact(button, response.id.with("icon"), sense);
    response.widget_info(|| {
        WidgetInfo::selected(
            egui::WidgetType::Button,
            row.enabled,
            row.selected,
            row.label,
        )
    });
    icon.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, row.enabled, row.icon_label));
    // The icon after the row: it lies on top.
    controls::register(ui, row.label, rect);
    controls::register(ui, row.icon_label, button);
    // A row seen for the first time (a layer just added) fades in where it lands.
    let appear = animate_from(ui.ctx(), row.key.with("appear"), 0.0, 1.0, ACKNOWLEDGE);
    let shown = rect.translate(Vec2::new(0.0, offset));
    if ui.is_rect_visible(shown) || row.lifted.is_some() {
        let mut painter = row_painter(ui, row.key, shown, row.lifted.is_some());
        painter.multiply_opacity(appear);
        paint(ui, &painter, (row, indent), shown, (&response, &icon));
    }
    RowResponse {
        row: response,
        icon: icon.on_hover_text(row.icon_label),
    }
}

/// How far below its place (`rect`, as laid out) the row `key` shows: carried by the hand,
/// `lifted` points, exactly; else sliding from where it showed to its new place (a reorder, a
/// row added or deleted above), whose place takes the contact at once. Measured in the list, so
/// scrolling never lags. Shared by every list whose rows the hand carries.
pub(crate) fn row_offset(ui: &egui::Ui, key: egui::Id, rect: Rect, lifted: Option<f32>) -> f32 {
    let place = rect.top() - ui.max_rect().top();
    match lifted {
        Some(lift) => {
            hold_at(ui.ctx(), key, place + lift);
            lift
        }
        None => settle(ui.ctx(), key, place, SETTLE),
    }
}

/// The painter a row draws with at `shown`: while the hand carries it, over every other row (still
/// cut to the list) on paper in an ink outline.
pub(crate) fn row_painter(
    ui: &egui::Ui,
    key: egui::Id,
    shown: Rect,
    lifted: bool,
) -> egui::Painter {
    let painter = ui.painter().clone();
    if !lifted {
        return painter;
    }
    let painter = painter.with_layer_id(egui::LayerId::new(Order::Tooltip, key));
    painter.rect_filled(Outline::row(shown).rect, Outline::row(shown).corner, PANEL);
    paint_mark(&painter, Outline::row(shown), 1.0);
    painter
}

fn paint(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (row, indent): (&ListRow<'_>, f32),
    rect: Rect,
    (body, icon): (&egui::Response, &egui::Response),
) {
    if row.enabled && body.is_pointer_button_down_on() {
        paint_pressed(painter, Outline::row(rect));
    }
    if row.selected {
        paint_mark(painter, Outline::row(rect), 1.0);
    }
    let color = match (row.enabled, row.selected) {
        (false, _) => INK_DISABLED,
        (true, true) => INK_ACTIVE,
        (true, false) => INK,
    };
    let font = TextStyle::Body.resolve(ui.style());
    let mut left = rect.left_center() + Vec2::new(2.0 * SPACE + indent, 0.0);
    if let Some(lead) = row.lead {
        let square = Rect::from_min_size(
            left - Vec2::new(0.0, ICON_SIZE / 2.0),
            Vec2::splat(ICON_SIZE),
        );
        let lead_color = if row.enabled {
            INK_SECONDARY
        } else {
            INK_DISABLED
        };
        paint_icon(painter, lead, square, lead_color);
        left += Vec2::new(ICON_SIZE + SPACE / 2.0, 0.0);
    }
    if let Some(picture) = row.picture {
        let frame = Rect::from_min_size(left - Vec2::new(0.0, picture.size.y / 2.0), picture.size);
        paint_picture(painter, picture.texture, frame);
        left += Vec2::new(picture.size.x + SPACE, 0.0);
    }
    painter.text(left, Align2::LEFT_CENTER, row.label, font.clone(), color);
    let right = rect.right_center() - Vec2::new(TOUCH_TARGET, 0.0);
    let small = TextStyle::Small.resolve(ui.style());
    painter.text(
        right,
        Align2::RIGHT_CENTER,
        row.detail,
        small,
        INK_SECONDARY,
    );
    let button = Rect::from_center_size(
        right + Vec2::new(TOUCH_TARGET / 2.0, 0.0),
        Vec2::splat(TOUCH_TARGET),
    );
    if row.enabled && icon.is_pointer_button_down_on() {
        paint_pressed(painter, Outline::ring(button));
    }
    let color = if row.enabled { INK } else { INK_DISABLED };
    let square = Rect::from_center_size(button.center(), Vec2::splat(ICON_SIZE));
    paint_icon(painter, row.icon, square, color);
}

/// Side of a checker cell under a picture, in points.
const CHECKER_CELL: f32 = 5.5;

/// A picture in `frame`: white and light checker cells (what shows where it is empty), the
/// texture over them, a faint line round it.
fn paint_picture(painter: &egui::Painter, texture: Option<egui::TextureId>, frame: Rect) {
    painter.rect_filled(frame, 0.0, Color32::WHITE);
    let cells = (frame.size() / CHECKER_CELL).ceil();
    for row in 0..cells.y as u32 {
        for column in (row % 2..cells.x as u32).step_by(2) {
            let at = frame.min + Vec2::new(column as f32, row as f32) * CHECKER_CELL;
            let cell = Rect::from_min_size(at, Vec2::splat(CHECKER_CELL)).intersect(frame);
            painter.rect_filled(cell, 0.0, CHECKER);
        }
    }
    if let Some(texture) = texture {
        let whole = Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        painter.image(texture, frame, whole, Color32::WHITE);
    }
    painter.rect_stroke(frame, 0.0, Stroke::new(1.0, DIVIDER), StrokeKind::Outside);
}
