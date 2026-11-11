//! A lesson as the screen shows it (S122): each lesson's label over its first row of the day's
//! sheet ("lesson n · skill"), the lesson in hand as the bar names it ("skill 3/5", "First lesson
//! 2/3"), the result an exercise ends with (the app's verdict, or the three the user chooses from
//! where the eye judges) and the lesson's end (each exercise's result, the skill's dots, the seals
//! earned, the lesson One more starts). What the flow fills them with is `sheet/lesson.rs`.

use openpaint_practice::{Judge, Offer, Outcome, Seal, Sheet, Skill, SkillLevel, Verdict};

/// A lesson on the sheet as its row's label names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LessonRow {
    /// The slot of its first exercise: the row it starts.
    pub slot: u16,
    /// Its number among the learner's lessons, from 1, as `openpaint_practice::number` counts
    /// them (S121b; S122a: every lesson but a copy's counted together).
    pub number: u32,
    /// The skill a skill's lesson or a review practises; none in a full lesson and the first.
    pub skill: Option<Skill>,
    /// A full lesson's unit.
    pub unit: Option<u8>,
    /// It is the first lesson.
    pub first: bool,
}

impl LessonRow {
    /// Every lesson of `sheet` in order, lesson `index` numbered `number(index)`; none on a long
    /// colour copy's sheet.
    #[must_use]
    pub fn of_sheet(sheet: &Sheet, number: impl Fn(usize) -> u32) -> Vec<Self> {
        if sheet.is_copy() {
            return Vec::new();
        }
        (sheet.lessons.iter().enumerate())
            .map(|(index, lesson)| Self {
                slot: lesson.exercises.first().map_or(0, |laid| laid.slot),
                number: number(index),
                skill: lesson.skill(),
                unit: lesson.unit(),
                first: lesson.picked == openpaint_practice::Picked::First,
            })
            .collect()
    }
}

/// The lesson in hand as the bar names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LessonShown {
    /// Its row's label.
    pub row: LessonRow,
    /// The level the exercise shown was laid at.
    pub level: SkillLevel,
    /// The exercise shown's place in its lesson, from 1, and how many the lesson holds; once it
    /// is done, its last.
    pub place: (usize, usize),
    /// The exercise shown is part of a full lesson's warm-up.
    pub warming_up: bool,
}

/// What the shown comparison says of the exercise's result: who judges it and what is kept.
/// `Verdict::App`: `result` holds the app's outcome, shown ringed among the three.
/// `Verdict::Eye` with no result yet: the three show as buttons (`PracticeIntent::Judge`); with
/// one, the user's choice shows ringed. `Verdict::Incomplete`: nothing to show yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Judged {
    pub verdict: Verdict,
    pub result: Option<(Outcome, Judge)>,
}

/// A lesson's end, once its last exercise was left with Next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct End {
    /// Each exercise of the lesson, by its index among the sheet's, and what it ended with (none:
    /// not judged): a small mark on its frames.
    pub results: Vec<(usize, Option<Outcome>)>,
    /// A skill's lesson: its skill, its dots before the lesson and after it (a level reached
    /// where they differ).
    pub skill: Option<(Skill, u8, u8)>,
    /// The seals the lesson earned.
    pub sealed: Vec<Seal>,
    /// The lesson One more starts, which its word names; none while nothing can be practised.
    pub next: Option<Offer>,
}
