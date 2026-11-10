//! The app's practice log (`PracticeLog`), kept across artworks like the toolbox: every
//! session's record, for the gallery's Today card (the stage, sessions finished, the one still
//! open) and for the summary's earlier sessions; the long colour copies beside them with their
//! sheets' titles (their names); and the practice packs kept (their manifests: the pictures single
//! shapes and colour copies copy, `pictures.rs`). Progress counts sessions, never days;
//! the stage is the last session's, the next one offered once a stage's sessions are done and
//! earlier at the user's choice, an earlier one revisited. A long colour copy counts in no stage:
//! it is worked on whenever the user likes, of a picture the user chose (`today.rs`).
//! Nor does a drill (S91): one kind on its own sheet, chosen from the options' tiles. Sessions,
//! drills and copies are lessons, counted in the order of sheet and lesson (`lessons`).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use openpaint_practice::{
    Before, Book, Family, ItemRef, Kind, Lesson, OnSheet, PERSONAL, Pack, PackItem, PictureKind,
    Pictures, Random, STAGE_SESSIONS, STAGES_READY, Sheet, Tally, drill, kinds_of, lessons,
    local_day, met, number, plan, recent, scales_met, used,
};

use crate::book::{BookProjection, BookRecord, SheetRead};
use crate::pictures::PackShown;
use crate::today::{CopyShown, DrillShown, PictureShown, TodayProjection, pictures_shown};
use openpaint_app::ProjectId;
use openpaint_app::Title;

/// Every practice session and colour copy the library holds, oldest first, the titles of the
/// copies' sheets, and the practice packs kept; the practice book made of them on the device's
/// clock, with the sheets read for it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PracticeLog {
    sheets: Vec<Sheet>,
    /// Each colour copy's sheet's title, by its project.
    titles: HashMap<u64, Title>,
    packs: Vec<Pack>,
    /// The packs' pictures as the options show them, made again as the log changes.
    pictures: Arc<[PictureShown]>,
    /// The device's offset from UTC in minutes, and the local day now.
    offset: i32,
    today: i64,
    /// The practice book and the records it names, made again as the log changes; the sheets
    /// read for it, by record (one is read again once its record changed).
    book: Arc<Book>,
    named: Arc<[BookRecord]>,
    read: Arc<BTreeMap<u64, Arc<SheetRead>>>,
}

impl PracticeLog {
    /// The log of `sessions` (with the titles of the sheets `titles` names) and `packs`.
    #[must_use]
    pub fn new(mut sheets: Vec<Sheet>, titles: Vec<(ProjectId, Title)>, packs: Vec<Pack>) -> Self {
        sheets.sort_by_key(|sheet| sheet.id);
        let mut log = Self {
            sheets,
            titles: (titles.into_iter())
                .map(|(id, title)| (id.get(), title))
                .collect(),
            ..Self::default()
        };
        log.set_packs(packs);
        log.booked();
        log
    }

    /// The session lessons, colour copies and drills left out.
    fn sessions(&self) -> impl DoubleEndedIterator<Item = OnSheet<'_>> {
        lessons(&self.sheets)
            .into_iter()
            .filter(|s| s.stage().is_some())
    }

    /// The stage practised: the last session's, 1 before any.
    #[must_use]
    pub fn stage(&self) -> u8 {
        self.sessions()
            .last()
            .map_or(1, |s| s.stage().unwrap_or(1).clamp(1, STAGES_READY))
    }

    /// The Today card: the stage practised, its finished sessions, the open one, the next stage
    /// once this one's sessions are done, the packs kept, the colour copies and the pictures.
    #[must_use]
    pub fn today(&self) -> TodayProjection {
        let stage = self.stage();
        let done = (self.sessions())
            .filter(|s| s.stage() == Some(stage) && s.finished().is_some())
            .count();
        let done = u32::try_from(done).unwrap_or(u32::MAX);
        let open = (self.sessions().rev())
            .find(|s| s.finished().is_none() && s.sheet.canvas != 0)
            .map(|s| ProjectId::new(s.sheet.canvas));
        let any = |held: fn(&PackItem) -> bool| self.packs.iter().any(|p| p.items.iter().any(held));
        TodayProjection {
            stage,
            done,
            sessions: STAGE_SESSIONS,
            open,
            next: (done >= STAGE_SESSIONS && stage < STAGES_READY).then_some(stage + 1),
            ready: STAGES_READY,
            packs: self.packs.iter().map(PackShown::of).collect(),
            compositions: any(|item| item.kind == PictureKind::Composition),
            crops: any(|item| item.kind == PictureKind::Crop),
            masses: any(|item| item.masses.is_some()),
            copies: self.copies(),
            pictures: Arc::clone(&self.pictures),
            book: BookProjection {
                book: Arc::clone(&self.book),
                today: self.today,
                records: Arc::clone(&self.named),
                read: Arc::clone(&self.read),
            },
            drills: self.drills(),
        }
    }

    /// Each ready stage's kinds: in ink once compared anywhere, with the newest drill of it left
    /// unfinished, which its tile continues.
    fn drills(&self) -> Vec<DrillShown> {
        let met: Vec<Family> = (met(&self.sheets).iter()).map(|k| k.family()).collect();
        let lessons = lessons(&self.sheets);
        let open = |family: Family| {
            (lessons.iter().rev())
                .find(|s| {
                    s.drilled() == Some(family) && s.finished().is_none() && s.sheet.canvas != 0
                })
                .map(|s| ProjectId::new(s.sheet.canvas))
        };
        (1..=STAGES_READY)
            .flat_map(|stage| kinds_of(stage).iter().map(move |family| (stage, *family)))
            .map(|(stage, family)| DrillShown {
                stage,
                family,
                compared: met.contains(&family),
                open: open(family),
            })
            .collect()
    }

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

    /// The device's clock now: `now` seconds since 1970 UTC, `offset` minutes from UTC.
    pub fn set_clock(&mut self, now: u64, offset: i32) {
        self.today = local_day(now, offset);
        if offset != self.offset {
            self.offset = offset;
            self.booked();
        }
    }

    /// The sheets the store read for the book, by record.
    pub fn sheets_read(&mut self, read: Vec<(u64, SheetRead)>) {
        let mut kept = (*self.read).clone();
        kept.extend(read.into_iter().map(|(id, sheet)| (id, Arc::new(sheet))));
        self.read = Arc::new(kept);
    }

    /// What every lesson but `sheet`'s in hand adds up to: what its seals are earned beside.
    #[must_use]
    pub fn tally_beside(&self, sheet: &Sheet) -> Tally {
        Tally::beside(&self.sheets, sheet)
    }

    /// The book made again from the records.
    fn booked(&mut self) {
        self.book = Arc::new(Book::of(&self.sheets, self.stage(), self.offset));
        self.named = (self.sheets.iter())
            .filter(|s| s.canvas != 0)
            .map(|s| BookRecord {
                id: s.id,
                sheet: ProjectId::new(s.canvas),
                number: self.number(s),
                drill: s.drilled(),
                copy: (s.is_copy()).then(|| {
                    (self.titles.get(&s.canvas).cloned())
                        .unwrap_or_else(|| copy_title(self.number(s)))
                }),
            })
            .collect();
    }

    /// The colour copies begun, newest first, as the options list them.
    fn copies(&self) -> Vec<CopyShown> {
        (self.sheets.iter().rev())
            .filter(|s| s.is_copy() && s.canvas != 0)
            .map(|copy| {
                let item = copy.laid(0).and_then(|laid| laid.exercise.picture.as_ref());
                let kept = item.filter(|item| self.packs.iter().any(|p| p.find(item).is_some()));
                let name = (self.titles.get(&copy.canvas).cloned())
                    .unwrap_or_else(|| copy_title(self.number(copy)));
                CopyShown {
                    sheet: ProjectId::new(copy.canvas),
                    number: self.number(copy),
                    name,
                    picture: kept.cloned(),
                }
            })
            .collect()
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

    /// The packs kept now (as the store listed them).
    pub fn set_packs(&mut self, mut packs: Vec<Pack>) {
        packs.sort_by(|a, b| (a.personal(), &a.id).cmp(&(b.personal(), &b.id)));
        self.packs = packs;
        self.shown();
    }

    /// Keeps `pack`, new or a newer edition of one kept.
    pub fn keep_pack(&mut self, pack: Pack) {
        let mut packs = std::mem::take(&mut self.packs);
        packs.retain(|kept| kept.id != pack.id);
        packs.push(pack);
        self.set_packs(packs);
    }

    /// The user's own pictures as the store keeps them now: `pack`, or none left.
    pub fn set_personal(&mut self, pack: Option<Pack>) {
        match pack {
            Some(pack) => self.keep_pack(pack),
            None => {
                let mut packs = std::mem::take(&mut self.packs);
                packs.retain(|kept| kept.id != PERSONAL);
                self.set_packs(packs);
            }
        }
    }

    /// The packs kept.
    #[must_use]
    pub fn packs(&self) -> &[Pack] {
        &self.packs
    }

    /// The id the next session or copy begun takes.
    #[must_use]
    pub fn next_id(&self) -> u64 {
        self.sheets.last().map_or(1, |s| s.id + 1)
    }

    /// Session (or copy) `id`, if the log holds it.
    #[must_use]
    pub fn session(&self, id: u64) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }

    /// The number of `sheet`'s lesson in hand among the lessons of its kind the log holds
    /// (`openpaint_practice::number`).
    #[must_use]
    pub fn number(&self, sheet: &Sheet) -> u32 {
        number(&self.sheets, sheet, sheet.in_hand())
    }

    /// The session or copy whose sheet is project `id`.
    #[must_use]
    pub fn of_canvas(&self, id: ProjectId) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.canvas == id.get())
    }

    /// What the summary of `session`'s lesson in hand sets it beside: the finished session
    /// lessons of its stage before it, or for a drill the finished drills of its kind before it
    /// (never a session), earlier ones on its sheet included.
    #[must_use]
    pub fn before(&self, session: &Sheet) -> Vec<Lesson> {
        let Some(this) = session.held() else {
            return Vec::new();
        };
        let alike = |s: &OnSheet<'_>| match this.drilled() {
            Some(family) => s.drilled() == Some(family),
            None => s.stage().is_some() && s.stage() == this.stage(),
        };
        (lessons(&self.sheets).into_iter())
            .filter(|s| alike(s) && s.order() < this.order() && s.finished().is_some())
            .map(|s| s.lesson.clone())
            .collect()
    }

    /// Keeps `session`, new or newer; its sheet is read again when the book asks.
    pub fn keep(&mut self, session: Sheet) {
        if self.read.contains_key(&session.id) {
            let mut read = (*self.read).clone();
            read.remove(&session.id);
            self.read = Arc::new(read);
        }
        match self.sheets.iter_mut().find(|s| s.id == session.id) {
            Some(kept) => *kept = session,
            None => {
                self.sheets.push(session);
                self.sheets.sort_by_key(|s| s.id);
                self.shown();
            }
        }
        self.booked();
    }

    /// The sheet of project `sheet` is titled `title` (a colour copy named or renamed).
    pub fn name(&mut self, sheet: ProjectId, title: Title) {
        self.titles.insert(sheet.get(), title);
        self.booked();
    }

    /// Forgets session `id`, which the store removed, with its sheet's title.
    pub fn forget(&mut self, id: u64) {
        if let Some(canvas) = self.session(id).map(|session| session.canvas) {
            self.titles.remove(&canvas);
        }
        self.sheets.retain(|s| s.id != id);
        self.shown();
        self.booked();
    }

    /// The pictures as the options show them, made again.
    fn shown(&mut self) {
        self.pictures = pictures_shown(&self.packs, &self.sheets);
    }
}

/// The title a new colour copy's sheet takes, its name until renamed: "Colour copy n".
fn copy_title(number: u32) -> Title {
    Title::new(&format!("Colour copy {number}")).unwrap_or_else(|_| Title::untitled())
}
