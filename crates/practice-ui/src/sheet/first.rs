//! The first lesson's How (S122): in place of a kind's, whichever of its three exercises is in
//! hand: "First lesson", its exercises and minutes, three short numbered steps (scribble, copy a
//! line, Compare), and Got it.

use openpaint_ui::components::{card_step, card_text, text_button};
use openpaint_ui::design::{INK, SPACE};

use crate::home::FIRST;
use crate::text::{self as t, Words, language};

/// The card's contents; whether Got it was tapped.
pub(crate) fn first_how(ui: &mut egui::Ui) -> bool {
    let language = language(ui.ctx());
    ui.heading(Words::from(t::FIRST_LESSON).shown(ui.ctx()));
    card_text(ui, Words::made(&t::exercises_line(language, FIRST)), true);
    ui.add_space(SPACE);
    let steps = [
        t::FIRST_STEP_SCRIBBLE,
        t::FIRST_STEP_COPY,
        t::FIRST_STEP_COMPARE,
    ];
    for (index, step) in steps.into_iter().enumerate() {
        card_step(ui, index + 1, step, INK);
    }
    ui.add_space(SPACE);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        text_button(ui, t::GOT_IT, true).clicked()
    })
    .inner
}
