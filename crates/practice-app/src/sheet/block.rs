//! The open sheet in a Pomodoro's block (S124, `crate::pomodoro`): End cuts the lesson in hand to
//! the exercises begun (the one in hand and those before it; those after it go from the record,
//! from the sheet as the screen draws it, `SheetView`, and from the stored givens), finishes it at
//! the tick and fits the view to the rows drawn in the block (its lessons finished at or after the
//! block's start, as `BlockTally` counts them), in the room left of the card. The record cut goes
//! to the store with its givens in one write (`trim_request`, `practice_store::trim`), before any
//! other save of it.

use openpaint_practice::Sheet;

use super::PracticeSheet;

impl PracticeSheet {
    /// The record End cut, once, for the store to write with its last lesson's givens; none
    /// otherwise (the runtime asks it before `save_request`).
    pub fn trim_request(&mut self) -> Option<Sheet> {
        // S124b
        None
    }
}
