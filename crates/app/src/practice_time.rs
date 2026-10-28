//! A practice sheet's time and its finish (S89; `openpaint_practice::time`): each use while the
//! sheet shows (the pen touching or hovering, a finger, a control) runs the record's last span on
//! to it, or starts a new one after a gap over `PAUSE`, after the app was hidden or when the sheet
//! opens again (the time between was not spent on it). The time goes to the store with every
//! other change, at least once a minute while it grows, when the app is hidden and when the sheet
//! is left. Once a session is done: its dot on its stage's road and the seals its work earned
//! beside every other record's; once a drill is done, the seals only.

use openpaint_practice::{STAGE_SESSIONS, Seal, practised, sealed, seconds};

use super::PracticeSheet;

/// Seconds of time not yet stored after which the record goes to the store again.
const SAVE_EVERY: u64 = 60;

/// A session or drill done, as its finish shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    /// A session's dot on its stage's road: the sessions of its stage finished, this one the last
    /// of them, and about how many the stage holds; none for a drill, which is no session (S91).
    pub road: Option<(u32, u32)>,
    /// The seals its work earned.
    pub sealed: Vec<Seal>,
}

impl PracticeSheet {
    /// A use at `now` (seconds since 1970 UTC) while the sheet shows.
    pub(crate) fn practised(&mut self, now: u64) {
        let hidden = std::mem::take(&mut self.hidden);
        let began = practised(&mut self.record.time, now, hidden);
        let unsaved = seconds(&self.record.time).saturating_sub(self.time_saved);
        if began || unsaved >= SAVE_EVERY {
            self.changed = true;
        }
    }

    /// The app is hidden: the next use starts a new span, and the time goes to the store.
    pub(crate) fn hidden(&mut self) {
        self.hidden = true;
        self.flush();
    }

    /// The time goes to the store with the next save request, if it grew since the last.
    pub(crate) fn flush(&mut self) {
        if seconds(&self.record.time) != self.time_saved {
            self.changed = true;
        }
    }

    /// The finish of a session or drill done.
    pub(super) fn finished(&self) -> Option<Finished> {
        let done = self.current().is_none() && self.record.finished.is_some();
        if !done || self.record.is_project() {
            return None;
        }
        let done = u32::try_from(self.before.len() + 1).unwrap_or(u32::MAX);
        Some(Finished {
            road: self.record.is_session().then_some((done, STAGE_SESSIONS)),
            sealed: sealed(&self.others, &self.record),
        })
    }
}
