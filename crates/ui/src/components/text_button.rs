//! Words the hand taps (S25, one ring rule): a word in ink, bare on the paper, one touch target
//! high and at least one wide (the frame was never the hit area). Pressed, a faint wash fills
//! the pill the word would carry if chosen, nothing wider; unavailable, the word fades to pencil
//! grey and taps go nowhere. A command stays bare. A choice among words (one of a few options,
//! such as the selection's shape) belongs to a `choices` group, whose one ink pill (the icon's
//! ring drawn round the word) slides to the word chosen. When a bar holds a choice and
//! commands, a `divider` sets them apart; never a frame inside a frame.

use egui::{Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};

use crate::components::{Choices, controls};
use crate::design::{
    INK, INK_ACTIVE, INK_DISABLED, Outline, SPACE, TOUCH_TARGET, paint_divider, paint_pressed,
};

/// Shows a command reading `label`; only an `enabled` one can be clicked.
pub fn text_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    button(ui, label, (enabled, None), TOUCH_TARGET)
}

/// A word reading `label` in the group `group`, showing whether it is the one `chosen`.
pub fn text_choice(
    ui: &mut egui::Ui,
    group: &mut Choices,
    label: &str,
    chosen: bool,
) -> egui::Response {
    button(ui, label, (true, Some((group, chosen))), TOUCH_TARGET)
}

/// As `text_choice`, at least `width` wide (options in even columns); unavailable ones grey.
pub fn text_choice_wide(
    ui: &mut egui::Ui,
    group: &mut Choices,
    label: &str,
    (enabled, chosen): (bool, bool),
    width: f32,
) -> egui::Response {
    button(ui, label, (enabled, Some((group, chosen))), width)
}

/// A thin upright line between a bar's choice and its commands; it takes no width of its own,
/// lying in the middle of the usual gap between the two.
pub fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(0.0, TOUCH_TARGET), Sense::hover());
    paint_divider(
        ui.painter(),
        rect.left(),
        rect.center().y,
        TOUCH_TARGET / 2.0,
    );
}

/// `choice` is `None` for a command, its group and whether it is chosen for a choice.
fn button(
    ui: &mut egui::Ui,
    label: &str,
    (enabled, choice): (bool, Option<(&mut Choices, bool)>),
    min_width: f32,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let font = TextStyle::Button.resolve(ui.style());
    let on = choice.as_ref().is_some_and(|(_, chosen)| *chosen);
    let color = match (enabled, on) {
        (false, _) => INK_DISABLED,
        (true, true) => INK_ACTIVE,
        (true, false) => INK,
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
    let width = (galley.size().x + 4.0 * SPACE).max(min_width);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, label));
    controls::register(ui, label, rect);
    // The pill holds the word, not the whole column of a wide choice.
    let hug = Vec2::new(width.min(galley.size().x + 4.0 * SPACE), TOUCH_TARGET);
    let outline = Outline::pill(Rect::from_center_size(rect.center(), hug));
    if let Some((group, true)) = choice {
        group.chosen(outline);
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.is_pointer_button_down_on() {
            paint_pressed(painter, outline);
        }
        let text = Rect::from_center_size(rect.center(), galley.size());
        painter.galley(text.min, galley, color);
    }
    response
}
