//! A practice sheet's chrome (G9): the practice bar in the middle of the top band ("Warm-up 1 of
//! 4", "Line 3 of 6", How, Compare carrying the ink pill while the comparison shows, Next or
//! Finish; once done "Session 3 done" and One more; on a long colour copy "Colour copy 2", How
//! and Compare, never an end), the exercise's own tools beside its frame (`tools.rs`), and
//! under them the card, grown out of the word that asked for it: the instructions (by
//! themselves the first time a kind comes up), the comparison, or the session's summary
//! (`card.rs`). The sheet itself is `marks.rs`.

use egui::Align;
use openpaint_app::Viewport;
use openpaint_practice_app::{PracticeIntent, PracticeProjection};
use openpaint_ui::components::{
    Place, ScreenArea, Source, UNDER_ISLANDS, card, choices, divider, pill, ring_of, text_button,
    text_choice_wide,
};
use openpaint_ui::design::SPACE;
use openpaint_ui::taps::push_once;

use super::SheetTaps;
use super::card as practice_card;
use super::demo::Demonstrations;
use super::tools::exercise_tools;
use crate::text as t;

#[derive(Debug, Default)]
pub(crate) struct PracticeBar {
    /// Where the bar, the exercise's tools and the card lie on screen, in pixels, from the last pass.
    bar: Option<ScreenArea>,
    tools: Option<ScreenArea>,
    card: Option<ScreenArea>,
    /// The word the card last grew out of, in points.
    source: Option<egui::Rect>,
    /// The How card's demonstrations (S92).
    demos: Demonstrations,
}

impl PracticeBar {
    /// The bar, the exercise's tools (a colour's wheel the colour panel's, lent with the canvas's
    /// projection and the painting's taps) and the card on the open practice sheet, their taps
    /// pushed to `taps`; Next moves the view within the canvas the shell lends. The tools and the
    /// card lie at the right under the islands, in the room the view leaves them
    /// (`PRACTICE_ROOM`).
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        practice: &PracticeProjection,
        taps: &mut SheetTaps<'_, '_>,
    ) {
        (self.bar, self.tools, self.card) = (None, None, None);
        let projection = taps.lent.session;
        let viewport = taps.lent.viewport;
        let taken = projection.selection().tool || projection.transform().tool;
        let place = Place::middle_unless(taken);
        let source = Source::Within(Align::Center);
        let ((), bar) = pill(ctx, "practice bar", place, source, |ui| {
            self.words(ui, practice, viewport, taps.sheet);
        });
        let ppp = ctx.pixels_per_point();
        self.bar = Some(ScreenArea::from_points(bar, ppp));
        let tools = exercise_tools(ctx, practice, taps);
        self.tools = tools.map(|rect| ScreenArea::from_points(rect, ppp));
        let top = tools.map_or(UNDER_ISLANDS, |rect| rect.bottom() + SPACE);
        let opened = (ring_of(self.source.unwrap_or(bar)), top);
        let card = if let Some(kinds) = &practice.summary {
            card(ctx, "practice card", opened, |ui| {
                practice_card::summary(ui, practice, kinds);
            })
            .1
        } else if let Some(family) = practice.instructions {
            let (demos, of) = (&mut self.demos, practice_card::how_of(practice, family));
            let (done, rect) = card(ctx, "practice card", opened, |ui| {
                practice_card::instructions(ui, of, demos)
            });
            if done {
                push_once(taps.sheet, PracticeIntent::Instructions(false));
            }
            rect
        } else if practice.comparing {
            card(ctx, "practice card", opened, |ui| {
                practice_card::compared(ui, practice, taps.sheet);
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
        taps: &mut Vec<PracticeIntent>,
    ) {
        // A label, then the commands, as far apart as a notice's message and its answer.
        let language = t::language(ui.ctx());
        if practice.current.is_none() {
            ui.label(practice_card::done_heading(language, practice));
            ui.add_space(2.0 * SPACE);
            divider(ui);
            if text_button(ui, t::ONE_MORE, true).clicked() {
                push_once(taps, PracticeIntent::OneMore);
            }
            return;
        }
        let study = practice.copy;
        if study {
            // Its name: its sheet's title, "Colour copy n" until renamed.
            ui.label(t::shown_name(ui.ctx(), practice.sheet.name.as_str()));
        } else if let Some(family) = practice.family {
            let called = (family, practice.warming_up);
            ui.label(t::exercise_of(language, called, practice.place));
        }
        ui.add_space(2.0 * SPACE);
        divider(ui);
        let how = text_button(ui, t::HOW, true);
        if how.clicked() {
            let show = practice.instructions.is_none();
            push_once(taps, PracticeIntent::Instructions(show));
            self.source = Some(how.rect);
        }
        let offered = practice.compare.is_offered();
        choices(ui, "compare", |ui, group| {
            let on = practice.comparing;
            let compare = text_choice_wide(ui, group, t::COMPARE, (offered, on), 0.0);
            if compare.clicked() {
                push_once(taps, PracticeIntent::Compare(!on));
                self.source = Some(compare.rect);
            }
        });
        if study {
            // A colour copy is worked on whenever the user likes: no Next, no end.
            return;
        }
        let next = if practice.last { t::FINISH } else { t::NEXT };
        let tapped = text_button(ui, next, viewport.is_some());
        if let (true, Some(viewport)) = (tapped.clicked(), viewport) {
            push_once(taps, PracticeIntent::Next(viewport));
            self.source = Some(tapped.rect);
        }
    }

    /// Where the bar, the exercise's tools and the card lie, which the canvas must not take contacts
    /// from.
    pub(crate) fn areas(&self) -> [Option<ScreenArea>; 3] {
        [self.bar, self.tools, self.card]
    }
}
