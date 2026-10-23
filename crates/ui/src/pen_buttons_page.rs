//! The Actions panel's Pen buttons page: Actions > Pen buttons opens it in the
//! panel's place. Per side button, its name and the six things it can do as words in two
//! columns, the one it does in the ink pill (one group per button: the pill slides); the eraser
//! end is named too, as it always erases. Android cannot tell which buttons a pen has until one
//! is pressed, so the page lists all three side buttons (from the tip outwards) and says so.
//! Back returns to Actions.

use openpaint_app::{ButtonAction, ButtonMap, Intent, Language, PenButton, ToolIntent};

use crate::components::{Lead, hint, label_row, panel_heading_with, text_button, word_columns};
use crate::design::SPACE;
use crate::taps::push_once;
use crate::text::{self as t, Words};

/// The Pen buttons row's note on the Actions page: what the side buttons do, from the tip
/// outwards ("Eraser · Eyedropper · Undo" at first), in `language`.
pub(crate) fn summary(language: Language, buttons: ButtonMap) -> String {
    PenButton::ALL
        .map(|button| t::button_action(buttons.action(button)).get(language))
        .join(" · ")
}

/// The page; its choices pushed to `taps`. Returns whether Back was tapped.
pub(crate) fn pen_buttons_page(
    ui: &mut egui::Ui,
    buttons: ButtonMap,
    taps: &mut Vec<Intent>,
) -> bool {
    let back = panel_heading_with(ui, t::PEN_BUTTONS, |ui| {
        ui.add_space(SPACE);
        text_button(ui, t::BACK, true).clicked()
    });
    hint(ui, t::FOR_YOUR_PEN);
    for button in PenButton::ALL {
        label_row(ui, t::pen_button(button), true);
        actions(ui, button, buttons.action(button), taps);
    }
    label_row(ui, t::ERASER_END, true);
    hint(ui, t::ERASER_END_ERASES);
    hint(ui, t::WHILE_HELD);
    back
}

/// The six actions for `button` in two even columns, `chosen` in the ink pill.
fn actions(ui: &mut egui::Ui, button: PenButton, chosen: ButtonAction, taps: &mut Vec<Intent>) {
    let words = ButtonAction::ALL.map(|action| {
        (
            Words::from(t::button_action(action)),
            action == chosen,
            true,
        )
    });
    if let Some(at) = word_columns(ui, t::pen_button(button).id(), &words, Lead::Indent) {
        let action = ButtonAction::ALL[at];
        push_once(taps, Intent::Tool(ToolIntent::SetButton { button, action }));
    }
}
