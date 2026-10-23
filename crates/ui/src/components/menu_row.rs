//! The menu row: one touch target high across a panel, an action's name at its left and an
//! optional note at its right end, dimmer. Tapping it does the action; pressed, a faint wash
//! fills the row's outline under the hand; unavailable, it dims and taps go nowhere. Words show
//! in the pass's language; a row is named by its English (`text.rs`).

use egui::{Align, Align2, Layout, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::controls;
use crate::design::{
    ICON_SIZE, INK, INK_DISABLED, INK_SECONDARY, Icon, Outline, SPACE, TOUCH_TARGET, paint_icon,
    paint_pressed,
};
use crate::text::Words;

/// A row one touch target high across the panel, named `name` (`info` for accessibility),
/// taking taps only while `enabled`, the press wash in its outline under the hand. Returns its
/// response and whether it lies in sight (whether to paint the rest).
pub(crate) fn tap_row(
    ui: &mut egui::Ui,
    name: &str,
    enabled: bool,
    info: impl Fn() -> WidgetInfo,
) -> (egui::Response, bool) {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(info);
    controls::register(ui, name, rect);
    let shown = ui.is_rect_visible(rect);
    if shown && enabled && response.is_pointer_button_down_on() {
        paint_pressed(ui.painter(), Outline::row(rect));
    }
    (response, shown)
}

/// Shows a row reading `label`, with `note` (`Words::EMPTY` for none) at its right end.
pub fn menu_row<'a>(
    ui: &mut egui::Ui,
    label: impl Into<Words<'a>>,
    note: impl Into<Words<'a>>,
    enabled: bool,
) -> egui::Response {
    let (label, note) = (label.into(), note.into());
    let (shown, note) = (label.shown(ui.ctx()), note.shown(ui.ctx()));
    let info = || WidgetInfo::labeled(WidgetType::Button, enabled, shown);
    let (response, visible) = tap_row(ui, label.id(), enabled, info);
    if !visible {
        return response;
    }
    let (rect, painter) = (response.rect, ui.painter());
    let (text, quiet) = if enabled {
        (INK, INK_SECONDARY)
    } else {
        (INK_DISABLED, INK_DISABLED)
    };
    let font = TextStyle::Body.resolve(ui.style());
    let left = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(left, Align2::LEFT_CENTER, shown, font, text);
    let small = TextStyle::Small.resolve(ui.style());
    let right = rect.right_center() - Vec2::new(2.0 * SPACE, 0.0);
    painter.text(right, Align2::RIGHT_CENTER, note, small, quiet);
    response
}

/// A command row (Add Mask, Merge Down): `icon` leading the words `label`, both in ink, and at
/// its right end `note` (empty for none: why it cannot act now, small and in the quieter ink),
/// so it reads as a thing to tap among the switches and labels around it. Pressed and
/// unavailable as `menu_row`.
pub fn command_row(
    ui: &mut egui::Ui,
    (icon, label, note): (Icon, Words<'_>, Words<'_>),
    enabled: bool,
) -> egui::Response {
    let (shown, note) = (label.shown(ui.ctx()), note.shown(ui.ctx()));
    let info = || WidgetInfo::labeled(WidgetType::Button, enabled, shown);
    let (response, visible) = tap_row(ui, label.id(), enabled, info);
    if !visible {
        return response;
    }
    let (rect, painter) = (response.rect, ui.painter());
    let color = if enabled { INK } else { INK_DISABLED };
    let lead = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    let square = Rect::from_min_size(
        lead - Vec2::new(0.0, ICON_SIZE / 2.0),
        Vec2::splat(ICON_SIZE),
    );
    paint_icon(painter, icon, square, color);
    let font = TextStyle::Body.resolve(ui.style());
    let at = lead + Vec2::new(ICON_SIZE + SPACE, 0.0);
    painter.text(at, Align2::LEFT_CENTER, shown, font, color);
    let small = TextStyle::Small.resolve(ui.style());
    let right = rect.right_center() - Vec2::new(2.0 * SPACE, 0.0);
    painter.text(right, Align2::RIGHT_CENTER, note, small, INK_SECONDARY);
    response
}

/// A hint across a panel, one touch target high: what something does, in the quieter ink, so the
/// bare words that are commands stay what reads as tappable.
pub fn hint<'a>(ui: &mut egui::Ui, text: impl Into<Words<'a>>) {
    let text = text.into().shown(ui.ctx());
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let font = TextStyle::Body.resolve(ui.style());
    let at = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    ui.painter()
        .text(at, Align2::LEFT_CENTER, text, font, INK_SECONDARY);
}

/// A panel's name over its rows, two spaces in, one touch target high.
pub fn panel_heading<'a>(ui: &mut egui::Ui, name: impl Into<Words<'a>>) {
    let name = name.into().shown(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading(name);
    });
}

/// As `panel_heading`, with `trailing` laid out from the row's right end (a command, a field).
pub fn panel_heading_with<'a, R>(
    ui: &mut egui::Ui,
    name: impl Into<Words<'a>>,
    trailing: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let name = name.into().shown(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading(name);
        ui.with_layout(Layout::right_to_left(Align::Center), trailing)
            .inner
    })
    .inner
}

/// A section's name over its rows: small and quiet, so the rows stay what the eye reads.
pub fn section<'a>(ui: &mut egui::Ui, title: impl Into<Words<'a>>) {
    let title = title.into().shown(ui.ctx());
    let size = Vec2::new(ui.available_width(), 4.0 * SPACE);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let font = TextStyle::Small.resolve(ui.style());
    let at = rect.left_bottom() + Vec2::new(2.0 * SPACE, -SPACE);
    ui.painter()
        .text(at, Align2::LEFT_BOTTOM, title, font, INK_SECONDARY);
}

/// A setting's name on its own line (over its words), in ink, or pencil grey unless `enabled`.
pub fn label_row<'a>(ui: &mut egui::Ui, name: impl Into<Words<'a>>, enabled: bool) {
    let name = name.into().shown(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.colored_label(if enabled { INK } else { INK_DISABLED }, name);
    });
}
