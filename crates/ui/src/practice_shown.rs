//! What practice gives the screen each frame (`Shell::show`) and what the user asks of it there
//! (`Shell::take_practice`). Its intents never travel among the painting's (D112): the runtime
//! applies them after the frame's painting intents.

use openpaint_app::Intent;
use openpaint_practice_app::{
    OptionsIntent, OptionsNotice, PracticeIntent, PracticeProjection, TodayProjection,
};

/// Practice's part of one frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct PracticeShown<'a> {
    /// The open practice sheet, on the canvas.
    pub sheet: Option<&'a PracticeProjection>,
    /// The Today card, its options and the practice book, in the gallery.
    pub today: Option<&'a TodayProjection>,
    /// What the practice options tell the user, in the gallery once it has nothing to.
    pub notice: Option<OptionsNotice>,
}

/// Practice's taps since the last `Shell::take_practice`: the sheet's and the options'.
#[derive(Debug, Default)]
pub(crate) struct PracticeTaps {
    pub(crate) sheet: Vec<PracticeIntent>,
    pub(crate) options: Vec<OptionsIntent>,
}

impl PracticeTaps {
    pub(crate) fn is_empty(&self) -> bool {
        self.sheet.is_empty() && self.options.is_empty()
    }
}

/// A tap on a practice sheet's chrome asks the painting (a tool, a colour, undo) or the sheet.
pub(crate) struct SheetTaps<'a> {
    pub(crate) painting: &'a mut Vec<Intent>,
    pub(crate) sheet: &'a mut Vec<PracticeIntent>,
}
