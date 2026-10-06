//! The Actions panel's Pen buttons page (S41): Actions > Pen buttons opens it in the panel's
//! place. Per side button, its name and the six things it can do as words in two columns, the
//! one it does in the ink pill (one group per button: the pill slides); the eraser end is
//! named too, as it always erases. Android cannot tell which buttons a pen has until one is
//! pressed, so the page lists both side buttons and says so. Back returns to Actions.

use egui::{Align, Layout};
use openpaint_app::{ButtonAction, ButtonMap, Intent, PenButton, ToolIntent};

use crate::components::{choices, hint, text_button, text_choice_wide};
use crate::design::{INK, SPACE, TOUCH_TARGET};

/// The Pen buttons row's note on the Actions page: what the two side buttons do.
pub(crate) fn summary(buttons: ButtonMap) -> String {
    format!(
        "{} · {}",
        buttons.primary.label(),
        buttons.secondary.label()
    )
}

/// The page; its choices pushed to `taps`. Returns whether Back was tapped.
pub(crate) fn pen_buttons_page(
    ui: &mut egui::Ui,
    buttons: ButtonMap,
    taps: &mut Vec<Intent>,
) -> bool {
    let back = ui
        .horizontal(|ui| {
            ui.set_height(TOUCH_TARGET);
            ui.add_space(2.0 * SPACE);
            ui.heading("Pen buttons");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(SPACE);
                text_button(ui, "Back", true).clicked()
            })
            .inner
        })
        .inner;
    hint(ui, "For the buttons your pen has.");
    for button in PenButton::ALL {
        label(ui, button.label());
        actions(ui, button, buttons.action(button), taps);
    }
    label(ui, "Eraser end");
    hint(ui, "Erases with the eraser's brush.");
    hint(ui, "Eraser, Eyedropper: while held.");
    back
}

/// A button's name, in ink.
fn label(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.colored_label(INK, text);
    });
}

/// The six actions for `button` in two even columns, `chosen` in the ink pill.
fn actions(ui: &mut egui::Ui, button: PenButton, chosen: ButtonAction, taps: &mut Vec<Intent>) {
    let half = ((ui.available_width() - 2.0 * SPACE) / 2.0).floor();
    choices(ui, button.label(), |ui, group| {
        for pair in ButtonAction::ALL.chunks(2) {
            ui.horizontal(|ui| {
                ui.add_space(2.0 * SPACE);
                for action in pair {
                    let on = *action == chosen;
                    let word = text_choice_wide(ui, group, action.label(), (true, on), half);
                    if word.clicked() && !on {
                        let intent = Intent::Tool(ToolIntent::SetButton {
                            button,
                            action: *action,
                        });
                        if taps.last() != Some(&intent) {
                            taps.push(intent);
                        }
                    }
                }
            });
        }
    });
}
