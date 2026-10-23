//! The app's practice log (`PracticeLog`), kept across artworks like the toolbox: every
//! session's record, for the gallery's Today card (the stage, sessions finished, the one still
//! open) and for the summary's earlier sessions; the long colour copies beside them with their
//! sheets' titles (their names); and the practice packs kept (their manifests: the pictures single
//! shapes and colour copies copy, `practice_pictures.rs`). Progress counts sessions, never days;
//! the stage is the last session's, the next one offered once a stage's sessions are done and
//! earlier at the user's choice, an earlier one revisited. A long colour copy counts in no stage:
//! it is worked on whenever the user likes, of a picture the user chose (`practice_today.rs`).

use std::collections::HashMap;
use std::sync::Arc;

use openpaint_practice::{
    Before, ItemRef, PERSONAL, Pack, PictureKind, Pictures, Random, STAGE_SESSIONS, STAGES_READY,
    SessionRecord, met, plan, used,
};

use crate::practice_pictures::PackShown;
use crate::practice_today::{CopyShown, PictureShown, TodayProjection, pictures_shown};
use crate::project::ProjectId;
use crate::title::Title;

/// Every practice session and colour copy the library holds, oldest first, the titles of the
/// copies' sheets, and the practice packs kept.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PracticeLog {
    sessions: Vec<SessionRecord>,
    /// Each colour copy's sheet's title, by its project.
    titles: HashMap<u64, Title>,
    packs: Vec<Pack>,
    /// The packs' pictures as the options show them, made again as the log changes.
    pictures: Arc<[PictureShown]>,
}

impl PracticeLog {
    /// The log of `sessions` (with the titles of the sheets `titles` names) and `packs`.
    #[must_use]
    pub fn new(
        mut sessions: Vec<SessionRecord>,
        titles: Vec<(ProjectId, Title)>,
        packs: Vec<Pack>,
    ) -> Self {
        sessions.sort_by_key(|session| session.id);
        let mut log = Self {
            sessions,
            titles: (titles.into_iter())
                .map(|(id, title)| (id.get(), title))
                .collect(),
            ..Self::default()
        };
        log.set_packs(packs);
        log
    }

    /// The sessions, colour copies left out.
    fn sessions(&self) -> impl DoubleEndedIterator<Item = &SessionRecord> {
        self.sessions.iter().filter(|s| !s.is_project())
    }

    /// The stage practised: the last session's, 1 before any.
    #[must_use]
    pub fn stage(&self) -> u8 {
        self.sessions()
            .last()
            .map_or(1, |s| s.stage.clamp(1, STAGES_READY))
    }

    /// The Today card: the stage practised, its finished sessions, the open one, the next stage
    /// once this one's sessions are done, the packs kept, the colour copies and the pictures.
    #[must_use]
    pub fn today(&self) -> TodayProjection {
        let stage = self.stage();
        let done = (self.sessions())
            .filter(|s| s.stage == stage && s.finished.is_some())
            .count();
        let done = u32::try_from(done).unwrap_or(u32::MAX);
        let open = (self.sessions().rev())
            .find(|s| s.finished.is_none() && s.canvas != 0)
            .map(|s| ProjectId::new(s.canvas));
        TodayProjection {
            stage,
            done,
            sessions: STAGE_SESSIONS,
            open,
            next: (done >= STAGE_SESSIONS && stage < STAGES_READY).then_some(stage + 1),
            ready: STAGES_READY,
            packs: self.packs.iter().map(PackShown::of).collect(),
            copies: self.copies(),
            pictures: Arc::clone(&self.pictures),
        }
    }

    /// The colour copies begun, newest first, as the options list them.
    fn copies(&self) -> Vec<CopyShown> {
        (self.sessions.iter().rev())
            .filter(|s| s.is_project() && s.canvas != 0)
            .map(|copy| {
                let item = copy.exercises.first().and_then(|e| e.picture.as_ref());
                let kept = item.filter(|item| self.packs.iter().any(|p| p.find(item).is_some()));
                let name = (self.titles.get(&copy.canvas).cloned())
                    .unwrap_or_else(|| copy_title(copy.number));
                CopyShown {
                    sheet: ProjectId::new(copy.canvas),
                    number: copy.number,
                    name,
                    picture: kept.cloned(),
                }
            })
            .collect()
    }

    /// The next session of `stage` (one ready, else stage 1), seeded `seed`, started at `now`
    /// (seconds since 1970 UTC), its warm-up from the kinds compared in every session so far and
    /// its shapes' pictures from the packs kept (others than those shown before while there
    /// are), with its sheet's title; it has no canvas until the store gives it one.
    #[must_use]
    pub fn begin(&self, stage: u8, seed: u64, now: u64) -> (SessionRecord, Title) {
        let stage = if (1..=STAGES_READY).contains(&stage) {
            stage
        } else {
            1
        };
        let id = self.next_id();
        let number = self.sessions().filter(|s| s.stage == stage).count() + 1;
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let title = Title::new(&format!("Practice {number}")).unwrap_or_else(|_| Title::untitled());
        let (met, used) = (met(&self.sessions), used(&self.sessions));
        let pictures = Pictures::of(&self.packs);
        let before = Before {
            met: &met,
            pictures: &pictures,
            used: &used,
        };
        let plan = plan(stage, number, seed, &before);
        let record = SessionRecord::begun(id, (stage, number), (seed, now), plan);
        (record, title)
    }

    /// A new colour copy of `picture` (the user chose it; none unless a pack kept holds it),
    /// seeded `seed`, begun at `now`, with its sheet's title.
    #[must_use]
    pub fn begin_copy(
        &self,
        picture: &ItemRef,
        seed: u64,
        now: u64,
    ) -> Option<(SessionRecord, Title)> {
        let found = self.packs.iter().find_map(|pack| pack.find(picture))?;
        if found.kind != PictureKind::Shape {
            return None;
        }
        let number = self.sessions.iter().filter(|s| s.is_project()).count() + 1;
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let record = SessionRecord::project(self.next_id(), number, (seed, now), picture.clone());
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
        self.sessions.last().map_or(1, |s| s.id + 1)
    }

    /// Session (or copy) `id`, if the log holds it.
    #[must_use]
    pub fn session(&self, id: u64) -> Option<&SessionRecord> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// The session or copy whose sheet is project `id`.
    #[must_use]
    pub fn of_canvas(&self, id: ProjectId) -> Option<&SessionRecord> {
        self.sessions.iter().find(|s| s.canvas == id.get())
    }

    /// The finished sessions of `session`'s stage before it.
    #[must_use]
    pub fn before(&self, session: &SessionRecord) -> Vec<SessionRecord> {
        self.sessions()
            .filter(|s| s.stage == session.stage && s.id < session.id && s.finished.is_some())
            .cloned()
            .collect()
    }

    /// Keeps `session`, new or newer.
    pub fn keep(&mut self, session: SessionRecord) {
        match self.sessions.iter_mut().find(|s| s.id == session.id) {
            Some(kept) => *kept = session,
            None => {
                self.sessions.push(session);
                self.sessions.sort_by_key(|s| s.id);
                self.shown();
            }
        }
    }

    /// The sheet of project `sheet` is titled `title` (a colour copy named or renamed).
    pub fn name(&mut self, sheet: ProjectId, title: Title) {
        self.titles.insert(sheet.get(), title);
    }

    /// Forgets session `id`, which the store removed, with its sheet's title.
    pub fn forget(&mut self, id: u64) {
        if let Some(canvas) = self.session(id).map(|session| session.canvas) {
            self.titles.remove(&canvas);
        }
        self.sessions.retain(|s| s.id != id);
        self.shown();
    }

    /// The pictures as the options show them, made again.
    fn shown(&mut self) {
        self.pictures = pictures_shown(&self.packs, &self.sessions);
    }
}

/// The title a new colour copy's sheet takes, its name until renamed: "Colour copy n".
fn copy_title(number: u32) -> Title {
    Title::new(&format!("Colour copy {number}")).unwrap_or_else(|_| Title::untitled())
}
