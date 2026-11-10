//! Practice counted by lesson (S121b): a sheet may hold several lessons (the day's sheet), so
//! whatever counts sessions, drills or copies walks every lesson of every sheet in the order of
//! (sheet id, lesson index), each told by its own lesson: a stage's session, a drill of a kind, a
//! long colour copy. A lesson's number among its kind is its place among the lessons alike; the
//! lesson in hand is a sheet's last unfinished one, else its last.

use crate::records::sheet::{Lesson, Picked, Sheet};
use crate::sessions::plan::Family;

impl Lesson {
    /// The stage of a stage's session; none for a drill or a copy.
    #[must_use]
    pub fn stage(&self) -> Option<u8> {
        match self.picked {
            Picked::Full { stage, .. } => Some(stage),
            Picked::Chosen => None,
        }
    }

    /// The kind a chosen lesson holds (its first exercise's): a drill's, a copy's study; none for
    /// a session.
    #[must_use]
    pub fn chosen(&self) -> Option<Family> {
        let first = self
            .exercises
            .first()
            .filter(|_| self.picked == Picked::Chosen)?;
        Some(first.exercise.kind.family())
    }

    /// How many of its first exercises are its warm-up.
    #[must_use]
    pub fn warm_up(&self) -> usize {
        match self.picked {
            Picked::Full { warm_up, .. } => usize::from(warm_up),
            Picked::Chosen => 0,
        }
    }
}

/// One lesson on its sheet, as the counts walk them.
#[derive(Debug, Clone, Copy)]
pub struct OnSheet<'a> {
    pub sheet: &'a Sheet,
    /// Its index among the sheet's lessons.
    pub index: usize,
    pub lesson: &'a Lesson,
}

impl<'a> OnSheet<'a> {
    /// Lesson `index` of `sheet`, if it holds one.
    #[must_use]
    pub fn of(sheet: &'a Sheet, index: usize) -> Option<Self> {
        let lesson = sheet.lessons.get(index)?;
        Some(Self {
            sheet,
            index,
            lesson,
        })
    }

    /// The stage of a stage's session; none for a drill or a copy.
    #[must_use]
    pub fn stage(&self) -> Option<u8> {
        self.lesson.stage()
    }

    /// The kind a drill drills; none for a session or a copy.
    #[must_use]
    pub fn drilled(&self) -> Option<Family> {
        self.lesson.chosen().filter(|_| !self.sheet.is_copy())
    }

    /// When it finished.
    #[must_use]
    pub fn finished(&self) -> Option<u64> {
        self.lesson.finished
    }

    /// Whether it is a long colour copy.
    #[must_use]
    pub fn is_copy(&self) -> bool {
        self.sheet.is_copy()
    }

    /// Its place in the order every count walks: its sheet's id, then its index.
    #[must_use]
    pub fn order(&self) -> (u64, usize) {
        (self.sheet.id, self.index)
    }

    /// The index among its sheet's exercises of its first exercise.
    #[must_use]
    pub fn first_exercise(&self) -> usize {
        (self.sheet.lessons[..self.index].iter())
            .map(|lesson| lesson.exercises.len())
            .sum()
    }

    /// Whether `other` is a lesson of its kind: a session of its stage, a drill of its kind, a
    /// copy.
    fn alike(&self, other: &Self) -> bool {
        self.is_copy() == other.is_copy()
            && self.stage() == other.stage()
            && self.drilled() == other.drilled()
    }
}

/// Every lesson of `sheets`, in the order of (sheet id, lesson index).
pub fn lessons<'a>(sheets: impl IntoIterator<Item = &'a Sheet>) -> Vec<OnSheet<'a>> {
    let mut all: Vec<OnSheet<'a>> = (sheets.into_iter())
        .flat_map(|sheet| (0..sheet.lessons.len()).filter_map(|index| OnSheet::of(sheet, index)))
        .collect();
    all.sort_by_key(OnSheet::order);
    all
}

/// Lesson `lesson` of `sheet`'s number among the lessons of its kind in `all` (`sheet` among them
/// or not), from 1: a session's in its stage, a drill's among its kind's drills, a copy's among
/// the copies.
#[must_use]
pub fn number(all: &[Sheet], sheet: &Sheet, lesson: usize) -> u32 {
    let Some(this) = OnSheet::of(sheet, lesson) else {
        return 1;
    };
    let earlier = (lessons(all).iter())
        .filter(|other| other.order() < this.order() && this.alike(other))
        .count();
    u32::try_from(earlier + 1).unwrap_or(u32::MAX)
}
