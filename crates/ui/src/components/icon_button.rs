//! The icon button: one touch target on the chrome with an ink-line icon. Pressed, a faint wash
//! fills its ring's circle under the hand (never the wider touch target); unavailable, its icon
//! fades to pencil grey and taps go nowhere; the pen hovering over it shows its name. The
//! toggle form carries the active mark (an ink ring, never a filled block) while what it opens
//! is open; the choice form is one of a `choices` group, whose one ring slides to the icon
//! chosen; the ringed form stands in its ring at rest, the one round button of a bar.

use egui::{Rect, Sense, Vec2, WidgetInfo, WidgetType};

use crate::components::{Choices, animate, controls};
use crate::design::motion::ACKNOWLEDGE;
use crate::design::{
    ICON_SIZE, INK, INK_ACTIVE, INK_DISABLED, Icon, MARK_LINE, Outline, TOUCH_TARGET, paint_icon,
    paint_mark, paint_pressed,
};
use crate::text::Words;

/// Shows a button of one touch target with `icon`; `label` names it (the hover label in the
/// pass's language, the control by its English). Only an `enabled` button can be clicked.
pub fn icon_button<'a>(
    ui: &mut egui::Ui,
    icon: Icon,
    label: impl Into<Words<'a>>,
    enabled: bool,
) -> egui::Response {
    let label = label.into();
    let response = button(ui, icon, label, enabled, false);
    response.on_hover_text(label.shown(ui.ctx()))
}

/// An icon button that shows whether what it opens is `on`.
pub fn icon_toggle<'a>(
    ui: &mut egui::Ui,
    icon: Icon,
    label: impl Into<Words<'a>>,
    on: bool,
) -> egui::Response {
    let label = label.into();
    let response = button(ui, icon, label, true, on);
    let mark = animate(
        ui.ctx(),
        response.id.with("mark"),
        f32::from(u8::from(on)),
        ACKNOWLEDGE,
    );
    if ui.is_rect_visible(response.rect) {
        paint_mark(ui.painter(), Outline::ring(response.rect), mark);
    }
    response.on_hover_text(label.shown(ui.ctx()))
}

/// The one round button of a bar (the replay's Play and Pause): its icon standing in an ink ring,
/// as `pill_button` stands a word in its pill; unavailable, ring and icon in pencil grey.
pub fn icon_ringed<'a>(
    ui: &mut egui::Ui,
    icon: Icon,
    label: impl Into<Words<'a>>,
    enabled: bool,
) -> egui::Response {
    let label = label.into();
    let response = button(ui, icon, label, enabled, false);
    if ui.is_rect_visible(response.rect) {
        let ring = Outline::ring(response.rect);
        let color = if enabled { INK } else { INK_DISABLED };
        let line = egui::Stroke::new(MARK_LINE, color);
        let kind = egui::StrokeKind::Middle;
        ui.painter().rect_stroke(ring.rect, ring.corner, line, kind);
    }
    response.on_hover_text(label.shown(ui.ctx()))
}

/// An icon of the group `group`, showing whether it is the one `chosen`.
pub fn icon_choice(
    ui: &mut egui::Ui,
    group: &mut Choices,
    (icon, label): (Icon, Words<'_>),
    chosen: bool,
) -> egui::Response {
    let response = button(ui, icon, label, true, chosen);
    if chosen {
        group.chosen(Outline::ring(response.rect));
    }
    response.on_hover_text(label.shown(ui.ctx()))
}

/// The button without its mark; `on` inks the icon deeper.
fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: Words<'_>,
    enabled: bool,
    on: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TOUCH_TARGET), sense);
    let shown = label.shown(ui.ctx());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, shown));
    controls::register(ui, label.id(), rect);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.is_pointer_button_down_on() {
            paint_pressed(painter, Outline::ring(rect));
        }
        let color = match (enabled, on) {
            (false, _) => INK_DISABLED,
            (true, true) => INK_ACTIVE,
            (true, false) => INK,
        };
        let square = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        paint_icon(painter, icon, square, color);
    }
    response
}
