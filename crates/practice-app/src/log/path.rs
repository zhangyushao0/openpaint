//! The path from the log (S122, `crate::path`): the learner the records tell of, the path made
//! again as they change (the units with their skills' dots, the lessons offered, the open day's
//! lesson), a lesson started (`start`), a sheet opened on its lesson in hand (`practising`, with
//! what its lesson is set beside: `beside`) and each lesson's label (`rows`).

use std::collections::HashSet;
use std::sync::Arc;

use openpaint_app::{ProjectId, Title};
use openpaint_practice::{
    Given, Learner, Pictures, Purpose, Sheet, Standing, Tally, UNITS, lessons, number, schedule,
};

use super::PracticeLog;
use crate::lesson::LessonRow;
use crate::path::{OpenLesson, PathProjection, SkillShown, Start, UnitShown};
use crate::pictures::PackShown;
use crate::projection::SheetView;
use crate::sheet::{Beside, Practising};

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

    /// The home's practice column, the path page and the practice book, as the log last made
    /// them.
    #[must_use]
    pub fn path(&self) -> PathProjection {
        self.path.clone()
    }

    /// The path made again from the records, the packs and the clock.
    pub(super) fn pathed(&mut self) {
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
                full: schedule::full_offer(unit.number, &standing),
            })
            .collect();
        let first = learner.fresh();
        self.path = PathProjection {
            units,
            unit: learner.unit(&pictures),
            next: schedule::next(&standing).filter(|_| !first),
            review: schedule::review(&standing),
            first,
            open: self.open_lesson(),
            packs: self.packs.iter().map(PackShown::of).collect(),
            copies: self.copies(),
            pictures: Arc::clone(&self.pictures),
            book: self.book(),
        };
    }

    /// The day's lesson in hand left unfinished, the newest: its sheet, its row's label and the
    /// exercise in hand's place.
    fn open_lesson(&self) -> Option<OpenLesson> {
        let all = lessons(&self.sheets);
        let on = (all.iter().rev()).find(|on| {
            on.sheet.purpose == Purpose::Day
                && on.index == on.sheet.in_hand()
                && on.finished().is_none()
                && on.sheet.canvas != 0
        })?;
        let row = *self.rows(on.sheet).get(on.index)?;
        let count = on.lesson.exercises.len();
        let at = usize::from(on.lesson.at).min(count.saturating_sub(1));
        Some(OpenLesson {
            sheet: ProjectId::new(on.sheet.canvas),
            row,
            place: (at + 1, count),
        })
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

    /// The sheet of `sheet` opening on its lesson in hand, on its canvas of `size` pixels, titled
    /// `title`, what each exercise printed as `printed` holds it: what `open_sheet` takes. The
    /// one place a `Practising` is made.
    #[must_use]
    pub fn practising(
        &self,
        sheet: &Sheet,
        printed: &[Given],
        (size, title): ([u32; 2], &Title),
    ) -> Practising {
        let rows = self.rows(sheet);
        Practising {
            record: sheet.clone(),
            view: SheetView::of(sheet, printed, &self.packs, (size, title, rows)),
            beside: self.beside(sheet),
            told: self.told(sheet),
        }
    }

    /// What `sheet`'s lesson in hand is set beside: what every other lesson adds up to, the
    /// learner every other lesson tells of and the pictures kept.
    pub(crate) fn beside(&self, sheet: &Sheet) -> Beside {
        Beside {
            others: Tally::beside(&self.sheets, sheet),
            learner: self.learner_beside(sheet),
            pictures: Pictures::of(&self.packs),
        }
    }

    /// Each lesson of `sheet` as its row's label names it, numbered among the learner's lessons
    /// (`openpaint_practice::number`); none on a long colour copy's.
    pub(crate) fn rows(&self, sheet: &Sheet) -> Vec<LessonRow> {
        LessonRow::of_sheet(sheet, |lesson| number(&self.sheets, sheet, lesson))
    }

    /// The kinds whose How need not show by itself on `sheet`: compared on any sheet (it among
    /// them), by their facts' keys.
    fn told(&self, sheet: &Sheet) -> HashSet<&'static str> {
        (self.sheets.iter().chain([sheet]))
            .flat_map(Sheet::exercises)
            .filter(|laid| laid.first.is_some())
            .map(|laid| laid.exercise.kind.facts().key)
            .collect()
    }
}
