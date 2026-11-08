//! What every practice test takes: the sheet a session holds, and its intents, as the runtime
//! reaches them.

use openpaint_app::Session;
use openpaint_practice_app::{PracticeIntent, PracticeSheet, Refused};

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
