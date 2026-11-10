//! What each exercise of a sheet printed (S120a): its marks before Compare and the picture its
//! kind drew for it, kept once so a sheet shows what it printed whatever became of the generator.
//! Their bytes are practice-store's (`givens/`).

use crate::marks::asked::Painted;
use crate::marks::mark::SheetMark;
use crate::packs::pack::Pack;
use crate::records::sheet::Lesson;
use crate::sessions::plan::Exercise;
use crate::sessions::sheet_picture::SheetPicture;

/// What one exercise printed: its marks before Compare, and the picture its kind drew for it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Given {
    pub marks: Vec<SheetMark>,
    pub painted: Option<Painted>,
}

/// What each exercise of a sheet printed, by its slot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Givens {
    pub slots: Vec<(u16, Given)>,
}

impl Given {
    /// What `exercise` prints now, its picture as `packs` hold it: its kind's generator's.
    #[must_use]
    pub fn printed(exercise: &Exercise, packs: &[Pack]) -> Self {
        let original = exercise.original();
        let picture = (exercise.picture.as_ref()).and_then(|item| SheetPicture::of(item, packs));
        Self {
            marks: exercise.givens(&original),
            painted: exercise.painted(&original, picture.as_ref()),
        }
    }
}

impl Givens {
    /// What each exercise of `lesson` prints now (`Given::printed`), by its slot.
    #[must_use]
    pub fn of_lesson(lesson: &Lesson, packs: &[Pack]) -> Self {
        let slots = (lesson.exercises.iter())
            .map(|laid| (laid.slot, Given::printed(&laid.exercise, packs)))
            .collect();
        Self { slots }
    }

    /// What the exercise in `slot` printed; none for a slot not kept.
    #[must_use]
    pub fn of(&self, slot: u16) -> Option<&Given> {
        (self.slots.iter())
            .find(|(held, _)| *held == slot)
            .map(|(_, given)| given)
    }
}
