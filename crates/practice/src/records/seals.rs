//! Seals (S89, S122): milestones of practice, stamped in vermilion in the practice book. They are
//! derived from the records, never stored: what has been earned is a function of them, so a
//! record removed (a test lesson) takes its part with it. Each seal counts what was done, never
//! when: no days in a row, quotas or reminders (the author, 2026-10-28).
//!
//! On the path (S122) `FirstSession`, `TenSessions` and `FiftySessions` count lessons finished (a
//! skill's, a full one, the first; a copy's never) and `Stage(n)` is earned when every skill of
//! unit n has its first dot (the learner as the records tell it, `course/learner.rs`); the names
//! stand through S122.

use crate::course::learner::Learner;
use crate::course::unit::UNITS;
use crate::records::sheet::Sheet;
use crate::records::time::seconds;
use crate::sheets::exercise::Family;

/// A milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Seal {
    /// The first lesson finished, then ten and fifty.
    FirstSession,
    TenSessions,
    FiftySessions,
    /// Every skill of unit n with its first dot.
    Stage(u8),
    /// A hundred lines compared (single and combined) and a hundred colours.
    HundredLines,
    HundredColours,
    /// The first long colour copy begun.
    FirstCopy,
    /// Ten hours of practice (`time.rs`).
    TenHours,
}

/// Seconds of practice in ten hours.
const TEN_HOURS: u64 = 10 * 3600;

/// What records add up to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tally {
    /// Lessons finished, a long colour copy's never.
    pub sessions: u32,
    /// Exercises compared: single and combined lines, and colours.
    pub lines: u32,
    pub colours: u32,
    /// Long colour copies begun.
    pub copies: u32,
    /// The time practised, in seconds.
    pub seconds: u64,
    /// The learner the lessons added tell of: the units whose skills all hold a dot.
    learner: Learner,
}

impl Tally {
    /// What `records` add up to, taken in the order of their ids.
    #[must_use]
    pub fn of<'a>(records: impl IntoIterator<Item = &'a Sheet>) -> Self {
        let mut ordered: Vec<&Sheet> = records.into_iter().collect();
        ordered.sort_by_key(|record| record.id);
        let mut tally = Self::default();
        for record in ordered {
            tally.add(record);
        }
        tally
    }

    /// What every record of `records` but `record` adds up to, with `record`'s lessons but the
    /// one in hand: what that lesson's seals are earned beside (`sealed`).
    #[must_use]
    pub fn beside<'a>(records: impl IntoIterator<Item = &'a Sheet>, record: &Sheet) -> Self {
        let mut tally = Self::of(records.into_iter().filter(|other| other.id != record.id));
        let hand = record.in_hand();
        for index in (0..record.lessons.len()).filter(|index| *index != hand) {
            tally.add_lesson(record, index);
        }
        tally
    }

    /// Adds what `record` did: its time and every lesson of it.
    pub fn add(&mut self, record: &Sheet) {
        self.add_time(record);
        for index in 0..record.lessons.len() {
            self.add_lesson(record, index);
        }
    }

    /// Adds `record`'s time, and a long colour copy begun.
    fn add_time(&mut self, record: &Sheet) {
        self.seconds += seconds(&record.spans);
        if record.is_copy() {
            self.copies += u32::from(record.canvas != 0);
        }
    }

    /// Adds what lesson `index` of `record` did; a copy's nothing.
    fn add_lesson(&mut self, record: &Sheet, index: usize) {
        let Some(lesson) = record.lessons.get(index).filter(|_| !record.is_copy()) else {
            return;
        };
        self.sessions += u32::from(lesson.finished.is_some());
        self.learner.take(lesson);
        for laid in lesson.exercises.iter().filter(|laid| laid.first.is_some()) {
            match laid.exercise.kind.family() {
                Family::Line | Family::Combined => self.lines += 1,
                Family::Colour => self.colours += 1,
                _ => {}
            }
        }
    }

    /// The seals it has earned, in a fixed order.
    #[must_use]
    pub fn seals(&self) -> Vec<Seal> {
        let mut seals = Vec::new();
        let mut earn = |earned: bool, seal: Seal| {
            if earned {
                seals.push(seal);
            }
        };
        earn(self.sessions >= 1, Seal::FirstSession);
        earn(self.sessions >= 10, Seal::TenSessions);
        earn(self.sessions >= 50, Seal::FiftySessions);
        for unit in UNITS.iter().filter(|unit| unit.built()) {
            let dotted = (unit.skills.iter()).all(|skill| self.learner.dots(*skill) > 0);
            earn(dotted, Seal::Stage(unit.number));
        }
        earn(self.lines >= 100, Seal::HundredLines);
        earn(self.colours >= 100, Seal::HundredColours);
        earn(self.copies >= 1, Seal::FirstCopy);
        earn(self.seconds >= TEN_HOURS, Seal::TenHours);
        seals
    }
}

/// A seal earned, and the record whose work earned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Earned {
    pub seal: Seal,
    pub by: u64,
}

/// Every seal `records` earned, in the order they were: the records added up by their ids
/// (the order they began in), each seal going to the record that crossed its mark.
#[must_use]
pub fn earned(records: &[Sheet]) -> Vec<Earned> {
    let mut ordered: Vec<&Sheet> = records.iter().collect();
    ordered.sort_by_key(|record| record.id);
    let mut tally = Tally::default();
    let mut earned: Vec<Earned> = Vec::new();
    for record in ordered {
        tally.add(record);
        for seal in tally.seals() {
            if !earned.iter().any(|kept| kept.seal == seal) {
                earned.push(Earned {
                    seal,
                    by: record.id,
                });
            }
        }
    }
    earned
}

/// The seals `record`'s lesson in hand earns beside `others` (what every other lesson adds up
/// to, `Tally::beside`): those its own work and its sheet's time add.
#[must_use]
pub fn sealed(others: &Tally, record: &Sheet) -> Vec<Seal> {
    let before = others.seals();
    let mut with = others.clone();
    with.add_time(record);
    with.add_lesson(record, record.in_hand());
    (with.seals().into_iter())
        .filter(|seal| !before.contains(seal))
        .collect()
}
