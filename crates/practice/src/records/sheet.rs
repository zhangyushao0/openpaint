//! What is kept of a practice sheet (S120): the sheet a hidden project of the library holds, its
//! purpose (the day's practice, a test's, or a long colour copy), the spans it was practised in
//! (`time.rs`)
//! and its lessons, each with how it was picked (a stage's session with its warm-up, or a kind
//! chosen: a drill, a copy's picture), the exercise in hand and its exercises as laid: each one's
//! place on the sheet (its slot), its picture's size (its frames' shape), its level, its first
//! Compare (when, which strokes after it are corrections, its numbers), how often it was compared,
//! its result and who judged it, and the seconds spent on it. What each exercise printed is kept
//! beside it once (`givens.rs`), so a generator may change. A lesson appended to a sheet is laid
//! from the row after the last one used (`append`).
//!
//! A lesson's number among its kind (a stage's sessions, a kind's drills, the copies) is never
//! stored: it is its place among the lessons alike (`lessons.rs`).

use crate::course::schedule::Planned;
use crate::kinds::SkillLevel;
use crate::marks::compared::Outcome;
use crate::packs::pack::{ItemRef, Pack};
use crate::records::givens::Givens;
use crate::records::lessons::OnSheet;
use crate::records::metric::Metrics;
use crate::records::time::Span;
use crate::sessions::layout::Layout;
use crate::sessions::plan::{Exercise, Family, Kind};
use crate::sessions::session::Plan;

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
    /// The day's practice: a stage's session or a drill.
    Day,
    /// A long colour copy (V大's 色彩临摹): one exercise, never finished, in no stage or summary.
    Copy,
    /// A test session or drill the device chore started (`practice-test`): a day's sheet in all
    /// else.
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
    /// One skill's lesson, tapped on the path or offered (S122), a review's; a drill before it; a
    /// copy's picture. Stored as 2.
    Chosen,
    /// A full lesson of unit `stage` (S122; a stage's session before it), its first `warm_up`
    /// exercises the warm-up. Stored as 3.
    Full { stage: u8, warm_up: u8 },
    /// The first lesson (S122): a free scribble, a straight line, an arc. Stored as 4.
    First,
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
    /// corrections; `None` in 1.1.0's records (a compared frame took nothing more).
    pub corrected_from: Option<u64>,
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
            corrected_from: Some(next_stroke),
            metrics,
        }
    }

    /// Whether the stroke `id` begun in its frame is a correction.
    #[must_use]
    pub fn corrects(&self, id: u64) -> bool {
        self.corrected_from.is_some_and(|from| id >= from)
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
    /// A stage's session, the `id`th of all sheets, started at `started`, holding `plan`.
    #[must_use]
    pub fn session(id: u64, stage: u8, started: u64, plan: Plan) -> Self {
        let warm_up = u8::try_from(plan.warm_up).unwrap_or(u8::MAX);
        let picked = Picked::Full { stage, warm_up };
        Self::laid_out(id, (Purpose::Day, started), picked, plan.exercises)
    }

    /// A drill (S91), the `id`th of all sheets, begun at `started`, holding `plan`.
    #[must_use]
    pub fn drill(id: u64, started: u64, plan: Plan) -> Self {
        Self::laid_out(id, (Purpose::Day, started), Picked::Chosen, plan.exercises)
    }

    /// A long colour copy of `picture`, the `id`th of all sheets, seeded `seed`, begun at
    /// `started`.
    #[must_use]
    pub fn copy(id: u64, (seed, started): (u64, u64), picture: ItemRef) -> Self {
        let copy = Exercise {
            picture: Some(picture),
            ..Exercise::generated(Kind::Study, seed)
        };
        Self::laid_out(id, (Purpose::Copy, started), Picked::Chosen, vec![copy])
    }

    /// A lesson as planned (S122, `schedule`), the `id`th of all sheets, on a sheet for `purpose`
    /// started at `started`, each exercise at the level it was planned at. Takes the place of
    /// `session` and `drill`, which go with S122a.
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

    /// The stage of the session in hand; none for a drill or a copy.
    #[must_use]
    pub fn stage(&self) -> Option<u8> {
        self.lesson()?.stage()
    }

    /// The kind the drill in hand drills (its exercises'); none for a session or a copy.
    #[must_use]
    pub fn drilled(&self) -> Option<Family> {
        self.held()?.drilled()
    }

    /// Whether it is a long colour copy.
    #[must_use]
    pub fn is_copy(&self) -> bool {
        self.purpose == Purpose::Copy
    }

    /// How many of the lesson in hand's first exercises are its warm-up.
    #[must_use]
    pub fn warm_up(&self) -> usize {
        self.lesson().map_or(0, Lesson::warm_up)
    }

    /// The lesson holding exercise `index`, and the exercise's index in it.
    fn locate(&self, mut index: usize) -> Option<(&Lesson, usize)> {
        for lesson in &self.lessons {
            if index < lesson.exercises.len() {
                return Some((lesson, index));
            }
            index -= lesson.exercises.len();
        }
        None
    }

    /// Whether exercise `index` is part of its lesson's warm-up.
    #[must_use]
    pub fn warming_up(&self, index: usize) -> bool {
        self.locate(index)
            .is_some_and(|(lesson, at)| at < lesson.warm_up())
    }

    /// Exercise `index`'s place, from 1, and how many there are: in the warm-up among the
    /// warm-up's exercises, after it among those of its family after it, in its lesson.
    #[must_use]
    pub fn place(&self, index: usize) -> (usize, usize) {
        let Some((lesson, at)) = self.locate(index) else {
            return (0, 0);
        };
        let exercises = &lesson.exercises;
        let warm_up = lesson.warm_up().min(exercises.len());
        if at < warm_up {
            return (at + 1, warm_up);
        }
        let family = exercises[at].exercise.kind.family();
        let alike = |l: &&Laid| l.exercise.kind.family() == family;
        let before = exercises[warm_up..at].iter().filter(alike).count();
        let all = exercises[warm_up..].iter().filter(alike).count();
        (before + 1, all)
    }
}
