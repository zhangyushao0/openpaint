//! Beginning a lesson from the log: the next session of a stage, a drill of a kind (S91) or a
//! long colour copy of a picture chosen, each numbered among its kind's lessons, its pictures
//! picked from the packs kept (others than those shown before while there are).

use openpaint_practice::{
    Before, Family, ItemRef, Kind, PictureKind, Pictures, Random, STAGES_READY, Sheet, drill,
    lessons, met, plan, recent, scales_met, used,
};

use super::{PracticeLog, copy_title};
use crate::today::PictureShown;
use openpaint_app::Title;

impl PracticeLog {
    /// A new drill of `family` seeded `seed`, begun at `now`, numbered among the kind's drills,
    /// its shapes' pictures picked as a session's are (others than those shown before while there
    /// are), with its sheet's title; none for a kind no stage drills, or shapes without a picture.
    #[must_use]
    pub fn begin_drill(&self, family: Family, seed: u64, now: u64) -> Option<(Sheet, Title)> {
        let number = (lessons(&self.sheets).iter())
            .filter(|s| s.drilled() == Some(family))
            .count();
        let number = u32::try_from(number + 1).unwrap_or(u32::MAX);
        let plan = self.planned(&[], |before| drill(family, number, seed, before))?;
        let record = Sheet::drill(self.next_id(), now, plan).framed(&self.packs);
        let title = Title::new(&format!("Drill {number}")).unwrap_or_else(|_| Title::untitled());
        Some((record, title))
    }

    /// The next session of `stage` (one ready, else stage 1), or the one numbered `number` of it
    /// (a test session's, `practice-test start <stage> <n>`), seeded `seed`, started at `now`
    /// (seconds since 1970 UTC), its warm-up from the kinds compared in every session so far and
    /// its shapes' pictures from the packs kept (others than those shown before while there
    /// are), with its sheet's title; it has no canvas until the store gives it one.
    #[must_use]
    pub fn begin(&self, (stage, number): (u8, Option<u32>), seed: u64, now: u64) -> (Sheet, Title) {
        let stage = if (1..=STAGES_READY).contains(&stage) {
            stage
        } else {
            1
        };
        let id = self.next_id();
        let next = self.sessions().filter(|s| s.stage() == Some(stage)).count() + 1;
        let number = number.unwrap_or_else(|| u32::try_from(next).unwrap_or(u32::MAX));
        let number = number.max(1);
        let title = Title::new(&format!("Practice {number}")).unwrap_or_else(|_| Title::untitled());
        let kinds = met(&self.sheets);
        let plan = self.planned(&kinds, |before| plan(stage, number, seed, before));
        let record = Sheet::session(id, stage, now, plan).framed(&self.packs);
        (record, title)
    }

    /// What `plan` makes of what the records tell (`Before`) with the kinds `met` (a drill's none:
    /// it has no warm-up): the pictures kept, used and lately shown, the scales met.
    fn planned<T>(&self, met: &[Kind], plan: impl FnOnce(&Before<'_>) -> T) -> T {
        let (used, recent) = (used(&self.sheets), recent(&self.sheets));
        let pictures = Pictures::of(&self.packs);
        plan(&Before {
            met,
            pictures: &pictures,
            used: &used,
            recent: &recent,
            scales: scales_met(&self.sheets),
        })
    }

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
}
