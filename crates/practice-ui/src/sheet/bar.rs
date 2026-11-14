//! A practice sheet's chrome (G9, S122): the practice bar in the middle of the top band (while a
//! Pomodoro's block runs its ring and minutes first, `timer.rs`; the lesson in hand and its place,
//! "Lines 3/5", "First lesson 2/3", "Warm-up 1/3"; How, Compare carrying the ink pill while the
//! comparison shows, Next or Finish, End at the block's time; a scribble Next alone; once the
//! lesson is done "Lines · lesson done", once the block is "Pomodoro done"; on a long colour copy
//! its name, How and Compare, never an end nor a ring), the exercise's own tools beside its frame
//! (`tools.rs`), and under them the card, grown out of the word that asked for it: the How (by
//! itself the first time a kind comes up; the first lesson's own), the comparison with its
//! result, a lesson's end (while a block runs only when the next lesson was rolled back) or a block's end
//! (`card.rs`, `first.rs`, `result.rs`, `end.rs`, `block_end.rs`). The sheet itself is `marks.rs`
//! and `rows.rs`.

use egui::Align;
use openpaint_app::{GalleryIntent, Intent, Viewport};
use openpaint_practice_app::{BlockShown, PracticeIntent, PracticeProjection, TimerIntent};
use openpaint_ui::components::{
    Place, ScreenArea, Source, UNDER_ISLANDS, card, choices, divider, pill, ring_of, text_button,
    text_choice_wide,
};
use openpaint_ui::design::SPACE;
use openpaint_ui::taps::push_once;

use super::block_end::{BlockTap, block_end_card};
use super::card as practice_card;
use super::demo::Demonstrations;
use super::end::{EndTap, end_card};
use super::first::first_how;
use super::timer::{Command, next_or_end, ring_menu, ring_word};
use super::tools::exercise_tools;
use super::{SheetTaps, lesson_name};
use crate::text as t;

#[derive(Debug, Default)]
pub(crate) struct PracticeBar {
    /// Where the bar, the exercise's tools, the card and the ring's menu lie on screen, in
    /// pixels, from the last pass.
    bar: Option<ScreenArea>,
    tools: Option<ScreenArea>,
    card: Option<ScreenArea>,
    menu: Option<ScreenArea>,
    /// The word the card last grew out of, in points.
    source: Option<egui::Rect>,
    /// The ring in the bar (points, the last pass) and whether its menu is open.
    ring: Option<egui::Rect>,
    menu_open: bool,
    /// The How card's demonstrations (S92).
    demos: Demonstrations,
}

/// The card a sheet shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardShown {
    BlockEnd,
    End,
    How,
    Compared,
}

/// The card for a sheet whose lesson has ended (`end`), shows a How (`how`) or compares
/// (`comparing`), a Pomodoro's `block` in hand: a block's end in place of every other, a lesson's
/// end while no block runs, or while one runs once the lesson the chain laid after it was rolled
/// back (`rolled_back`: nothing follows it then but its One more).
pub(crate) fn card_shown(
    (end, how, comparing): (bool, bool, bool),
    (block, rolled_back): (Option<&BlockShown>, bool),
) -> Option<CardShown> {
    match block {
        Some(BlockShown::Ended { .. }) => Some(CardShown::BlockEnd),
        _ if end && (block.is_none() || rolled_back) => Some(CardShown::End),
        _ if how => Some(CardShown::How),
        _ if comparing => Some(CardShown::Compared),
        _ => None,
    }
}

impl PracticeBar {
    /// The bar, the exercise's tools (a colour's wheel the colour panel's, lent with the canvas's
    /// projection and the painting's taps), the ring's menu and the card on the open practice
    /// sheet, their taps pushed to `taps`; Next, One more, End and Another pomodoro move the view
    /// within the canvas the shell lends. The tools and the card lie at the right under the
    /// islands, in the room the view leaves them (`PRACTICE_ROOM`).
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        (practice, block): (&PracticeProjection, Option<&BlockShown>),
        taps: &mut SheetTaps<'_, '_>,
    ) {
        (self.bar, self.tools, self.card, self.menu) = (None, None, None, None);
        // A colour copy is no lesson: no block shows on it.
        let block = block.filter(|_| !practice.copy);
        let projection = taps.lent.session;
        let taken = projection.selection().tool || projection.transform().tool;
        let place = Place::middle_unless(taken);
        let source = Source::Within(Align::Center);
        let ((), bar) = pill(ctx, "practice bar", place, source, |ui| {
            self.words(ui, (practice, block), taps);
        });
        let ppp = ctx.pixels_per_point();
        self.bar = Some(ScreenArea::from_points(bar, ppp));
        let menu = self.menu(ctx, block, taps.timer);
        self.menu = menu.map(|rect| ScreenArea::from_points(rect, ppp));
        let tools = exercise_tools(ctx, practice, taps);
        self.tools = tools.map(|rect| ScreenArea::from_points(rect, ppp));
        let top = tools.map_or(UNDER_ISLANDS, |rect| rect.bottom() + SPACE);
        let opened = (ring_of(self.source.unwrap_or(bar)), top);
        let shown = (
            practice.end.is_some(),
            practice.how.is_some(),
            practice.comparing,
        );
        let card = match card_shown(shown, (block, practice.rolled_back)) {
            Some(CardShown::BlockEnd) => self.block_end(ctx, block, opened, taps),
            Some(CardShown::End) => Self::lesson_end(ctx, practice, opened, taps),
            Some(CardShown::How) => self.how(ctx, practice, opened, taps),
            Some(CardShown::Compared) => Some(
                card(ctx, "practice card", opened, |ui| {
                    practice_card::compared(ui, practice, taps.sheet);
                })
                .1,
            ),
            None => None,
        };
        self.card = card.map(|rect| ScreenArea::from_points(rect, ppp));
    }

    /// A lesson's end on the card.
    fn lesson_end(
        ctx: &egui::Context,
        practice: &PracticeProjection,
        opened: (Source, f32),
        taps: &mut SheetTaps<'_, '_>,
    ) -> Option<egui::Rect> {
        let end = practice.end.as_ref()?;
        let name = lesson_name(practice.lesson, practice.kind);
        let (tapped, rect) = card(ctx, "practice card", opened, |ui| end_card(ui, end, name));
        match (tapped, taps.lent.viewport) {
            (Some(EndTap::OneMore), Some(viewport)) => {
                push_once(taps.sheet, PracticeIntent::OneMore(viewport));
            }
            (Some(EndTap::Enough), _) => {
                push_once(taps.lent.taps, Intent::Gallery(GalleryIntent::Show));
            }
            _ => {}
        }
        Some(rect)
    }

    /// A block's end on the card.
    fn block_end(
        &mut self,
        ctx: &egui::Context,
        block: Option<&BlockShown>,
        opened: (Source, f32),
        taps: &mut SheetTaps<'_, '_>,
    ) -> Option<egui::Rect> {
        let Some(BlockShown::Ended { end, rest }) = block else {
            return None;
        };
        let (tapped, rect) = card(ctx, "practice card", opened, |ui| {
            block_end_card(ui, end, *rest)
        });
        match (tapped, taps.lent.viewport) {
            (Some(BlockTap::Another), Some(viewport)) => {
                push_once(taps.timer, TimerIntent::Another(viewport));
                self.source = None;
            }
            (Some(BlockTap::Enough), _) => {
                push_once(taps.lent.taps, Intent::Gallery(GalleryIntent::Show));
            }
            _ => {}
        }
        Some(rect)
    }

    /// A How on the card: a kind's, or the first lesson's.
    fn how(
        &mut self,
        ctx: &egui::Context,
        practice: &PracticeProjection,
        opened: (Source, f32),
        taps: &mut SheetTaps<'_, '_>,
    ) -> Option<egui::Rect> {
        let kind = practice.how?;
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
        Some(rect)
    }

    /// The ring's menu while it is open and the block counts or is held; where it lies. A press
    /// outside it and the ring closes it, as a tap in it does.
    fn menu(
        &mut self,
        ctx: &egui::Context,
        block: Option<&BlockShown>,
        timer: &mut Vec<TimerIntent>,
    ) -> Option<egui::Rect> {
        let paused = match block {
            Some(BlockShown::Running { paused, .. }) => *paused,
            _ => {
                self.menu_open = false;
                return None;
            }
        };
        let ring = self.ring.filter(|_| self.menu_open)?;
        let (tapped, rect) = ring_menu(ctx, ring, paused);
        let pressed = ctx.input(|input| {
            (input.pointer.any_pressed())
                .then(|| input.pointer.interact_pos())
                .flatten()
        });
        let outside = pressed.is_some_and(|at| !rect.contains(at) && !ring.contains(at));
        if let Some(intent) = tapped {
            push_once(timer, intent);
        }
        if tapped.is_some() || outside {
            self.menu_open = false;
        }
        Some(rect)
    }

    /// The bar's words.
    fn words(
        &mut self,
        ui: &mut egui::Ui,
        (practice, block): (&PracticeProjection, Option<&BlockShown>),
        taps: &mut SheetTaps<'_, '_>,
    ) {
        // A label, then the commands, as far apart as a notice's message and its answer.
        let language = t::language(ui.ctx());
        let name = lesson_name(practice.lesson, practice.kind);
        if matches!(block, Some(BlockShown::Ended { .. })) {
            ui.label(t::POMODORO_DONE.get(language));
            return;
        }
        if let Some((ring, tapped)) = block.and_then(|block| ring_word(ui, block)) {
            self.ring = Some(ring);
            self.menu_open ^= tapped;
            divider(ui);
        }
        let up = matches!(block, Some(BlockShown::Up));
        if practice.current.is_none() {
            ui.label(t::lesson_done(language, name));
            if up {
                self.next(ui, (practice.last, up), taps);
            }
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
            self.how_and_compare(ui, practice, taps.sheet);
        }
        if study {
            // A colour copy is worked on whenever the user likes: no Next, no end.
            return;
        }
        self.next(ui, (practice.last, up), taps);
    }

    /// Next or Finish, or at the time End; the view moves within the canvas lent.
    fn next(&mut self, ui: &mut egui::Ui, (last, up): (bool, bool), taps: &mut SheetTaps<'_, '_>) {
        let viewport: Option<Viewport> = taps.lent.viewport;
        let tapped = next_or_end(ui, (last, up), viewport.is_some());
        let (Some((command, rect)), Some(viewport)) = (tapped, viewport) else {
            return;
        };
        match command {
            Command::Next => push_once(taps.sheet, PracticeIntent::Next(viewport)),
            Command::End => push_once(taps.timer, TimerIntent::End(viewport)),
        }
        self.source = Some(rect);
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

    /// Where the bar, the exercise's tools, the card and the ring's menu lie, which the canvas
    /// must not take contacts from.
    pub(crate) fn areas(&self) -> [Option<ScreenArea>; 4] {
        [self.bar, self.tools, self.card, self.menu]
    }
}
