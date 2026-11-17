//! The first lesson's How (S122): in place of a kind's, for the exercise in hand alone (S160):
//! "First lesson", its exercises and minutes, then at its scribble the one step it asks (draw
//! something in front of you, S151), at its lines two numbered steps (copy the line; Compare),
//! and Got it.

use openpaint_practice::Kind;
use openpaint_ui::components::{card_step, card_text, text_button};
use openpaint_ui::design::{INK, SPACE};

use crate::home::FIRST;
use crate::text::{self as t, Words, language};

/// The card's contents for an exercise of `kind`; whether Got it was tapped.
pub(crate) fn first_how(ui: &mut egui::Ui, kind: Kind) -> bool {
    let language = language(ui.ctx());
    ui.heading(Words::from(t::FIRST_LESSON).shown(ui.ctx()));
    card_text(ui, Words::made(&t::exercises_line(language, FIRST)), true);
    ui.add_space(SPACE);
    if kind == Kind::Free {
        card_text(ui, t::FIRST_STEP_IN_FRONT, false);
    } else {
        let steps = [t::FIRST_STEP_COPY, t::FIRST_STEP_COMPARE];
        for (index, step) in steps.into_iter().enumerate() {
            card_step(ui, index + 1, step, INK);
        }
    }
    ui.add_space(SPACE);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        text_button(ui, t::GOT_IT, true).clicked()
    })
    .inner
}
