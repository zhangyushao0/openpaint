//! A practice sheet's time (S89; `openpaint_practice::time`): each use while the sheet shows (the
//! pen touching or hovering, a finger, a control) runs the record's last span on to it, or starts
//! a new one after a gap over `PAUSE`, after the app was hidden or when the sheet opens again (the
//! time between was not spent on it). The time goes to the store with every other change, at
//! least once a minute while it grows, when the app is hidden and when the sheet is left.

use openpaint_practice::{practised, seconds};

use super::PracticeSheet;

/// Seconds of time not yet stored after which the record goes to the store again.
const SAVE_EVERY: u64 = 60;

impl PracticeSheet {
    /// A use at `now` (seconds since 1970 UTC) while the sheet shows.
    pub fn practised(&mut self, now: u64) {
        let hidden = std::mem::take(&mut self.hidden);
        let before = seconds(&self.record.spans);
        let began = practised(&mut self.record.spans, now, hidden);
        let after = seconds(&self.record.spans);
        // The span's growth is the exercise in hand's.
        if let Some(index) = self.current()
            && let Some(laid) = self.record.laid_mut(index)
        {
            let grown = u32::try_from(after.saturating_sub(before)).unwrap_or(u32::MAX);
            laid.seconds = laid.seconds.saturating_add(grown);
        }
        let unsaved = after.saturating_sub(self.time_saved);
        if began || unsaved >= SAVE_EVERY {
            self.changed = true;
        }
    }

    /// The app is hidden: the next use starts a new span, and the time goes to the store.
    pub fn hidden(&mut self) {
        self.hidden = true;
        self.flush();
    }

    /// The time goes to the store with the next save request, if it grew since the last.
    pub fn flush(&mut self) {
        if seconds(&self.record.spans) != self.time_saved {
            self.changed = true;
        }
    }
}
