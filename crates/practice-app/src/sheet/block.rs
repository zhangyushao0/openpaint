//! The open sheet in a Pomodoro's block (S124, `crate::pomodoro`): End cuts the lesson in hand to
//! the exercises begun (the one in hand and those before it; those after it go from the record,
//! from the sheet as the screen draws it, `SheetView`, and from the stored givens), finishes it at
//! the tick and fits the view to the rows drawn in the block (its lessons finished at or after the
//! block's start, as `BlockTally` counts them), in the room left of the card. The record cut goes
//! to the store with its givens in one write (`trim_request`, `practice_store::trim`), before any
//! other save of it.
//!
//! A lesson whose first exercise is the one in hand, neither drawn in nor compared, leaves nothing
//! (a lesson the chain laid as the time ran out), unless it is its sheet's only one: a sheet keeps
//! a lesson, so that lesson keeps its first exercise. Whether anything of the block was begun on
//! the sheet at all (`begun_since`) the log asks first: a block with nothing begun is not ended
//! here. The session's side of End (what it ends first, the history cut) is `SheetSession`'s.

use std::sync::Arc;

use openpaint_app::{Availability, Viewport};
use openpaint_practice::{Layout, Sheet, seconds};

use super::{Beside, PracticeSheet, unavailable};
use crate::copy::holds_copy;
use crate::refused::Refused;

impl PracticeSheet {
    /// The record End cut, once, for the store to write with its last lesson's givens; none
    /// otherwise (the runtime asks it before `save_request`).
    pub fn trim_request(&mut self) -> Option<Sheet> {
        if !std::mem::take(&mut self.trimmed) {
            return None;
        }
        // This write carries the record as it stands, its time with it.
        self.changed = false;
        self.time_saved = seconds(&self.record.spans);
        Some(self.record.clone())
    }

    /// What the lesson in hand is set beside, from the log again (after End took a lesson off).
    pub(crate) fn set_beside(&mut self, beside: Beside) {
        self.beside = beside;
        self.ended();
    }

    /// End at `now` of a block begun at `since`: the lesson in hand cut to the exercises begun and
    /// finished, the view fitting the rows drawn in the block within `viewport`. Returns the
    /// record as it stands.
    pub(crate) fn end_block(
        &mut self,
        session: (&openpaint_core::Artwork, &mut openpaint_app::CanvasView),
        since: u64,
        (now, viewport): (u64, &Viewport),
    ) -> Result<Sheet, Refused> {
        if self.record.is_copy() {
            return Err(unavailable(Availability::NothingLeft));
        }
        // A lesson the chain appended waits for the store before End may cut it.
        if self.appending.is_some() {
            return Err(unavailable(Availability::Running));
        }
        let (artwork, view) = session;
        if let Some(index) = self.current() {
            self.cut(index, artwork, now);
        }
        (self.compared, self.instructions, self.notice) = (None, None, None);
        (self.fit_job, self.aligned, self.construction) = (None, false, false);
        *view = self.rows_from(self.block_top(since), viewport);
        self.shown();
        Ok(self.record.clone())
    }

    /// The lesson in hand, exercise `index` shown, ended at `now`: its exercises after `index`
    /// taken off, or the whole lesson where `index` is its first and was not begun on `artwork`.
    fn cut(&mut self, index: usize, artwork: &openpaint_core::Artwork, now: u64) {
        let first = self
            .record
            .held()
            .map_or(index, |held| held.first_exercise());
        let begun = self.record.first(index).is_some()
            || (self.pair(index)).is_some_and(|pair| holds_copy(artwork, &pair.copy));
        let hand = self.record.in_hand();
        let before = self.record.exercises().count();
        if index == first && !begun && self.record.lessons.len() > 1 {
            self.record.lessons.remove(hand);
        } else if let Some(lesson) = self.record.lesson_mut() {
            lesson.exercises.truncate(index - first + 1);
            lesson.finished = Some(now);
        }
        let kept = self.record.exercises().count();
        if kept < before {
            self.trimmed = true;
            let mut sheet = (*self.sheet).clone();
            sheet.pairs = Layout::on(&self.record, sheet.size).pairs;
            sheet.pictures.truncate(kept);
            sheet.painted.truncate(kept);
            sheet.givens.truncate(kept);
            sheet.kinds.truncate(kept);
            sheet.rows.truncate(self.record.lessons.len());
            self.sheet = Arc::new(sheet);
        } else {
            self.changed = true;
        }
    }

    /// Whether an exercise of a block begun at `since` was begun on this sheet (drawn in on
    /// `artwork` or compared): those of its lessons finished at or after `since` and of the lesson
    /// in hand up to the exercise shown.
    pub(crate) fn begun_since(&self, artwork: &openpaint_core::Artwork, since: u64) -> bool {
        let mut first = 0;
        let mut indices = Vec::new();
        for lesson in &self.record.lessons {
            let count = lesson.exercises.len();
            match lesson.finished {
                Some(finished) if finished >= since => indices.extend(first..first + count),
                Some(_) => {}
                None => indices.extend(first..=first + usize::from(lesson.at)),
            }
            first += count;
        }
        indices.into_iter().any(|index| {
            self.record.first(index).is_some()
                || (self.pair(index)).is_some_and(|pair| holds_copy(artwork, &pair.copy))
        })
    }

    /// The top of the rows drawn in a block begun at `since`: its first lesson's on this sheet
    /// (those finished at or after `since`); the sheet's top where none is.
    fn block_top(&self, since: u64) -> u32 {
        let lesson = (self.record.lessons.iter())
            .find(|lesson| lesson.finished.is_some_and(|finished| finished >= since));
        let slot = lesson.and_then(|lesson| lesson.exercises.first());
        slot.map_or(0, |laid| {
            let [_, top, _, _] = Layout::row_rect(usize::from(laid.slot));
            top as u32
        })
    }
}
