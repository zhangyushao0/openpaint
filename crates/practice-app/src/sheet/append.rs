//! One more in place (S121c): a lesson done on a day's (or a test's) sheet started today, the
//! next lesson is laid on the same sheet from the row after the last one used, where the rows left
//! on its canvas hold it, and the view glides to its first pair. Its record and what it prints go
//! to the store in one write; until the store answers the record waits (`save_request`), and a
//! write that failed rolls the lesson back: the sheet, the end and the view as they were, the
//! notice a failed save shows, and the end shown even while a Pomodoro's block runs (the chain's
//! lesson rolled back leaves One more to try again, S124).

use std::sync::Arc;

use openpaint_practice::{Given, Givens, Layout, Lesson, Purpose, Sheet};

use super::{Beside, Painting, PracticeSheet};
use crate::log::PracticeLog;
use crate::projection::{PracticeNotice, SheetView};
use openpaint_app::{CanvasView, Viewport};

/// What a lesson appended rolls back to if the store does not take it.
#[derive(Debug)]
pub(super) struct Appending {
    sheet: Arc<SheetView>,
    beside: Beside,
    view: CanvasView,
}

impl PracticeSheet {
    /// Whether a lesson of `exercises` goes onto this sheet in place: its lesson in hand done,
    /// none waiting for the store, a day's or a test's sheet started `today`, the rows left on
    /// its canvas holding the lesson.
    #[must_use]
    pub(crate) fn takes(&self, exercises: usize, today: bool) -> bool {
        let done = self.current().is_none() && self.record.finished().is_some();
        let day = matches!(self.record.purpose, Purpose::Day | Purpose::Test);
        let size = [self.canvas.width(), self.canvas.height()];
        done && day
            && today
            && self.appending.is_none()
            && Layout::fits(&self.record, exercises, size)
    }

    /// `lesson` (as begun on a sheet of its own) and `givens`, what it prints, appended from the
    /// row after the last one used, the view `view` gliding to its first pair in `viewport`; its
    /// row's label and what it is set beside from `log`. Returns the record and the givens, moved
    /// to their slots, for the store.
    pub(crate) fn append(
        &mut self,
        (lesson, givens): (Lesson, Givens),
        log: &PracticeLog,
        (view, viewport): (&mut CanvasView, &Viewport),
    ) -> (Sheet, Givens) {
        let moved = self.record.append(lesson, givens);
        let kept = Appending {
            sheet: Arc::clone(&self.sheet),
            beside: std::mem::replace(&mut self.beside, log.beside(&self.record)),
            view: *view,
        };
        let printed: Vec<Given> = (self.record.lessons.last().into_iter())
            .flat_map(|lesson| lesson.exercises.iter())
            .map(|laid| moved.of(laid.slot).cloned().unwrap_or_default())
            .collect();
        let rows = log.rows(&self.record);
        let sheet = (self.sheet).appended(&self.record, &printed, log.packs(), rows);
        self.sheet = Arc::new(sheet);
        self.appending = Some(Box::new(kept));
        self.rolled_back = false;
        self.anew();
        *view = self.one_more_view(viewport);
        (self.record.clone(), moved)
    }

    /// The store wrote sheet `id`'s lesson appended (`stored`), or did not: then the lesson goes,
    /// the sheet and `view` as they were before One more, the notice a failed save shows.
    pub(crate) fn appended(&mut self, id: u64, stored: bool, view: &mut CanvasView) {
        if id != self.record.id {
            return;
        }
        let Some(kept) = self.appending.take() else {
            return;
        };
        // Whatever changed while it waited goes now (or the record as it was again).
        self.changed = true;
        if stored {
            return;
        }
        self.record.lessons.pop();
        self.sheet = kept.sheet;
        self.beside = kept.beside;
        *view = kept.view;
        self.anew();
        self.notice = Some(PracticeNotice::NotSaved);
        self.rolled_back = true;
    }

    /// The lesson in hand shown afresh, as a sheet opening on it shows it.
    fn anew(&mut self) {
        (self.compared, self.instructions, self.notice) = (None, None, None);
        (self.fit_job, self.aligned, self.construction) = (None, false, false);
        self.painting_state = Painting::default();
        self.all_tools = false;
        self.shown();
    }
}
