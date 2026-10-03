//! The icon button: one touch target on the chrome with an ink-line icon. Pressed, a faint wash
//! shows under the hand; unavailable, its icon fades to pencil grey and taps go nowhere; the pen
//! hovering over it shows its name. The toggle form carries the active mark (an ink ring, never
//! a filled block) while what it opens is open.

use egui::{Rect, Sense, Vec2, WidgetInfo, WidgetType};

use crate::components::animate;
use crate::design::motion::ACKNOWLEDGE;
use crate::design::{
    ICON_SIZE, INK, INK_ACTIVE, INK_DISABLED, Icon, SPACE, TOUCH_TARGET, paint_icon,
    paint_icon_mark, paint_pressed,
};

/// Shows a button of one touch target with `icon`; `label` names it for accessibility. Only an
/// `enabled` button can be clicked.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    button(ui, icon, label, enabled, false)
}

/// An icon button that shows whether what it opens is `on`.
pub fn icon_toggle(ui: &mut egui::Ui, icon: Icon, label: &str, on: bool) -> egui::Response {
    button(ui, icon, label, true, on)
}

fn button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool, on: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    let mark = animate(
        ui.ctx(),
        response.id.with("mark"),
        f32::from(u8::from(on)),
        ACKNOWLEDGE,
    );
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.is_pointer_button_down_on() {
            paint_pressed(painter, rect.shrink(SPACE / 2.0));
        }
        paint_icon_mark(painter, rect, mark);
        let color = match (enabled, on) {
            (false, _) => INK_DISABLED,
            (true, true) => INK_ACTIVE,
            (true, false) => INK,
        };
        let square = Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        paint_icon(painter, icon, square, color);
    }
    // The pen hovering over it reads its name (fingers do not hover).
    response.on_hover_text(label)
}
