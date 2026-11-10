//! What every practice test takes: the sheet a session holds, and its intents, as the runtime
//! reaches them.

use openpaint_app::{Session, Title};
use openpaint_practice::{Given, Pack, Sheet};
use openpaint_practice_app::{PracticeIntent, PracticeSheet, Refused, SheetView};

/// `record`'s sheet as the screen draws it on a canvas of `size`, untitled, its pictures as
/// `packs` hold them and what it prints as its generators print it now.
pub fn view(record: &Sheet, packs: &[Pack], size: [u32; 2]) -> SheetView {
    let printed = Given::of_sheet(record, packs);
    SheetView::of(record, &printed, packs, (size, &Title::untitled(), 1))
}

/// The practice sheet a session holds and its intents, as methods of the session.
pub trait OnSheet {
    /// The sheet, which the test opened.
    fn sheet(&mut self) -> &mut PracticeSheet;

    /// `intent` applied to the sheet at `now` (`openpaint_practice_app::practise`).
    fn practise(&mut self, intent: PracticeIntent, now: u64) -> Result<(), Refused>;
}

impl OnSheet for Session {
    fn sheet(&mut self) -> &mut PracticeSheet {
        PracticeSheet::of_mut(self).expect("a practice sheet")
    }

    fn practise(&mut self, intent: PracticeIntent, now: u64) -> Result<(), Refused> {
        openpaint_practice_app::practise(self, intent, now)
    }
}
