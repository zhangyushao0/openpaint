//! A practice sheet's chrome (G9): the practice bar in the middle of the top band ("Line 3 of
//! 10", How, Compare carrying the ink pill while the check shows, Next or Finish; once done
//! "Session 3 done" and One more), and the card under it, grown out of the word that asked for
//! it: the instructions (by themselves the first time a kind comes up), a check's numbers, or the
//! session's summary (`practice_card.rs`). The sheet itself is `practice_marks.rs`.

use egui::Align;
use openpaint_app::{Intent, PracticeIntent, PracticeProjection, SessionProjection, Viewport};

use crate::components::{
    Place, ScreenArea, Source, card, choices, divider, pill, ring_of, text_button, text_choice_wide,
};
use crate::design::SPACE;
use crate::practice_card;
use crate::taps::push_once;

#[derive(Debug, Default)]
pub(crate) struct PracticeBar {
    /// Where the bar and the card lie on screen, in pixels, from the last pass.
    bar: Option<ScreenArea>,
    card: Option<ScreenArea>,
    /// The word the card last grew out of, in points.
    source: Option<egui::Rect>,
}

impl PracticeBar {
    /// The bar and the card on a practice sheet, its taps pushed to `taps`; Next moves the view
    /// within `viewport`. The card lies at the right under the islands, in the room the view
    /// leaves it (`PRACTICE_ROOM`).
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        viewport: Option<Viewport>,
        taps: &mut Vec<Intent>,
    ) {
        (self.bar, self.card) = (None, None);
        let Some(practice) = projection.practice() else {
            return;
        };
        let taken = projection.selection().tool || projection.transform().tool;
        let place = Place::middle_unless(taken);
        let source = Source::Within(Align::Center);
        let ((), bar) = pill(ctx, "practice bar", place, source, |ui| {
            self.words(ui, practice, viewport, taps);
        });
        let ppp = ctx.pixels_per_point();
        self.bar = Some(ScreenArea::from_points(bar, ppp));
        let opened = ring_of(self.source.unwrap_or(bar));
        let card = if let Some(kinds) = &practice.summary {
            card(ctx, "practice card", opened, |ui| {
                practice_card::summary(ui, practice, kinds);
            })
            .1
        } else if let Some(lines) = practice.instructions {
            let (done, rect) = card(ctx, "practice card", opened, |ui| {
                practice_card::instructions(ui, lines)
            });
            if done {
                push_once(taps, Intent::Practice(PracticeIntent::Instructions(false)));
            }
            rect
        } else if practice.comparing && practice.result.is_some() {
            card(ctx, "practice card", opened, |ui| {
                practice_card::results(ui, practice);
            })
            .1
        } else {
            return;
        };
        self.card = Some(ScreenArea::from_points(card, ppp));
    }

    /// The bar's words.
    fn words(
        &mut self,
        ui: &mut egui::Ui,
        practice: &PracticeProjection,
        viewport: Option<Viewport>,
        taps: &mut Vec<Intent>,
    ) {
        // A label, then the commands, as far apart as a notice's message and its answer.
        if practice.current.is_none() {
            ui.label(format!("Session {} done", practice.number));
            ui.add_space(2.0 * SPACE);
            divider(ui);
            if text_button(ui, "One more", true).clicked() {
                push_once(taps, Intent::Practice(PracticeIntent::OneMore));
            }
            return;
        }
        let kind = if practice.line { "Line" } else { "Colour" };
        let (place, of) = practice.place;
        ui.label(format!("{kind} {place} of {of}"));
        ui.add_space(2.0 * SPACE);
        divider(ui);
        let how = text_button(ui, "How", true);
        if how.clicked() {
            let show = practice.instructions.is_none();
            push_once(taps, Intent::Practice(PracticeIntent::Instructions(show)));
            self.source = Some(how.rect);
        }
        let offered = practice.compare.is_offered();
        choices(ui, "compare", |ui, group| {
            let on = practice.comparing;
            let compare = text_choice_wide(ui, group, "Compare", (offered, on), 0.0);
            if compare.clicked() {
                push_once(taps, Intent::Practice(PracticeIntent::Compare(!on)));
                self.source = Some(compare.rect);
            }
        });
        let next = if practice.last { "Finish" } else { "Next" };
        let tapped = text_button(ui, next, viewport.is_some());
        if let (true, Some(viewport)) = (tapped.clicked(), viewport) {
            push_once(taps, Intent::Practice(PracticeIntent::Next(viewport)));
            self.source = Some(tapped.rect);
        }
    }

    /// Where the bar and the card lie, which the canvas must not take contacts from.
    pub(crate) fn areas(&self) -> [Option<ScreenArea>; 2] {
        [self.bar, self.card]
    }
}
