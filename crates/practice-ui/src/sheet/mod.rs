//! A practice sheet on the canvas (G9, D112), the ui's canvas hook (`openpaint_ui::CanvasHook`)
//! filled: the pairs, givens and comparison's marks over the canvas (`marks.rs`, `mark_draw.rs`,
//! `mark_lines.rs`), All tools after Gallery, the practice bar holding the top band's middle,
//! the exercise's own tools beside its frame (`tools.rs`) and the card under them (`bar.rs`,
//! `card.rs`, `card_items.rs`, `card_disc.rs`), How's demonstrations (`demo*.rs`), and why the
//! sheet took no contact. The sheet's taps are its own (`PracticeIntent`); what it asks of the
//! painting goes to the shell's taps it is lent.

mod bar;
mod card;
mod card_disc;
mod card_items;
mod demo;
mod demo_marks;
mod demo_strip;
mod demo_wheel;
mod mark_draw;
mod mark_lines;
mod marks;
mod tools;

use egui::{Context, Ui};
use openpaint_app::{Intent, Language, ViewMapping};
use openpaint_practice_app::{PracticeIntent, PracticeNotice, PracticeProjection};
use openpaint_ui::components::ScreenArea;
use openpaint_ui::taps::push_once;
use openpaint_ui::{CanvasHook, Chrome};

use crate::text::{self as t, Refused, Text};

/// A practice sheet's screen state between frames: its pictures as textures, its bar and card.
#[derive(Debug, Default)]
pub(crate) struct SheetPart {
    pictures: marks::SheetPictures,
    bar: bar::PracticeBar,
}

/// A tap on a practice sheet's chrome asks the painting (a tool, a colour, undo, through what
/// the shell lends) or the sheet.
pub(crate) struct SheetTaps<'a, 'c> {
    pub(crate) lent: &'a mut Chrome<'c>,
    pub(crate) sheet: &'a mut Vec<PracticeIntent>,
}

/// The open practice sheet's hook for one frame: its state, its projection and its taps.
pub(crate) struct SheetHook<'a> {
    pub(crate) part: &'a mut SheetPart,
    pub(crate) sheet: &'a PracticeProjection,
    pub(crate) taps: &'a mut Vec<PracticeIntent>,
}

impl CanvasHook for SheetHook<'_> {
    fn marks(&mut self, ctx: &Context, at: &ViewMapping, pixels_per_point: f32) {
        let pictures = &mut self.part.pictures;
        marks::show_marks(ctx, self.sheet, (at, pixels_per_point), pictures);
    }

    fn left(&mut self, ui: &mut Ui, taps: &mut Vec<Intent>) {
        tools::all_tools(ui, self.sheet, (taps, self.taps));
    }

    fn chrome(&mut self, ctx: &Context, lent: &mut Chrome<'_>) -> Vec<ScreenArea> {
        let mut taps = SheetTaps {
            lent,
            sheet: self.taps,
        };
        self.part.bar.show(ctx, self.sheet, &mut taps);
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
            PracticeNotice::OriginalLocked => t::ORIGINAL_LOCKED.get(language).to_owned(),
            PracticeNotice::CannotFill(refusal) => t::refused(language, Refused::Fill, refusal),
            PracticeNotice::Done => t::SESSION_DONE.get(language).to_owned(),
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
        !self.taps.is_empty()
    }

    fn glides(&self) -> bool {
        // A tap on Next: the view glides to the next exercise, or to the whole sheet once done.
        (self.taps.iter()).any(|tap| matches!(tap, PracticeIntent::Next(_)))
    }
}
