//! The path from the log (S122, `crate::path`): the learner the records tell of, the units with
//! their skills' dots, the lessons offered, a lesson started (`start`) and a sheet opened on its
//! lesson in hand (`practising`).
//!
//! The skeleton's bodies. S122b: `begin`, `begin_drill`, the stage and the gallery's card go, the
//! path is made again only as the log changes, and a sheet opens with the learner beside its
//! lesson in hand.

use std::sync::Arc;

use openpaint_app::{ProjectId, Title};
use openpaint_practice::{
    Given, Learner, Pictures, Purpose, Sheet, Standing, UNITS, lessons, number, schedule,
};

use super::PracticeLog;
use crate::lesson::LessonRow;
use crate::path::{PathProjection, SkillShown, Start, UnitShown};
use crate::projection::SheetView;
use crate::sheet::Practising;

impl PracticeLog {
    /// The learner the records tell of.
    #[must_use]
    pub const fn learner(&self) -> &Learner {
        &self.learner
    }

    /// The learner every lesson but `sheet`'s in hand tells of: what that lesson's end sets its
    /// skill's dots beside (`Learner::with`).
    #[must_use]
    pub fn learner_beside(&self, sheet: &Sheet) -> Learner {
        let mut own = sheet.clone();
        let hand = own.in_hand();
        if hand < own.lessons.len() {
            own.lessons.remove(hand);
        }
        let others = self.sheets.iter().filter(|kept| kept.id != sheet.id);
        Learner::of(others.chain([&own]))
    }

    /// The home's practice column, the path page and the practice book.
    #[must_use]
    pub fn path(&self) -> PathProjection {
        let pictures = Pictures::of(&self.packs);
        let learner = &self.learner;
        let standing = Standing {
            pictures: &pictures,
            learner,
        };
        let shown = |skill| SkillShown {
            skill,
            dots: learner.dots(skill),
            lacks: skill.lacks(&pictures),
            redo: learner.redo(skill).is_some(),
        };
        let units = (UNITS.iter())
            .map(|unit| UnitShown {
                number: unit.number,
                open: learner.open(unit.number, &pictures),
                skills: unit.skills.iter().copied().map(shown).collect(),
            })
            .collect();
        let today = self.today();
        let first = learner.fresh();
        PathProjection {
            units,
            unit: learner.unit(&pictures),
            next: schedule::next(&standing).filter(|_| !first),
            review: schedule::review(&standing),
            first,
            open: self.open_lesson(),
            packs: today.packs.clone(),
            copies: today.copies.clone(),
            pictures: Arc::clone(&today.pictures),
            book: today.book.clone(),
            today,
        }
    }

    /// The day's sheet whose lesson in hand is unfinished, the newest.
    fn open_lesson(&self) -> Option<ProjectId> {
        (lessons(&self.sheets).iter().rev())
            .find(|on| {
                on.sheet.purpose == Purpose::Day
                    && on.index == on.sheet.in_hand()
                    && on.finished().is_none()
                    && on.sheet.canvas != 0
            })
            .map(|on| ProjectId::new(on.sheet.canvas))
    }

    /// A lesson started as `start` says, seeded `seed`, at `now` (seconds since 1970 UTC), on a
    /// sheet of its own for the day (the runtime lays it onto today's sheet where that takes it,
    /// S121c), with its sheet's title; it has no canvas until the store gives it one. None where
    /// there is nothing to start: no skill to practise, no redo to review, a skill whose
    /// pictures are not kept, a unit not built.
    #[must_use]
    pub fn start(&self, start: Start, seed: u64, now: u64) -> Option<(Sheet, Title)> {
        let pictures = Pictures::of(&self.packs);
        let standing = Standing {
            pictures: &pictures,
            learner: &self.learner,
        };
        let planned = match start {
            Start::First => schedule::first(seed),
            Start::Next if self.learner.fresh() => schedule::first(seed),
            Start::Next => {
                let offer = schedule::next(&standing)?;
                schedule::chosen(offer.skill, seed, &standing)?
            }
            Start::Review => schedule::reviewed(seed, &standing)?,
            Start::Skill(skill) => schedule::chosen(skill, seed, &standing)?,
            Start::At(skill, level) => schedule::lesson(skill, level, seed, &standing)?,
            Start::Full(unit) => schedule::full(unit, seed, &standing)?,
        };
        let record = Sheet::begun(self.next_id(), (Purpose::Day, now), planned);
        let lesson = self.learner.lessons() + 1;
        let title = Title::new(&format!("Lesson {lesson}")).unwrap_or_else(|_| Title::untitled());
        Some((record.framed(&self.packs), title))
    }

    /// The sheet of `session` opening on its lesson in hand, on its canvas of `size` pixels,
    /// titled `title`, what each exercise printed as `printed` holds it: what `open_sheet` takes.
    /// The one place a `Practising` is made, so its shape is the flow's own.
    #[must_use]
    pub fn practising(
        &self,
        session: &Sheet,
        printed: &[Given],
        (size, title): ([u32; 2], &Title),
    ) -> Practising {
        let numbered = (size, title, self.number(session));
        let mut view = SheetView::of(session, printed, &self.packs, numbered);
        view.rows = LessonRow::of_sheet(session, |lesson| number(&self.sheets, session, lesson));
        let before = self.before(session);
        (session.clone(), before, view, self.tally_beside(session))
    }
}
