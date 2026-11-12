//! What the sheets kept tell of the pictures shown (`sheet.rs`), every lesson of each counted:
//! those every lesson showed (what the next one picks others than) and those the last few showed
//! (what a composition is picked others than). Long colour copies count in none. The learner
//! (`course/learner.rs`) keeps the same as it folds the lessons.

use crate::packs::pack::ItemRef;
use crate::records::lessons::lessons;
use crate::records::sheet::Sheet;

/// The pictures `sheets`' lessons showed (long colour copies left out), each once: what the next
/// lesson picks others than.
#[must_use]
pub fn used(sheets: &[Sheet]) -> Vec<ItemRef> {
    let mut items: Vec<ItemRef> = (sheets.iter())
        .filter(|sheet| !sheet.is_copy())
        .flat_map(Sheet::exercises)
        .filter_map(|laid| laid.exercise.picture.clone())
        .collect();
    items.sort();
    items.dedup();
    items
}

/// The lessons a composition waits before it comes back (S95).
pub const RECENT: usize = 10;

/// The pictures the last `RECENT` lessons of `sheets` (colour copies left out) showed: what a
/// division or a whole copy picks others than while there are.
#[must_use]
pub fn recent(sheets: &[Sheet]) -> Vec<ItemRef> {
    let mut items: Vec<ItemRef> = (lessons(sheets).iter().rev())
        .filter(|lesson| !lesson.is_copy())
        .take(RECENT)
        .flat_map(|lesson| lesson.lesson.exercises.iter())
        .filter_map(|laid| laid.exercise.picture.clone())
        .collect();
    items.sort();
    items.dedup();
    items
}
