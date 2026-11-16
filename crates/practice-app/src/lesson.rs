//! A lesson as the screen shows it (S122): each lesson's label over its first row of the day's
//! sheet ("lesson n · skill"), the lesson in hand as the bar names it ("skill 3/5", "First lesson
//! 2/3"), the result an exercise ends with (the app's verdict, or the three the user chooses from
//! where the eye judges) and the lesson's end (each exercise's result, the skill's dots, the seals
//! earned, the lesson One more starts or a due start re-test (S148), its blind pairs and extra
//! drill, S147). What the flow fills them with is `sheet/lesson.rs` and `sheet/pairs.rs`.

use openpaint_practice::{
    Answer, CopyRef, Judge, Offer, Outcome, PairAsked, Retest, Seal, Sheet, Skill, SkillLevel,
    Verdict,
};

use crate::path::Start;

/// A lesson on the sheet as its row's label names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LessonRow {
    /// The slot of its first exercise: the row it starts.
    pub slot: u16,
    /// Its number among the learner's lessons, from 1, as `openpaint_practice::number` counts
    /// them (S121b; S122a: every lesson but a copy's counted together; `PracticeLog::rows`).
    pub number: u32,
    /// The skill a skill's lesson or a review practises; none in the first and a re-test.
    pub skill: Option<Skill>,
    /// It is the first lesson.
    pub first: bool,
    /// A re-test's unit and whether it is the unit's end (S147).
    pub retest: Option<(u8, bool)>,
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
                first: lesson.picked == openpaint_practice::Picked::First,
                retest: lesson.retest(),
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
    /// The exercise shown's place among all of the lesson, from 1, and how many there are. Once
    /// it is done, its last's.
    pub place: (usize, usize),
}

/// What the shown comparison says of the exercise's result: who judges it and what is kept.
/// `Verdict::App`: `result` holds the app's outcome, shown ringed among the three.
/// `Verdict::Eye` with no result yet: the three show as buttons (`PracticeIntent::Judge`); with
/// one, the user's choice shows ringed. `Verdict::Incomplete`: nothing to show yet;
/// `Verdict::Nobody`: nothing ever (a project: a drawing from life, a cold-warm or long colour
/// copy; S151, S154, S158). Never on the first lesson, where
/// nothing is scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Judged {
    pub verdict: Verdict,
    pub result: Option<(Outcome, Judge)>,
}

/// A lesson's end, once its last exercise was left with Next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct End {
    /// Each exercise of the lesson, by its index among the sheet's, and what it ended with (none:
    /// not judged): a small mark on its frames. None at all on the first lesson, where nothing is
    /// scored.
    pub results: Vec<(usize, Option<Outcome>)>,
    /// How many exercises the lesson held, and the seconds practised on them.
    pub exercises: u8,
    pub seconds: u32,
    /// A skill's lesson: its skill, its dots before the lesson and after it (a level reached
    /// where they differ).
    pub skill: Option<(Skill, u8, u8)>,
    /// The seals the lesson earned.
    pub sealed: Vec<Seal>,
    /// The lesson One more starts, which its word names; none while nothing can be practised.
    pub next: Option<OneMore>,
    /// The blind pairs asked where the learner pauses (S147, S148): at this end outside a
    /// Pomodoro's block (a skill's lesson's one, a unit's end re-test's one a skill; a finished
    /// lesson opened again its answered ones), or at the block's end that ended it; each with its
    /// answer once given.
    pub pairs: Vec<PairShown>,
    /// An extra drill of its skill (S147): offered when its curve is sure the skill fell.
    pub drill: Option<Offer>,
}

/// What One more starts after a lesson (and a Pomodoro's chain lays): a skill's lesson, or a
/// unit's start re-test while it is due, as Start would lay it (S148).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OneMore {
    Skill(Offer),
    Retest(Retest),
}

impl OneMore {
    /// What starts it.
    #[must_use]
    pub const fn start(&self) -> Start {
        match self {
            Self::Skill(offer) => Start::Skill(offer.skill),
            Self::Retest(retest) => Start::Retest(retest.unit, retest.end),
        }
    }
}

/// A blind pair as an end shows it: its two copies, which shows on the left, the answer once
/// given (then the dates show) and the skill its copies practise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairShown {
    pub asked: PairAsked,
    pub older_left: bool,
    pub answer: Option<Answer>,
    pub skill: Option<Skill>,
}

impl PairShown {
    /// Its copies left and right.
    #[must_use]
    pub const fn sides(&self) -> [CopyRef; 2] {
        if self.older_left {
            [self.asked.older, self.asked.newer]
        } else {
            [self.asked.newer, self.asked.older]
        }
    }
}
