//! Practice counted by lesson (S121b, S122): a sheet may hold several lessons (the day's sheet),
//! so whatever counts walks every lesson of every sheet in the order of (sheet id, lesson index),
//! each told by its own lesson: a skill's, the first one, a re-test, a long colour copy.
//! A lesson's number is its place among the learner's lessons (a copy's among the copies); the
//! lesson in hand is a sheet's last unfinished one, else its last.

use crate::kinds::Skill;
use crate::records::sheet::{Lesson, Picked, Sheet};

impl Lesson {
    /// A re-test's unit and whether it is the unit's end (S147); none for any other lesson.
    #[must_use]
    pub const fn retest(&self) -> Option<(u8, bool)> {
        match self.picked {
            Picked::Retest { unit, end } => Some((unit, end)),
            _ => None,
        }
    }

    /// The skill a skill's lesson practises (its first exercise's, S122); none for the first
    /// lesson, a re-test or a copy.
    #[must_use]
    pub fn skill(&self) -> Option<Skill> {
        let first = self
            .exercises
            .first()
            .filter(|_| self.picked == Picked::Chosen)?;
        Skill::of(first.exercise.kind)
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

    /// When it finished.
    #[must_use]
    pub fn finished(&self) -> Option<u64> {
        self.lesson.finished
    }

    /// Whether it is a project's: a long colour copy's or a drawing from life's.
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
}

/// Every lesson of `sheets`, in the order of (sheet id, lesson index).
pub fn lessons<'a>(sheets: impl IntoIterator<Item = &'a Sheet>) -> Vec<OnSheet<'a>> {
    let mut all: Vec<OnSheet<'a>> = (sheets.into_iter())
        .flat_map(|sheet| (0..sheet.lessons.len()).filter_map(|index| OnSheet::of(sheet, index)))
        .collect();
    all.sort_by_key(OnSheet::order);
    all
}

/// Lesson `lesson` of `sheet`'s number in `all` (`sheet` among them or not), from 1: its place
/// among every lesson but a copy's (the learner's lessons: a row's label on the sheet, "lesson n ·
/// skill"), a project's among the projects of its sort (`Sheet::project_sort`): a colour copy's
/// among the colour copies, a drawing from life's among the drawings (S151), a cold-warm copy's
/// among the cold-warm copies (S154).
#[must_use]
pub fn number(all: &[Sheet], sheet: &Sheet, lesson: usize) -> u32 {
    let Some(this) = OnSheet::of(sheet, lesson) else {
        return 1;
    };
    let alike = |other: &OnSheet<'_>| other.sheet.project_sort() == this.sheet.project_sort();
    let earlier = (lessons(all).iter())
        .filter(|other| other.order() < this.order() && alike(other))
        .count();
    u32::try_from(earlier + 1).unwrap_or(u32::MAX)
}
