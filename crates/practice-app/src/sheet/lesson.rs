//! The lesson in hand as the screen shows it (S122, `crate::lesson`): how the bar names it, the
//! result the user chooses where the eye judges (once; Next without one leaves none, nothing
//! asks for it), who judges the shown comparison (nothing is scored on the first lesson: no
//! result kept, none chosen, none on its end), and the lesson's end: each exercise's result,
//! how many exercises it held and the seconds they took,
//! its skill's dots before it (the learner beside it, `PracticeLog::learner_beside`) and after it
//! (`Learner::with`), the seals it earned and the lesson One more starts (`schedule::next` on the
//! learner after it; the same skill again after a lesson of a unit past the learner's), its blind
//! pairs and extra drill (`pairs.rs`, S147). The end is kept once the lesson is done (`shown`); a
//! result a painted copy's reading brings after it makes it again.

use openpaint_app::Availability;
use openpaint_practice::{
    Laid, Lesson, Outcome, Picked, Sheet, Skill, SkillLevel, Standing, Unit, Verdict, retest,
    schedule, sealed,
};

use super::{PracticeSheet, unavailable};
use crate::lesson::{End, Judged, LessonShown, OneMore};
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
            place: place(held.lesson, at),
            warming_up: at < held.lesson.warm_up(),
        })
    }

    /// The user chose `outcome` for the shown exercise: kept with it, once, while its comparison
    /// shows, the eye judges it and nothing was chosen yet.
    pub(super) fn judge(&mut self, outcome: Outcome) -> Result<(), Refused> {
        let open = (self.judged())
            .is_some_and(|judged| judged.verdict == Verdict::Eye && judged.result.is_none());
        let shown = self.current().filter(|_| open);
        let Some(laid) = shown.and_then(|index| self.record.laid_mut(index)) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        laid.judged(outcome);
        self.changed = true;
        Ok(())
    }

    /// Who judges the shown comparison and the result kept; none while nothing is compared, and
    /// never on the first lesson.
    pub(super) fn judged(&self) -> Option<Judged> {
        let index = self.current().filter(|_| self.scored())?;
        let verdict = self.compared.as_ref()?.shown().verdict();
        let result = self.record.laid(index)?.result;
        Some(Judged { verdict, result })
    }

    /// The end of the lesson in hand, once it is done, its pairs as answered so far; none on a
    /// long colour copy's sheet.
    pub(super) fn end(&self) -> Option<End> {
        let results = self.results()?;
        let end = match &self.end {
            Some(kept) if kept.results == results => Some(kept.clone()),
            _ => self.ending(),
        };
        end.map(|end| End {
            pairs: self.pairs_shown(),
            ..end
        })
    }

    /// The lesson in hand's end kept, once it is done, its pairs asked.
    pub(super) fn ended(&mut self) {
        self.ask_pairs();
        self.end = self.ending();
    }

    /// The end of the lesson in hand as it stands, once it is done.
    fn ending(&self) -> Option<End> {
        let results = self.results()?;
        let lesson = self.record.lesson()?;
        let before = &self.beside.learner;
        let after = before.with(lesson);
        let standing = Standing {
            pictures: &self.beside.pictures,
            learner: &after,
        };
        let seconds = lesson.exercises.iter().map(|laid| laid.seconds).sum();
        let sheets = self.with_this();
        Some(End {
            results,
            exercises: u8::try_from(lesson.exercises.len()).unwrap_or(u8::MAX),
            seconds,
            skill: (lesson.skill()).map(|skill| (skill, before.dots(skill), after.dots(skill))),
            sealed: sealed(&self.beside.others, &self.record),
            next: one_more(lesson, (&standing, &sheets)),
            pairs: self.pairs_shown(),
            drill: self.drill(&standing, &sheets),
        })
    }

    /// Each result marked under its pair on the sheet, by the exercise's index among the sheet's:
    /// those of the lessons before the one in hand, and its own once it is done.
    pub(super) fn marked(&self) -> Vec<(usize, Outcome)> {
        let held = self.record.held();
        let before = held.map_or(usize::MAX, |held| held.first_exercise());
        let done = self.results().is_some();
        (self.record.exercises().enumerate())
            .filter(|(index, _)| *index < before || done)
            .filter_map(|(index, laid)| Some((index, laid.result?.0)))
            .collect()
    }

    /// Whether the lesson in hand's results are kept: on every lesson but the first, where
    /// nothing is scored.
    pub(super) fn scored(&self) -> bool {
        (self.record.lesson()).is_none_or(|lesson| lesson.picked != Picked::First)
    }

    /// Each exercise of the lesson in hand, by its index among the sheet's, and the result it
    /// ended with (none at all on the first lesson); once it is done, never on a long colour
    /// copy's sheet.
    fn results(&self) -> Option<Vec<(usize, Option<Outcome>)>> {
        let done = self.current().is_none() && self.record.finished().is_some();
        let held = self
            .record
            .held()
            .filter(|_| done && !self.record.is_copy())?;
        if !self.scored() {
            return Some(Vec::new());
        }
        let first = held.first_exercise();
        let results = (held.lesson.exercises.iter().enumerate())
            .map(|(at, laid)| (first + at, laid.result.map(|(outcome, _)| outcome)))
            .collect();
        Some(results)
    }
}

/// The lesson One more starts after `lesson`, the learner after it standing as `standing` among
/// `sheets`: the same skill again at its level where the lesson was a skill's of a unit past the
/// learner's (entered by hand ahead of the path); else as Start would lay it, the unit's start
/// re-test while it is due (S148), else the path's next (`schedule::next`); a full lesson and the
/// first lesson are never of a skill.
fn one_more(lesson: &Lesson, (standing, sheets): (&Standing<'_>, &[Sheet])) -> Option<OneMore> {
    let in_hand = standing.learner.unit(standing.pictures);
    let ahead = (lesson.skill())
        .filter(|skill| Unit::teaching(*skill).is_some_and(|unit| unit.number > in_hand));
    if let Some(skill) = ahead {
        return schedule::offer(skill, standing).map(OneMore::Skill);
    }
    let due = retest::start_due(standing);
    let retest = due.and_then(|unit| retest::offer(unit, false, sheets, standing.pictures));
    match retest {
        Some(retest) => Some(OneMore::Retest(retest)),
        None => schedule::next(standing).map(OneMore::Skill),
    }
}

/// Exercise `at`'s place in `lesson`, from 1, and how many there are: in a full lesson's warm-up
/// among the warm-up's, after it among those of its skill after it; in any other lesson among all
/// of it.
fn place(lesson: &Lesson, at: usize) -> (usize, usize) {
    let exercises = &lesson.exercises;
    let warm_up = lesson.warm_up().min(exercises.len());
    if at < warm_up {
        return (at + 1, warm_up);
    }
    if !matches!(lesson.picked, Picked::Full { .. }) {
        return (at + 1, exercises.len());
    }
    let skill = Skill::of(exercises[at].exercise.kind);
    let alike = |laid: &&Laid| Skill::of(laid.exercise.kind) == skill;
    let before = exercises[warm_up..at].iter().filter(alike).count();
    let all = exercises[warm_up..].iter().filter(alike).count();
    (before + 1, all)
}
