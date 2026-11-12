//! The long colour copies in the log: one begun of a picture chosen, numbered among the copies and
//! titled "Colour copy n" until renamed; the test copy's picture; the copies as the options list
//! them.

use openpaint_practice::{ItemRef, PictureKind, Random, Sheet, lessons, number};

use super::PracticeLog;
use crate::copies::{CopyShown, PictureShown};
use openpaint_app::{ProjectId, Title};

impl PracticeLog {
    /// A new colour copy of `picture` (the user chose it; none unless a pack kept holds it),
    /// seeded `seed`, begun at `now`, with its sheet's title.
    #[must_use]
    pub fn begin_copy(&self, picture: &ItemRef, seed: u64, now: u64) -> Option<(Sheet, Title)> {
        let found = self.packs.iter().find_map(|pack| pack.find(picture))?;
        if found.kind != PictureKind::Shape {
            return None;
        }
        let number = lessons(&self.sheets).iter().filter(|s| s.is_copy()).count() + 1;
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let record = Sheet::copy(self.next_id(), (seed, now), picture.clone()).framed(&self.packs);
        Some((record, copy_title(number)))
    }

    /// An official pack's picture not copied before (any once each was), picked by `seed`: the
    /// test colour copy's (`practice-test start copy`).
    #[must_use]
    pub fn fresh_picture(&self, seed: u64) -> Option<ItemRef> {
        let official: Vec<&PictureShown> = (self.pictures.iter())
            .filter(|picture| !picture.personal)
            .collect();
        let fresh: Vec<&PictureShown> = (official.iter().copied())
            .filter(|picture| picture.copies == 0)
            .collect();
        let from = if fresh.is_empty() { official } else { fresh };
        let at = (Random::new(seed).unit() * from.len() as f64) as usize;
        from.get(at.min(from.len().saturating_sub(1)))
            .map(|picture| picture.item.clone())
    }

    /// The colour copies begun, newest first, as the options list them.
    pub(super) fn copies(&self) -> Vec<CopyShown> {
        (self.sheets.iter().rev())
            .filter(|s| s.is_copy() && s.canvas != 0)
            .map(|copy| {
                let item = copy.laid(0).and_then(|laid| laid.exercise.picture.as_ref());
                let kept = item.filter(|item| self.packs.iter().any(|p| p.find(item).is_some()));
                CopyShown {
                    sheet: ProjectId::new(copy.canvas),
                    number: self.copy_number(copy),
                    name: self.copy_name(copy),
                    picture: kept.cloned(),
                }
            })
            .collect()
    }

    /// `copy`'s name: its sheet's title, "Colour copy n" until renamed.
    pub(super) fn copy_name(&self, copy: &Sheet) -> Title {
        (self.titles.get(&copy.canvas).cloned())
            .unwrap_or_else(|| copy_title(self.copy_number(copy)))
    }

    /// `copy`'s number among the copies.
    fn copy_number(&self, copy: &Sheet) -> u32 {
        number(&self.sheets, copy, copy.in_hand())
    }
}

/// The title a new colour copy's sheet takes, its name until renamed: "Colour copy n".
fn copy_title(number: u32) -> Title {
    Title::new(&format!("Colour copy {number}")).unwrap_or_else(|_| Title::untitled())
}
