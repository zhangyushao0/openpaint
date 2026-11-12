//! A practice sheet's chrome (G9, S122): the practice bar in the middle of the top band (the
//! lesson in hand and its place, "Lines 3/5", "First lesson 2/3", "Warm-up 1/3"; How, Compare
//! carrying the ink pill while the comparison shows, Next or Finish; a scribble Next alone; once
//! the lesson is done "Lines · lesson done"; on a long colour copy its name, How and Compare,
//! never an end), the exercise's own tools beside its frame (`tools.rs`), and under them the card,
//! grown out of the word that asked for it: the How (by itself the first time a kind comes up;
//! the first lesson's own), the comparison with its result, or the lesson's end (`card.rs`,
//! `first.rs`, `result.rs`, `end.rs`). The sheet itself is `marks.rs` and `rows.rs`.

use egui::Align;
use openpaint_app::{GalleryIntent, Intent, Viewport};
use openpaint_practice_app::{PracticeIntent, PracticeProjection};
use openpaint_ui::components::{
    Place, ScreenArea, Source, UNDER_ISLANDS, card, choices, divider, pill, ring_of, text_button,
    text_choice_wide,
};
use openpaint_ui::design::SPACE;
use openpaint_ui::taps::push_once;

use super::card as practice_card;
use super::demo::Demonstrations;
use super::end::{EndTap, end_card};
use super::first::first_how;
use super::tools::exercise_tools;
use super::{SheetTaps, lesson_name};
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
    /// pushed to `taps`; Next and One more move the view within the canvas the shell lends. The
    /// tools and the card lie at the right under the islands, in the room the view leaves them
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
        let card = if let Some(end) = &practice.end {
            let name = lesson_name(practice.lesson, practice.kind);
            let (tapped, rect) = card(ctx, "practice card", opened, |ui| end_card(ui, end, name));
            match (tapped, viewport) {
                (Some(EndTap::OneMore), Some(viewport)) => {
                    push_once(taps.sheet, PracticeIntent::OneMore(viewport));
                }
                (Some(EndTap::Enough), _) => {
                    push_once(taps.lent.taps, Intent::Gallery(GalleryIntent::Show));
                }
                _ => {}
            }
            rect
        } else if let Some(kind) = practice.how {
            let first = practice.lesson.is_some_and(|lesson| lesson.row.first);
            let (demos, of) = (&mut self.demos, practice_card::how_of(practice, kind));
            let (done, rect) = card(ctx, "practice card", opened, |ui| {
                if first {
                    first_how(ui)
                } else {
                    practice_card::instructions(ui, of, demos)
                }
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
        let name = lesson_name(practice.lesson, practice.kind);
        if practice.current.is_none() {
            ui.label(t::lesson_done(language, name));
            return;
        }
        let study = practice.copy;
        if study {
            // Its name: its sheet's title, "Colour copy n" until renamed.
            ui.label(t::shown_name(ui.ctx(), practice.sheet.name.as_str()));
        } else if let Some(lesson) = practice.lesson {
            ui.label(t::in_hand(language, name, lesson.place));
        }
        ui.add_space(2.0 * SPACE);
        divider(ui);
        // A scribble is never compared: Next alone.
        let compared = practice.kind.is_none_or(|kind| kind.facts().compared);
        if compared {
            self.how_and_compare(ui, practice, taps);
        }
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

    /// How, and Compare carrying the ink pill while the comparison shows.
    fn how_and_compare(
        &mut self,
        ui: &mut egui::Ui,
        practice: &PracticeProjection,
        taps: &mut Vec<PracticeIntent>,
    ) {
        let how = text_button(ui, t::HOW, true);
        if how.clicked() {
            let show = practice.how.is_none();
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
    }

    /// Where the bar, the exercise's tools and the card lie, which the canvas must not take contacts
    /// from.
    pub(crate) fn areas(&self) -> [Option<ScreenArea>; 3] {
        [self.bar, self.tools, self.card]
    }
}
