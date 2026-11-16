//! What is kept of a practice sheet (S120): the sheet a hidden project of the library holds, its
//! purpose (the day's practice, a test's, or a long colour copy), the spans it was practised in
//! (`time.rs`) and its lessons, each with how it was picked (a skill's lesson, the first lesson, a
//! unit's re-test, or a copy's picture), the exercise in hand and its
//! exercises as laid: each one's place on the sheet (its slot), its picture's size (its frames'
//! shape), its level, its first Compare (when, which strokes after it are corrections, its
//! numbers), how often it was compared, its result and who judged it, and the seconds spent on
//! it. What each exercise printed is kept beside it once (`givens.rs`), so a generator may change.
//! A lesson appended to a sheet is laid from the row after the last one used (`append`).
//!
//! A lesson's number is never stored: it is its place among the lessons (`lessons.rs`).

use crate::course::schedule::Planned;
use crate::kinds::Kind;
use crate::kinds::SkillLevel;
use crate::marks::compared::Outcome;
use crate::packs::pack::{ItemRef, Pack};
use crate::records::givens::Givens;
use crate::records::lessons::OnSheet;
use crate::records::metric::Metrics;
use crate::records::time::Span;
use crate::sheets::exercise::Exercise;
use crate::sheets::layout::Layout;

/// A practice sheet as it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// Its number among all sheets, from 1; never reused.
    pub id: u64,
    /// The project that holds it (the app's project id); 0 until it has one.
    pub canvas: u64,
    /// When it started, in seconds since 1970 UTC.
    pub started: u64,
    pub purpose: Purpose,
    /// The spans it was practised in, oldest first (`time.rs`).
    pub spans: Vec<Span>,
    /// Its lessons, oldest first; the last is the one in hand.
    pub lessons: Vec<Lesson>,
}

/// What a sheet is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The day's practice: its lessons, each from a new row.
    Day,
    /// A project of its own beside the lessons, kept until removed: a long colour copy (V大's
    /// 色彩临摹) or a drawing from life (写生, S151, its exercise's kind telling which). One
    /// exercise, never finished, in no lesson's count.
    Copy,
    /// A test lesson the device chore started (`practice-test`): a day's sheet in all else.
    Test,
}

/// One lesson on a sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Lesson {
    /// When it started and finished, in seconds since 1970 UTC.
    pub started: u64,
    pub finished: Option<u64>,
    pub picked: Picked,
    /// The exercise in hand, an index into `exercises`.
    pub at: u16,
    pub exercises: Vec<Laid>,
}

/// How a lesson was picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    /// One skill's lesson, tapped on the path or offered (S122), a review's; a copy's picture.
    /// Stored as 2.
    Chosen,
    /// The first lesson (S122): a drawing of something in front of the learner (S151), a straight
    /// line, an arc. Stored as 4.
    First,
    /// A unit's re-test (S147, `course/retest.rs`): one exercise of each of its skills, the same
    /// tasks at its start and, once its seal is earned, at its `end`. Stored as 5.
    Retest { unit: u8, end: bool },
}

/// One exercise as it was laid on a sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    pub exercise: Exercise,
    /// Its level within its kind (0: the first).
    pub level: u8,
    /// Its place on the sheet.
    pub slot: u16,
    /// Its picture's size in pixels, its frames' shape (S99); `[0, 0]` square.
    pub frame: [u16; 2],
    /// Its first Compare, once compared.
    pub first: Option<First>,
    /// How often it was compared.
    pub compares: u32,
    /// What it ended with and who judged it.
    pub result: Option<(Outcome, Judge)>,
    /// The seconds practised while it was in hand.
    pub seconds: u32,
}

/// An exercise's first Compare.
#[derive(Debug, Clone, PartialEq)]
pub struct First {
    /// When, in seconds since 1970 UTC.
    pub at: u64,
    /// The sheet's first stroke id after it: strokes begun in its frame from it on are
    /// corrections.
    pub corrected_from: u64,
    /// Its numbers (a colour's, a combination's, a scale's, an estimate's); none for a line.
    pub metrics: Metrics,
}

/// Who judged a result (S119).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judge {
    App,
    Eye,
}

impl First {
    /// A first Compare at `at`, the sheet's next stroke id then `next_stroke`, with `metrics`.
    #[must_use]
    pub const fn of(at: u64, next_stroke: u64, metrics: Metrics) -> Self {
        Self {
            at,
            corrected_from: next_stroke,
            metrics,
        }
    }

    /// Whether the stroke `id` begun in its frame is a correction.
    #[must_use]
    pub fn corrects(&self, id: u64) -> bool {
        id >= self.corrected_from
    }
}

impl Laid {
    /// `exercise` laid in `slot`, its frames square until `Sheet::framed`.
    #[must_use]
    pub fn new(exercise: Exercise, slot: usize) -> Self {
        Self {
            exercise,
            level: 0,
            slot: u16::try_from(slot).unwrap_or(u16::MAX),
            frame: [0, 0],
            first: None,
            compares: 0,
            result: None,
            seconds: 0,
        }
    }

    /// It compared for the first time: `first`, counted once.
    pub fn compared(&mut self, first: First) {
        self.first = Some(first);
        self.compares = 1;
    }

    /// The user chose `outcome` for it after Compare (S122), where the eye judges
    /// (`Verdict::Eye`): kept with `Judge::Eye`, the first choice only, never over the app's.
    pub fn judged(&mut self, outcome: Outcome) {
        if self.result.is_none() {
            self.result = Some((outcome, Judge::Eye));
        }
    }
}

impl Sheet {
    /// A long colour copy of `picture`, the `id`th of all sheets, seeded `seed`, begun at
    /// `started`.
    #[must_use]
    pub fn copy(id: u64, seeded: (u64, u64), picture: ItemRef) -> Self {
        Self::project(Kind::Study, id, seeded, picture)
    }

    /// A drawing from life against `photo` (S151, one of the user's own pictures), the `id`th of
    /// all sheets, seeded `seed`, begun at `started`: a project as a colour copy is.
    #[must_use]
    pub fn from_life(id: u64, seeded: (u64, u64), photo: ItemRef) -> Self {
        Self::project(Kind::Observation, id, seeded, photo)
    }

    /// A project of `kind` of `picture`: one exercise on a sheet of its own.
    fn project(kind: Kind, id: u64, (seed, started): (u64, u64), picture: ItemRef) -> Self {
        let exercise = Exercise {
            picture: Some(picture),
            ..Exercise::generated(kind, seed)
        };
        Self::laid_out(id, (Purpose::Copy, started), Picked::Chosen, vec![exercise])
    }

    /// A lesson as planned (S122, `schedule`), the `id`th of all sheets, on a sheet for `purpose`
    /// started at `started`, each exercise at the level it was planned at.
    #[must_use]
    pub fn begun(id: u64, (purpose, started): (Purpose, u64), planned: Planned) -> Self {
        let (exercises, levels): (Vec<Exercise>, Vec<SkillLevel>) =
            planned.exercises.into_iter().unzip();
        let mut sheet = Self::laid_out(id, (purpose, started), planned.picked, exercises);
        let laid = (sheet.lessons.iter_mut()).flat_map(|lesson| lesson.exercises.iter_mut());
        for (laid, level) in laid.zip(levels) {
            laid.level = level.code();
        }
        sheet
    }

    fn laid_out(
        id: u64,
        (purpose, started): (Purpose, u64),
        picked: Picked,
        exercises: Vec<Exercise>,
    ) -> Self {
        let exercises = (exercises.into_iter().enumerate())
            .map(|(slot, exercise)| Laid::new(exercise, slot))
            .collect();
        Self {
            id,
            canvas: 0,
            started,
            purpose,
            spans: Vec::new(),
            lessons: vec![Lesson {
                started,
                finished: None,
                picked,
                at: 0,
                exercises,
            }],
        }
    }

    /// It with each exercise's frames the shape of its picture as `packs` hold it (S99); one no
    /// pack holds stays square.
    #[must_use]
    pub fn framed(mut self, packs: &[Pack]) -> Self {
        for laid in self.lessons.iter_mut().flat_map(|l| l.exercises.iter_mut()) {
            let item = (laid.exercise.picture.as_ref())
                .and_then(|item| packs.iter().find_map(|pack| pack.find(item)));
            if let Some(item) = item {
                laid.frame = item
                    .size
                    .map(|side| u16::try_from(side).unwrap_or(u16::MAX));
            }
        }
        self
    }

    /// The index of the lesson in hand: the last unfinished one, else the last.
    #[must_use]
    pub fn in_hand(&self) -> usize {
        (self
            .lessons
            .iter()
            .rposition(|lesson| lesson.finished.is_none()))
        .unwrap_or_else(|| self.lessons.len().saturating_sub(1))
    }

    /// The lesson in hand (`in_hand`).
    #[must_use]
    pub fn lesson(&self) -> Option<&Lesson> {
        self.lessons.get(self.in_hand())
    }

    pub fn lesson_mut(&mut self) -> Option<&mut Lesson> {
        let index = self.in_hand();
        self.lessons.get_mut(index)
    }

    /// The lesson in hand as the counts walk it.
    #[must_use]
    pub fn held(&self) -> Option<OnSheet<'_>> {
        OnSheet::of(self, self.in_hand())
    }

    /// `lesson` appended as the lesson in hand, laid from the row after the last one used, and
    /// `givens`, what it prints, moved with it: returns them.
    pub fn append(&mut self, mut lesson: Lesson, mut givens: Givens) -> Givens {
        let shift = u16::try_from(Layout::next_slot(self)).unwrap_or(u16::MAX);
        for laid in &mut lesson.exercises {
            laid.slot = laid.slot.saturating_add(shift);
        }
        for (slot, _) in &mut givens.slots {
            *slot = slot.saturating_add(shift);
        }
        self.lessons.push(lesson);
        givens
    }

    /// Every exercise laid on it, its lessons' in order.
    pub fn exercises(&self) -> impl Iterator<Item = &Laid> {
        self.lessons
            .iter()
            .flat_map(|lesson| lesson.exercises.iter())
    }

    /// Exercise `index` of `exercises`.
    #[must_use]
    pub fn laid(&self, index: usize) -> Option<&Laid> {
        self.exercises().nth(index)
    }

    pub fn laid_mut(&mut self, index: usize) -> Option<&mut Laid> {
        (self.lessons.iter_mut())
            .flat_map(|lesson| lesson.exercises.iter_mut())
            .nth(index)
    }

    /// Exercise `index`'s first Compare, once compared.
    #[must_use]
    pub fn first(&self, index: usize) -> Option<&First> {
        self.laid(index)?.first.as_ref()
    }

    /// The exercise in hand, an index into `exercises`.
    #[must_use]
    pub fn at(&self) -> usize {
        let held = self.held();
        held.map_or(0, |held| {
            held.first_exercise() + usize::from(held.lesson.at)
        })
    }

    /// When the lesson in hand finished.
    #[must_use]
    pub fn finished(&self) -> Option<u64> {
        self.lesson()?.finished
    }

    /// Whether it is a project of its own: a long colour copy or a drawing from life.
    #[must_use]
    pub fn is_copy(&self) -> bool {
        self.purpose == Purpose::Copy
    }

    /// Whether it is a drawing from life (S151): a project whose picture, the photo, stays out of
    /// sight until compared (`Facts::hidden`).
    #[must_use]
    pub fn drawn_from_life(&self) -> bool {
        let hidden =
            (self.exercises().next()).is_some_and(|laid| laid.exercise.kind.facts().hidden);
        self.is_copy() && hidden
    }
}
