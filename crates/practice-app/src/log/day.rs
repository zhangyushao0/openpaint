//! The day's sheet (S121c): the day's lessons follow down one sheet. A sheet is today's when it
//! was started on today's local day; a session or drill started from the gallery or the options
//! goes onto today's sheet (the newest day's sheet started today) when its last lesson is
//! finished and the rows left on its canvas hold the new lesson (the store checks the canvas),
//! else onto a new sheet. A test's sheet is never continued from the gallery, a colour copy keeps
//! a sheet of its own, and sheets from before the day's sheet hold only the rows they use, so no
//! lesson fits on them.

use openpaint_practice::{Purpose, Sheet, local_day};

use super::PracticeLog;

impl PracticeLog {
    /// Whether `sheet` was started on today's local day.
    #[must_use]
    pub fn is_today(&self, sheet: &Sheet) -> bool {
        local_day(sheet.started, self.offset) == self.today
    }

    /// Today's sheet a lesson started from the gallery may go onto: the newest day's sheet
    /// started today, if its last lesson is finished.
    #[must_use]
    pub fn day_sheet(&self) -> Option<&Sheet> {
        let today = (self.sheets.iter().rev())
            .find(|s| s.purpose == Purpose::Day && s.canvas != 0 && self.is_today(s))?;
        let done = today.lessons.last().is_some_and(|l| l.finished.is_some());
        done.then_some(today)
    }
}
