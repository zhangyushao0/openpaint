//! What each exercise of a sheet printed (S120a): its marks before Compare and the picture its
//! kind drew for it, kept once so a sheet shows what it printed whatever became of the generator.
//! Their bytes are practice-store's (`givens/`).

use crate::marks::asked::Painted;
use crate::marks::mark::SheetMark;

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

impl Givens {
    /// What the exercise in `slot` printed; none for a slot not kept.
    #[must_use]
    pub fn of(&self, slot: u16) -> Option<&Given> {
        (self.slots.iter())
            .find(|(held, _)| *held == slot)
            .map(|(_, given)| given)
    }
}
