//! The lesson in hand as the screen shows it (S122, `crate::lesson`): how the bar names it, the
//! result the user chooses where the eye judges, who judges the shown comparison, and the
//! lesson's end.
//!
//! The skeleton's bodies, the least that fills the contract. S122b: the flow itself: a result
//! asked for before Next where the eye judges, the end's dots from the learner beside the lesson
//! (`PracticeLog::learner_beside`, `Learner::with`), the lesson One more starts
//! (`schedule::next`), the How by kind.

use openpaint_app::Availability;
use openpaint_practice::{Outcome, SkillLevel, Verdict};

use super::{PracticeSheet, unavailable};
use crate::lesson::{End, Judged, LessonShown};
use crate::refused::Refused;

impl PracticeSheet {
    /// The lesson in hand as the bar names it; none on a long colour copy's sheet.
    pub(super) fn lesson_shown(&self) -> Option<LessonShown> {
        let held = self.record.held()?;
        let row = *self.sheet.rows.get(held.index)?;
        let exercises = &held.lesson.exercises;
        let at = usize::from(held.lesson.at).min(exercises.len().checked_sub(1)?);
        Some(LessonShown {
            row,
            level: SkillLevel::of_code(exercises[at].level),
            place: (at + 1, exercises.len()),
            warming_up: at < held.lesson.warm_up(),
        })
    }

    /// The user chose `outcome` for the shown exercise: kept with it, once, while its comparison
    /// shows and the eye judges it.
    pub(super) fn judge(&mut self, outcome: Outcome) -> Result<(), Refused> {
        let eye = self
            .judged()
            .is_some_and(|judged| judged.verdict == Verdict::Eye);
        let shown = self.current().filter(|_| eye);
        let Some(laid) = shown.and_then(|index| self.record.laid_mut(index)) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        laid.judged(outcome);
        self.changed = true;
        Ok(())
    }

    /// Who judges the shown comparison and the result kept; none while nothing is compared.
    pub(super) fn judged(&self) -> Option<Judged> {
        let index = self.current()?;
        let verdict = self.compared.as_ref()?.shown().verdict();
        let result = self.record.laid(index)?.result;
        Some(Judged { verdict, result })
    }

    /// The end of the lesson in hand, once it is done; none on a long colour copy's sheet.
    pub(super) fn end(&self) -> Option<End> {
        let finished = self.finished()?;
        let held = self.record.held()?;
        let first = held.first_exercise();
        let results = (held.lesson.exercises.iter().enumerate())
            .map(|(at, laid)| (first + at, laid.result.map(|(outcome, _)| outcome)))
            .collect();
        Some(End {
            results,
            skill: None,
            sealed: finished.sealed,
            next: None,
        })
    }
}
