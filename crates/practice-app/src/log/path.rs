//! The path from the log (S122, `crate::path`): the learner the records tell of, the path made
//! again as they change (the units with their skills' dots, the lessons offered, a unit's re-tests
//! due, the open day's lesson), a lesson started (`start`), a sheet opened on its lesson in hand
//! (`practising`, with
//! what its lesson is set beside: `beside`) and each lesson's label (`rows`).

use std::collections::HashSet;
use std::sync::Arc;

use openpaint_app::{ProjectId, Title};
use openpaint_practice::{
    Given, Learner, Pictures, Purpose, Sheet, Standing, Tally, UNITS, lessons, number, retest,
    schedule,
};

use super::PracticeLog;
use crate::lesson::LessonRow;
use crate::path::{OpenLesson, PathProjection, SkillShown, Start, UnitShown};
use crate::pictures::PackShown;
use crate::projection::SheetView;
use crate::sheet::{Beside, Past, Practising};

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
        let pictures = self.pictures();
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
        let first = learner.fresh();
        let sheets = &self.sheets;
        let start = retest::start_due(&standing);
        let end = retest::end_due(learner);
        self.path = PathProjection {
            units,
            unit: learner.unit(&pictures),
            next: schedule::next(&standing).filter(|_| !first),
            review: schedule::review(&standing),
            first,
            retest: start.and_then(|unit| retest::offer(unit, false, sheets, &pictures)),
            retest_end: end.and_then(|unit| retest::offer(unit, true, sheets, &pictures)),
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
    /// pictures are not kept.
    #[must_use]
    pub fn start(&self, start: Start, seed: u64, now: u64) -> Option<(Sheet, Title)> {
        let pictures = self.pictures();
        let standing = Standing {
            pictures: &pictures,
            learner: &self.learner,
        };
        let due = retest::start_due(&standing);
        let planned = match start {
            Start::First => schedule::first(seed),
            Start::Next if self.learner.fresh() => schedule::first(seed),
            Start::Next if due.is_some() => {
                let unit = due?;
                retest::retest(unit, false, &self.sheets, &pictures)?
            }
            Start::Retest(unit, end) => retest::retest(unit, end, &self.sheets, &pictures)?,
            Start::Next => {
                let offer = schedule::next(&standing)?;
                schedule::chosen(offer.skill, seed, &standing)?
            }
            Start::Review => schedule::reviewed(seed, &standing)?,
            Start::Skill(skill) => schedule::chosen(skill, seed, &standing)?,
            Start::At(skill, level) => schedule::lesson(skill, level, seed, &standing)?,
        };
        let record = Sheet::begun(self.next_id(), (Purpose::Day, now), planned);
        let lesson = self.learner.lessons() + 1;
        let title = Title::new(&format!("Lesson {lesson}")).unwrap_or_else(|_| Title::untitled());
        Some((record.framed(&self.packs), title))
    }

    /// The sheet of `sheet` opening on its lesson in hand, on its canvas of `size` pixels, titled
    /// `title`, what each exercise printed as `printed` holds it: what `SheetSession::open` takes. The
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
    /// learner every other lesson tells of and the pictures kept (`pictures`).
    pub(crate) fn beside(&self, sheet: &Sheet) -> Beside {
        let others = self.sheets.iter().filter(|other| other.id != sheet.id);
        Beside {
            others: Tally::beside(&self.sheets, sheet),
            learner: self.learner_beside(sheet),
            pictures: self.pictures(),
            past: Arc::new(Past {
                sheets: others.cloned().collect(),
                pairs: self.pairs.clone(),
                skipped: self.asked_today(),
                offset: self.offset,
            }),
        }
    }

    /// The pictures kept as lessons pick from them: the packs', the photos drawings from life were
    /// drawn against set aside (S158: never a shape lesson's).
    fn pictures(&self) -> Pictures {
        let photos = (self.sheets.iter())
            .filter(|sheet| sheet.drawn_from_life())
            .filter_map(|sheet| sheet.laid(0)?.exercise.picture.as_ref());
        Pictures::of(&self.packs).photos_aside(photos)
    }

    /// Each lesson of `sheet` as its row's label names it, numbered among the learner's lessons
    /// (`openpaint_practice::number`); none on a long colour copy's.
    pub(crate) fn rows(&self, sheet: &Sheet) -> Vec<LessonRow> {
        LessonRow::of_sheet(sheet, |lesson| number(&self.sheets, sheet, lesson))
    }

    /// The kinds whose How need not show by itself on `sheet`: told on a sheet opened since the
    /// app started (`tell`), or by the records of any sheet (it among them, as it stands now), by
    /// their facts' keys (`told_on`).
    fn told(&self, sheet: &Sheet) -> HashSet<&'static str> {
        (self.sheets.iter().filter(|other| other.id != sheet.id))
            .chain([sheet])
            .flat_map(told_on)
            .chain(self.told.iter().copied())
            .collect()
    }

    /// The kinds whose How the open sheet showed while used (`PracticeSheet::seen`), kept for the
    /// sheets opened after it while the app runs.
    pub fn tell(&mut self, told: &HashSet<&'static str>) {
        if !told.is_subset(&self.told) {
            self.told.extend(told);
        }
    }
}

/// The facts' keys of `sheet`'s exercises whose How its record tells was told: compared, or come
/// up in hand, where a How shows by itself. An
/// exercise came up once its lesson moved past it or on to it, or, the first of a
/// lesson, once the sheet was used in a span begun after that lesson was laid (a later second): a
/// lesson laid but never shown tells nothing.
fn told_on(sheet: &Sheet) -> impl Iterator<Item = &'static str> + '_ {
    let used_since = |started: u64| sheet.spans.iter().any(|span| span.from > started);
    sheet.lessons.iter().flat_map(move |lesson| {
        let reached = match lesson.finished {
            Some(_) => lesson.exercises.len(),
            None if lesson.at > 0 || used_since(lesson.started) => usize::from(lesson.at) + 1,
            None => 0,
        };
        (lesson.exercises.iter().enumerate())
            .filter(move |(at, laid)| laid.first.is_some() || *at < reached)
            .map(|(_, laid)| laid.exercise.kind.facts().key)
    })
}
