//! The practice book (S89, S122) as pure math, all of it from the records: the lessons finished,
//! the days practised by the device's local day with their time and their records, the lessons
//! whose first single line shows side by side (the first and the latest), the drawings from life
//! after the first lesson's drawing (S151: its "before", then the first and the latest), the seals
//! earned, and how much was done in all. Progress counts what was done, never days in a row; each
//! skill's curve is `progress/` (S147, the colour's first picks' line before it). Lessons are
//! every sheet's but a project's (`lessons.rs`); days and time are the sheets', each day with the
//! Pomodoro's blocks completed on it (S124).

use crate::kinds::Kept;
use crate::records::blocks::Block;
use crate::records::lessons::{OnSheet, lessons, number};
use crate::records::seals::{Earned, earned};
use crate::records::sheet::{Laid, Picked, Sheet};
use crate::records::time::{by_day, seconds};

/// The practice book's contents.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Book {
    /// Lessons finished (a long colour copy's never), and the time practised in lessons and
    /// copies, in seconds.
    pub lessons: u32,
    pub seconds: u64,
    /// Each local day practised, oldest first.
    pub days: Vec<Day>,
    /// The first lesson with a single line compared, and the latest such one when it is another.
    pub lines: Vec<LineShown>,
    /// Once a drawing from life was begun (S151): the first lesson's drawing of what lay in front
    /// of the learner, its "before", where there is one, then the first drawing from life and the
    /// latest when it is another.
    pub life: Vec<LineShown>,
    /// The seals earned, in the order they were.
    pub seals: Vec<Earned>,
}

/// A local day practised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// Days since 1970-01-01 on the device's clock (`time::local_day`).
    pub day: i64,
    /// The time practised on it, in seconds.
    pub seconds: u64,
    /// The sheets and copies practised on it, by id, oldest first.
    pub records: Vec<u64>,
    /// The Pomodoro's blocks completed on it (S124, `Blocks::on`): its red seals beside its time.
    /// A day with a block is a day practised, its time or none.
    pub blocks: u32,
}

/// A lesson's first single line compared, shown in the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineShown {
    pub record: u64,
    /// The lesson's number (`lessons::number`).
    pub number: u32,
    /// The exercise, its index on its sheet.
    pub index: u32,
}

impl Book {
    /// The book of `records` and the Pomodoro's blocks completed (`blocks`, S124: each day's
    /// count, a day with a block and no time among the days), its days on a clock `offset`
    /// minutes from UTC; a block counts on the local day it started (`Block::day`).
    #[must_use]
    pub fn of(records: &[Sheet], blocks: &[Block], offset: i32) -> Self {
        let mut ordered: Vec<&Sheet> = records.iter().collect();
        ordered.sort_by_key(|record| record.id);
        let taken: Vec<OnSheet<'_>> = (lessons(records).into_iter())
            .filter(|lesson| !lesson.is_copy())
            .collect();
        let finished = (taken.iter())
            .filter(|lesson| lesson.finished().is_some())
            .count();
        Self {
            lessons: u32::try_from(finished).unwrap_or(u32::MAX),
            seconds: ordered.iter().map(|record| seconds(&record.spans)).sum(),
            days: sealed(days(&ordered, offset), blocks),
            lines: lines(records, &taken),
            life: life(records),
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
                    blocks: 0,
                }),
            }
        }
    }
    days.sort_by_key(|day| day.day);
    days
}

/// `days` with each block of `blocks` counted on the day it started; a day with a block and no
/// time joins them.
fn sealed(mut days: Vec<Day>, blocks: &[Block]) -> Vec<Day> {
    for block in blocks {
        match days.iter_mut().find(|kept| kept.day == block.day) {
            Some(kept) => kept.blocks += 1,
            None => days.push(Day {
                day: block.day,
                seconds: 0,
                records: Vec::new(),
                blocks: 1,
            }),
        }
    }
    days.sort_by_key(|day| day.day);
    days
}

/// The first lesson's first single line compared, and the latest lesson's when it is another,
/// each numbered among the lessons of `all`, its index its sheet's.
fn lines(all: &[Sheet], taken: &[OnSheet<'_>]) -> Vec<LineShown> {
    let line = |laid: &Laid| laid.first.is_some() && laid.exercise.kind.facts().kept == Kept::Line;
    let shown = |lesson: &OnSheet<'_>| kept_in(all, lesson, line);
    first_and_latest(
        taken.iter().find_map(shown),
        taken.iter().rev().find_map(shown),
    )
}

/// The first lesson's drawing (its "before", where the first lesson was taken) and the first and
/// latest drawings from life of `all`, each numbered among its own; none until a drawing from life
/// was begun (S151).
fn life(all: &[Sheet]) -> Vec<LineShown> {
    let every = lessons(all);
    let drawing = |laid: &Laid| laid.exercise.kind.facts().kept == Kept::Drawing;
    let drawn = |lesson: &&OnSheet<'_>| lesson.sheet.drawn_from_life() && lesson.sheet.canvas != 0;
    let first = every
        .iter()
        .filter(drawn)
        .find_map(|on| kept_in(all, on, drawing));
    let latest = (every.iter().rev().filter(drawn)).find_map(|on| kept_in(all, on, drawing));
    let Some(first) = first else {
        return Vec::new();
    };
    let before = (every.iter())
        .filter(|on| on.lesson.picked == Picked::First)
        .find_map(|on| kept_in(all, on, drawing));
    before
        .into_iter()
        .chain(first_and_latest(Some(first), latest))
        .collect()
}

/// `lesson`'s first exercise `kept` keeps, numbered among the lessons alike of `all`.
fn kept_in(all: &[Sheet], lesson: &OnSheet<'_>, kept: impl Fn(&Laid) -> bool) -> Option<LineShown> {
    let index = lesson.lesson.exercises.iter().position(kept)?;
    let index = lesson.first_exercise() + index;
    Some(LineShown {
        record: lesson.sheet.id,
        number: number(all, lesson.sheet, lesson.index),
        index: u32::try_from(index).unwrap_or(u32::MAX),
    })
}

/// `first`, and `latest` after it when it is another.
fn first_and_latest(first: Option<LineShown>, latest: Option<LineShown>) -> Vec<LineShown> {
    match (first, latest) {
        (Some(first), Some(latest)) if latest != first => vec![first, latest],
        (Some(first), _) => vec![first],
        _ => Vec::new(),
    }
}
