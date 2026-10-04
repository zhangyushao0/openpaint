//! The icon button: one touch target on the chrome with an ink-line icon. Pressed, a faint wash
//! fills its ring's circle under the hand (never the wider touch target); unavailable, its icon
//! fades to pencil grey and taps go nowhere; the pen hovering over it shows its name. The
//! toggle form carries the active mark (an ink ring, never a filled block) while what it opens
//! is open; the choice form is one of a `choices` group, whose one ring slides to the icon
//! chosen.

use egui::{Rect, Sense, Vec2, WidgetInfo, WidgetType};

use crate::components::{Choices, animate, controls};
use crate::design::motion::ACKNOWLEDGE;
use crate::design::{
    ICON_SIZE, INK, INK_ACTIVE, INK_DISABLED, Icon, Outline, TOUCH_TARGET, paint_icon, paint_mark,
    paint_pressed,
};

/// Shows a button of one touch target with `icon`; `label` names it for accessibility. Only an
/// `enabled` button can be clicked.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let response = button(ui, icon, label, enabled, false);
    response.on_hover_text(label)
}

/// An icon button that shows whether what it opens is `on`.
pub fn icon_toggle(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool) -> egui::Response {
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
    response.on_hover_text(label)
}

/// An icon of the group `group`, showing whether it is the one `chosen`.
pub fn icon_choice(
    ui: &mut egui::Ui,
    group: &mut Choices,
    (icon, label): (Icon, &str),
    chosen: bool,
) -> egui::Response {
    let response = button(ui, icon, label, true, chosen);
    if chosen {
        group.chosen(Outline::ring(response.rect));
    }
    response.on_hover_text(label)
}

/// The button without its mark; `on` inks the icon deeper.
fn button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool, on: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    controls::register(ui, label, rect);
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
