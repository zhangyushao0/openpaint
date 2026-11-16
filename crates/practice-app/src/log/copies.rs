//! The projects in the log (a long colour copy, a drawing from life, a cold-warm copy): one begun
//! of a picture chosen, numbered among its own and titled "Colour copy n", "Drawing from life n" or
//! "Cold-warm copy n" until renamed; the test copy's picture; the projects as the options list
//! them.

use openpaint_practice::{ItemRef, PERSONAL, PictureKind, Random, Sheet, lessons, number};

use super::PracticeLog;
use crate::copies::{CopyShown, PictureShown};
use openpaint_app::{ProjectId, Title};

/// The names a project's sheet is titled with until renamed, a number after each.
const COPY_NAME: &str = "Colour copy";
const LIFE_NAME: &str = "Drawing from life";
const COLD_WARM_NAME: &str = "Cold-warm copy";

impl PracticeLog {
    /// A new colour copy of `picture` (the user chose it; none unless a pack kept holds it),
    /// seeded `seed`, begun at `now`, with its sheet's title.
    #[must_use]
    pub fn begin_copy(&self, picture: &ItemRef, seed: u64, now: u64) -> Option<(Sheet, Title)> {
        self.found(picture)?;
        let record = Sheet::copy(self.next_id(), (seed, now), picture.clone());
        Some(self.titled(record.framed(&self.packs)))
    }

    /// A new drawing from life against `photo` (S151: one of the user's own pictures; none
    /// otherwise), seeded `seed`, begun at `now`, with its sheet's title.
    #[must_use]
    pub fn begin_from_life(&self, photo: &ItemRef, seed: u64, now: u64) -> Option<(Sheet, Title)> {
        self.found(photo).filter(|_| photo.pack == PERSONAL)?;
        let record = Sheet::from_life(self.next_id(), (seed, now), photo.clone());
        Some(self.titled(record.framed(&self.packs)))
    }

    /// A new cold-warm copy of `picture` (S154: the user chose it; none unless a pack kept holds
    /// it), its tendencies the pair after the cold-warm copies before it, seeded `seed`, begun at
    /// `now`, with its sheet's title.
    #[must_use]
    pub fn begin_cold_warm(
        &self,
        picture: &ItemRef,
        seed: u64,
        now: u64,
    ) -> Option<(Sheet, Title)> {
        self.found(picture)?;
        let record = Sheet::cold_warm(self.next_id(), (seed, now), picture.clone(), &self.sheets);
        Some(self.titled(record.framed(&self.packs)))
    }

    /// Whether a pack kept holds `picture` as a picture to copy.
    fn found(&self, picture: &ItemRef) -> Option<()> {
        let found = self.packs.iter().find_map(|pack| pack.find(picture))?;
        (found.kind == PictureKind::Shape).then_some(())
    }

    /// `record`, a project just begun, with its title: the next number among its own.
    fn titled(&self, record: Sheet) -> (Sheet, Title) {
        let alike = |sheet: &Sheet| sheet.project_sort() == record.project_sort();
        let number = (lessons(&self.sheets).iter())
            .filter(|on| on.is_copy() && alike(on.sheet))
            .count()
            + 1;
        let title = project_title(&record, u32::try_from(number).unwrap_or(u32::MAX));
        (record, title)
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

    /// The projects begun, newest first, as the options list them.
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
                    from_life: copy.drawn_from_life(),
                }
            })
            .collect()
    }

    /// `copy`'s name: its sheet's title, "Colour copy n" or "Drawing from life n" until renamed.
    pub(super) fn copy_name(&self, copy: &Sheet) -> Title {
        (self.titles.get(&copy.canvas).cloned())
            .unwrap_or_else(|| project_title(copy, self.copy_number(copy)))
    }

    /// `copy`'s number among the projects of its sort.
    fn copy_number(&self, copy: &Sheet) -> u32 {
        number(&self.sheets, copy, copy.in_hand())
    }
}

/// The title a new project's sheet takes, its name until renamed, by its kind's key: "Colour copy
/// n", "Drawing from life n", "Cold-warm copy n".
fn project_title(record: &Sheet, number: u32) -> Title {
    let key = (record.exercises().next()).map(|laid| laid.exercise.kind.facts().key);
    let name = match key {
        Some("observation") => LIFE_NAME,
        Some("cold-warm") => COLD_WARM_NAME,
        _ => COPY_NAME,
    };
    Title::new(&format!("{name} {number}")).unwrap_or_else(|_| Title::untitled())
}
