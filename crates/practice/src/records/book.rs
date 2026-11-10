//! The practice book (S89) as pure math, all of it from the records: the road through the course's
//! stages (the sessions finished in each, the stage practised), the days practised by the device's
//! local day with their time and their records, the colour's first pick per session (the mean of
//! its colours' first ΔE: the eye's own level, before any correction), the sessions whose first
//! single line shows side by side (the first and the latest), the seals earned, and how much was
//! done in all. Progress counts what was done, never days in a row. Sessions are lessons
//! (`lessons.rs`); days and time are the sheets'.

use crate::records::lessons::{OnSheet, lessons, number};
use crate::records::metric::{Metric, value};
use crate::records::seals::{Earned, earned};
use crate::records::sheet::Sheet;
use crate::records::time::{by_day, seconds};
use crate::sessions::plan::{Family, STAGES};

/// The practice book's contents.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Book {
    /// Sessions finished, and the time practised in sessions, drills and copies, in seconds.
    pub sessions: u32,
    pub seconds: u64,
    pub road: Road,
    /// Each local day practised, oldest first.
    pub days: Vec<Day>,
    /// The colour's first pick of each session holding a compared colour, oldest first: the mean
    /// of its colours' first differences (Oklab units).
    pub colour: Vec<f64>,
    /// The first session with a single line compared, and the latest such one when it is another.
    pub lines: Vec<LineShown>,
    /// The seals earned, in the order they were.
    pub seals: Vec<Earned>,
}

/// The road through the stages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Road {
    /// The stage practised (the last session's).
    pub stage: u8,
    /// Sessions finished in each stage, stage n at n - 1.
    pub done: [u32; STAGES as usize],
}

impl Road {
    /// Sessions finished in `stage`.
    #[must_use]
    pub fn done_in(&self, stage: u8) -> u32 {
        let index = usize::from(stage).wrapping_sub(1);
        self.done.get(index).copied().unwrap_or(0)
    }
}

/// A local day practised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// Days since 1970-01-01 on the device's clock (`time::local_day`).
    pub day: i64,
    /// The time practised on it, in seconds.
    pub seconds: u64,
    /// The sessions and copies practised on it, by id, oldest first.
    pub records: Vec<u64>,
}

/// A session's first single line compared, shown in the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineShown {
    pub record: u64,
    /// The session's number in its stage.
    pub number: u32,
    /// The exercise.
    pub index: u32,
}

impl Book {
    /// The book of `records`, `stage` practised, its days on a clock `offset` minutes from UTC.
    #[must_use]
    pub fn of(records: &[Sheet], stage: u8, offset: i32) -> Self {
        let mut ordered: Vec<&Sheet> = records.iter().collect();
        ordered.sort_by_key(|record| record.id);
        // Drills count in the time and the days, never on the road, the first picks or the lines.
        let sessions: Vec<OnSheet<'_>> = (lessons(records).into_iter())
            .filter(|lesson| lesson.stage().is_some())
            .collect();
        let finished: Vec<&OnSheet<'_>> = (sessions.iter())
            .filter(|session| session.finished().is_some())
            .collect();
        let mut road = Road {
            stage,
            done: [0; STAGES as usize],
        };
        for session in &finished {
            let index = usize::from(session.stage().unwrap_or(0)).wrapping_sub(1);
            if let Some(done) = road.done.get_mut(index) {
                *done += 1;
            }
        }
        Self {
            sessions: u32::try_from(finished.len()).unwrap_or(u32::MAX),
            seconds: ordered.iter().map(|record| seconds(&record.spans)).sum(),
            road,
            days: days(&ordered, offset),
            colour: sessions.iter().filter_map(first_pick).collect(),
            lines: lines(records, &sessions),
            seals: earned(records),
        }
    }

    /// The day `day` if it was practised.
    #[must_use]
    pub fn day(&self, day: i64) -> Option<&Day> {
        self.days.iter().find(|shown| shown.day == day)
    }
}

/// The local days `records` (by id) were practised on.
fn days(records: &[&Sheet], offset: i32) -> Vec<Day> {
    let mut days: Vec<Day> = Vec::new();
    for record in records {
        for (day, seconds) in by_day(&record.spans, offset) {
            match days.iter_mut().find(|kept| kept.day == day) {
                Some(kept) => {
                    kept.seconds += seconds;
                    kept.records.push(record.id);
                }
                None => days.push(Day {
                    day,
                    seconds,
                    records: vec![record.id],
                }),
            }
        }
    }
    days.sort_by_key(|day| day.day);
    days
}

/// The mean of `session`'s colours' first differences, if it compared any.
fn first_pick(session: &OnSheet<'_>) -> Option<f64> {
    let picks: Vec<f64> = (session.lesson.exercises.iter())
        .filter(|laid| laid.exercise.kind.family() == Family::Colour)
        .filter_map(|laid| value(&laid.first.as_ref()?.metrics, Metric::FirstDelta))
        .collect();
    (!picks.is_empty()).then(|| picks.iter().sum::<f64>() / picks.len() as f64)
}

/// The first session's first single line compared, and the latest session's when it is another,
/// each numbered among the sessions of its stage in `all`, its index its sheet's.
fn lines(all: &[Sheet], sessions: &[OnSheet<'_>]) -> Vec<LineShown> {
    let shown = |session: &OnSheet<'_>| {
        let index = (session.lesson.exercises.iter())
            .position(|laid| laid.first.is_some() && laid.exercise.kind.family() == Family::Line)?;
        let index = session.first_exercise() + index;
        Some(LineShown {
            record: session.sheet.id,
            number: number(all, session.sheet, session.index),
            index: u32::try_from(index).unwrap_or(u32::MAX),
        })
    };
    let first = sessions.iter().find_map(shown);
    let latest = sessions.iter().rev().find_map(shown);
    match (first, latest) {
        (Some(first), Some(latest)) if latest != first => vec![first, latest],
        (Some(first), _) => vec![first],
        _ => Vec::new(),
    }
}
