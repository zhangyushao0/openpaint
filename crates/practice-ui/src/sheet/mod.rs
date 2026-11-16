//! A practice sheet on the canvas (G9, D112), the ui's canvas hook (`openpaint_ui::CanvasHook`)
//! filled: the pairs, givens and comparison's marks over the canvas (`marks.rs`, `mark_draw.rs`,
//! `mark_lines.rs`), the lessons' labels and a lesson's results on its rows (`rows.rs`), All tools
//! after Gallery, the practice bar holding the top band's middle, the exercise's own tools beside
//! its frame (`tools.rs`) and the card under them (`bar.rs`, `card.rs`, `card_items.rs`,
//! `card_disc.rs`, a result's row `result.rs`, a lesson's end `end.rs`, the first lesson's How
//! `first.rs`), the Pomodoro's ring in the bar and its menu (`timer.rs`) and a block's end
//! (`block_end.rs`, S124), How's demonstrations (`demo*.rs`), and why the sheet took no contact.
//! The sheet's taps are its own (`PracticeIntent`), the timer's its own (`TimerIntent`); what it
//! asks of the painting goes to the shell's taps it is lent.

mod bar;
mod block_end;
mod card;
mod card_disc;
mod card_items;
mod demo;
mod demo_marks;
mod demo_strip;
mod demo_wheel;
mod end;
mod end_pair;
mod first;
mod mark_draw;
mod mark_lines;
mod marks;
mod result;
mod rows;
#[cfg(test)]
mod tests;
mod timer;
#[cfg(test)]
mod timer_tests;
mod tools;

use egui::{Context, Shape, Ui};
use openpaint_app::{Intent, Language, ViewMapping};
use openpaint_practice::{Kind, Skill};
use openpaint_practice_app::{
    LessonShown, PracticeIntent, PracticeNotice, PracticeProjection, TimerIntent, TimerShown,
};
use openpaint_ui::components::ScreenArea;
use openpaint_ui::taps::push_once;
use openpaint_ui::{CanvasHook, Chrome};

use crate::text::{self as t, Refused, Text};

/// What the bar and the cards call the lesson in hand: the first lesson, a re-test, its skill,
/// else the kind's own name.
pub(crate) fn lesson_name(lesson: Option<LessonShown>, kind: Option<Kind>) -> Text {
    let skill = lesson.and_then(|lesson| lesson.row.skill);
    match (lesson, skill, kind) {
        (Some(lesson), _, _) if lesson.row.first => t::FIRST_LESSON,
        (Some(lesson), _, _) if lesson.row.retest.is_some() => t::RETEST,
        (_, Some(skill), _) => t::skill_name(skill),
        (_, None, Some(kind)) => Skill::of(kind).map_or(t::kind_words(kind).name, t::skill_name),
        _ => t::STUDY_EXERCISE,
    }
}

/// A practice sheet's screen state between frames: its pictures as textures, its bar and card,
/// and what it laid over the canvas in the last pass, which goes back into the home with the
/// sheet's picture when the canvas is left (S144).
#[derive(Debug, Default)]
pub(crate) struct SheetPart {
    pictures: marks::SheetPictures,
    bar: bar::PracticeBar,
    laid: Vec<Shape>,
}

impl SheetPart {
    /// What the sheet laid over the canvas in the last pass that showed it, as it lay there.
    pub(crate) fn laid(&self) -> &[Shape] {
        &self.laid
    }
}

/// What the marks and the rows' labels drew in this pass, as they lie on the screen.
fn laid(ctx: &Context) -> Vec<Shape> {
    ctx.graphics(|layers| {
        [marks::layer(), rows::layer()]
            .into_iter()
            .filter_map(|layer| layers.get(layer))
            .flat_map(|list| list.all_entries().map(|clipped| clipped.shape.clone()))
            .collect()
    })
}

/// A tap on a practice sheet's chrome asks the painting (a tool, a colour, undo, through what
/// the shell lends), the sheet or the timer.
pub(crate) struct SheetTaps<'a, 'c> {
    pub(crate) lent: &'a mut Chrome<'c>,
    pub(crate) sheet: &'a mut Vec<PracticeIntent>,
    pub(crate) timer: &'a mut Vec<TimerIntent>,
}

/// The open practice sheet's hook for one frame: its state, its projection, the Pomodoro's timer
/// and their taps.
pub(crate) struct SheetHook<'a> {
    pub(crate) part: &'a mut SheetPart,
    pub(crate) sheet: &'a PracticeProjection,
    pub(crate) taps: &'a mut Vec<PracticeIntent>,
    pub(crate) timer: &'a TimerShown,
    pub(crate) timer_taps: &'a mut Vec<TimerIntent>,
}

impl CanvasHook for SheetHook<'_> {
    fn marks(&mut self, ctx: &Context, at: &ViewMapping, pixels_per_point: f32) {
        let pictures = &mut self.part.pictures;
        marks::show_marks(ctx, self.sheet, (at, pixels_per_point), pictures);
        rows::show_rows(ctx, self.sheet, (at, pixels_per_point));
        self.part.laid = laid(ctx);
    }

    fn left(&mut self, ui: &mut Ui, taps: &mut Vec<Intent>) {
        tools::all_tools(ui, self.sheet, (taps, self.taps));
    }

    fn chrome(&mut self, ctx: &Context, lent: &mut Chrome<'_>) -> Vec<ScreenArea> {
        let mut taps = SheetTaps {
            lent,
            sheet: self.taps,
            timer: self.timer_taps,
        };
        let shown = (self.sheet, self.timer.block.as_ref());
        self.part.bar.show(ctx, shown, &mut taps);
        self.part.bar.areas().into_iter().flatten().collect()
    }

    fn own_tools(&self) -> bool {
        !self.sheet.all_tools
    }

    fn notice(&self, language: Language) -> Option<String> {
        let message = match self.sheet.notice? {
            PracticeNotice::DrawInFrame => t::DRAW_IN_FRAME.get(language).to_owned(),
            PracticeNotice::DrawInCell => t::DRAW_IN_CELL.get(language).to_owned(),
            PracticeNotice::PickAfterCompare => t::PICK_AFTER_COMPARE.get(language).to_owned(),
            PracticeNotice::CannotFill(refusal) => t::refused(language, Refused::Fill, refusal),
            PracticeNotice::Done => t::SESSION_DONE.get(language).to_owned(),
            // The words a failed save shows (the painting's).
            PracticeNotice::NotSaved => t::SAVE_FAILED.get(language).to_owned(),
        };
        Some(message)
    }

    fn dismiss(&mut self) {
        push_once(self.taps, PracticeIntent::Dismiss);
    }

    fn outside(&self) -> Option<Text> {
        // The empty frame is what a fill stays inside.
        Some(t::DROP_IN_FRAME)
    }

    fn tapped(&self) -> bool {
        !self.taps.is_empty() || !self.timer_taps.is_empty()
    }

    fn glides(&self) -> bool {
        // A tap on Next: the view glides to the next exercise, or to the rows used once done; on
        // One more or Another pomodoro to the next lesson's first pair, laid on the same sheet;
        // on End to the rows drawn in the block.
        let timer = (self.timer_taps.iter())
            .any(|tap| matches!(tap, TimerIntent::End(_) | TimerIntent::Another(_)));
        timer
            || (self.taps.iter())
                .any(|tap| matches!(tap, PracticeIntent::Next(_) | PracticeIntent::OneMore(_)))
    }
}
