//! The list row: one touch target high across a panel, a name at its left, an optional detail
//! before an icon button at its right end. Tapping the row chooses it; the chosen row carries
//! the active mark. The icon button acts on the row without choosing it.

use egui::{Align2, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::animate::{animate_from, settle};
use crate::components::controls;
use crate::design::motion::{ACKNOWLEDGE, SETTLE};
use crate::design::{
    ICON_SIZE, INK, INK_ACTIVE, INK_DISABLED, INK_SECONDARY, Icon, Outline, SPACE, TOUCH_TARGET,
    paint_icon, paint_mark, paint_pressed,
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
    pub selected: bool,
    pub icon: Icon,
    /// Names the icon button for accessibility.
    pub icon_label: &'a str,
    pub enabled: bool,
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
    // Rows that moved (a reorder, a row added or deleted above) slide to their new place; the
    // place itself takes the contact at once. Measured in the list, so scrolling never lags.
    let place = rect.top() - ui.max_rect().top();
    let offset = settle(ui.ctx(), row.key, place, SETTLE);
    let indent = row.indent + settle(ui.ctx(), row.key.with("indent"), row.indent, SETTLE);
    let button = Rect::from_min_size(
        rect.right_top() - Vec2::new(TOUCH_TARGET, 0.0),
        Vec2::splat(TOUCH_TARGET),
    );
    // Added after the row, so it takes the contacts that land on it.
    let icon = ui.interact(button, response.id.with("icon"), sense);
    response.widget_info(|| {
        WidgetInfo::selected(WidgetType::Button, row.enabled, row.selected, row.label)
    });
    icon.widget_info(|| WidgetInfo::labeled(WidgetType::Button, row.enabled, row.icon_label));
    // The icon after the row: it lies on top.
    controls::register(ui, row.label, rect);
    controls::register(ui, row.icon_label, button);
    // A row seen for the first time (a layer just added) fades in where it lands.
    let appear = animate_from(ui.ctx(), row.key.with("appear"), 0.0, 1.0, ACKNOWLEDGE);
    let shown = rect.translate(Vec2::new(0.0, offset));
    if ui.is_rect_visible(shown) {
        let mut painter = ui.painter().clone();
        painter.multiply_opacity(appear);
        paint(ui, &painter, (row, indent), shown, (&response, &icon));
    }
    RowResponse {
        row: response,
        icon: icon.on_hover_text(row.icon_label),
    }
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
