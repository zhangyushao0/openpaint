//! The app's practice log (`PracticeLog`), kept across artworks like the toolbox: every
//! session's record, for the gallery's Today card (the stage, sessions finished, the one still
//! open) and for the summary's earlier sessions; the long colour copies beside them; and the
//! practice packs kept (their manifests: the pictures single shapes and colour copies copy,
//! `practice_pictures.rs`). Progress counts sessions, never days; the stage is the last session's,
//! the next one offered once a stage's sessions are done and earlier at the user's choice, an
//! earlier one revisited. A long colour copy counts in no stage: it is worked on whenever the
//! user likes.

use openpaint_practice::{
    Before, ItemRef, Pack, PictureKind, Pictures, Random, STAGE_SESSIONS, STAGES_READY,
    SessionRecord, met, plan, used,
};

use crate::practice_pictures::PackShown;
use crate::project::ProjectId;
use crate::title::Title;

/// The gallery's Today card and its options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodayProjection {
    pub stage: u8,
    /// Sessions finished in the stage, and about how many it holds.
    pub done: u32,
    pub sessions: u32,
    /// The sheet of the session still open, to continue.
    pub open: Option<ProjectId>,
    /// The next stage, offered on the card once this one's sessions are done.
    pub next: Option<u8>,
    /// The stages a session can be started in, from 1 (the options offer the others).
    pub ready: u8,
    /// The packs kept, the official ones first.
    pub packs: Vec<PackShown>,
    /// The long colour copies, newest first.
    pub copies: Vec<CopyShown>,
    /// Whether a colour copy can be begun from a pack's picture, and from one of the user's own.
    pub copy_from: [bool; 2],
}

/// A long colour copy as the practice options list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyShown {
    /// Its sheet, which opens it.
    pub sheet: ProjectId,
    /// Its number among the copies.
    pub number: u32,
    /// Its picture's title (none: a picture no pack holds now).
    pub title: Option<String>,
}

/// Where a new colour copy's picture comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFrom {
    /// An official pack's picture, one not copied before while there are.
    Pack,
    /// The user's own newest picture not copied before, else their newest.
    Mine,
}

/// Every practice session and colour copy the library holds, oldest first, and the practice
/// packs kept.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PracticeLog {
    sessions: Vec<SessionRecord>,
    packs: Vec<Pack>,
}

impl PracticeLog {
    #[must_use]
    pub fn new(mut sessions: Vec<SessionRecord>, packs: Vec<Pack>) -> Self {
        sessions.sort_by_key(|session| session.id);
        let mut log = Self {
            sessions,
            packs: Vec::new(),
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
    /// once this one's sessions are done, the packs kept and the colour copies.
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
        let pictures = |personal: bool| {
            (self.packs.iter())
                .filter(|pack| pack.personal() == personal)
                .any(|pack| pack.items.iter().any(|i| i.kind == PictureKind::Shape))
        };
        TodayProjection {
            stage,
            done,
            sessions: STAGE_SESSIONS,
            open,
            next: (done >= STAGE_SESSIONS && stage < STAGES_READY).then_some(stage + 1),
            ready: STAGES_READY,
            packs: self.packs.iter().map(PackShown::of).collect(),
            copies: self.copies(),
            copy_from: [pictures(false), pictures(true)],
        }
    }

    /// The colour copies begun, newest first, as the options list them.
    fn copies(&self) -> Vec<CopyShown> {
        (self.sessions.iter().rev())
            .filter(|s| s.is_project() && s.canvas != 0)
            .map(|copy| {
                let item = copy.exercises.first().and_then(|e| e.picture.as_ref());
                let found = item.and_then(|item| self.packs.iter().find_map(|p| p.find(item)));
                CopyShown {
                    sheet: ProjectId::new(copy.canvas),
                    number: copy.number,
                    title: found.map(|item| item.title.clone()),
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

    /// A new colour copy of a picture `from` the packs kept (picked by `seed`), begun at `now`,
    /// with its sheet's title; none without such a picture.
    #[must_use]
    pub fn begin_copy(
        &self,
        from: CopyFrom,
        seed: u64,
        now: u64,
    ) -> Option<(SessionRecord, Title)> {
        let copied: Vec<&ItemRef> = (self.sessions.iter())
            .filter(|s| s.is_project())
            .filter_map(|s| s.exercises.first()?.picture.as_ref())
            .collect();
        let personal = from == CopyFrom::Mine;
        let items: Vec<ItemRef> = (self.packs.iter())
            .filter(|pack| pack.personal() == personal)
            .flat_map(|pack| {
                (pack.items.iter())
                    .filter(|item| item.kind == PictureKind::Shape)
                    .map(|item| ItemRef {
                        pack: pack.id.clone(),
                        item: item.id.clone(),
                    })
            })
            .collect();
        let fresh: Vec<&ItemRef> = items.iter().filter(|i| !copied.contains(i)).collect();
        let from_these: Vec<&ItemRef> = if fresh.is_empty() {
            items.iter().collect()
        } else {
            fresh
        };
        let picture = if personal {
            // The newest: the user's own pictures are kept oldest first.
            from_these.last().copied()
        } else {
            let at = (Random::new(seed).unit() * from_these.len() as f64) as usize;
            from_these
                .get(at.min(from_these.len().saturating_sub(1)))
                .copied()
        }?;
        let number = self.sessions.iter().filter(|s| s.is_project()).count() + 1;
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let title =
            Title::new(&format!("Colour copy {number}")).unwrap_or_else(|_| Title::untitled());
        let record = SessionRecord::project(self.next_id(), number, (seed, now), picture.clone());
        Some((record, title))
    }

    /// The packs kept now (as the store listed them).
    pub fn set_packs(&mut self, mut packs: Vec<Pack>) {
        packs.sort_by(|a, b| (a.personal(), &a.id).cmp(&(b.personal(), &b.id)));
        self.packs = packs;
    }

    /// Keeps `pack`, new or a newer edition of one kept.
    pub fn keep_pack(&mut self, pack: Pack) {
        let mut packs = std::mem::take(&mut self.packs);
        packs.retain(|kept| kept.id != pack.id);
        packs.push(pack);
        self.set_packs(packs);
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
            }
        }
    }

    /// Forgets session `id`, which the store removed.
    pub fn forget(&mut self, id: u64) {
        self.sessions.retain(|s| s.id != id);
    }
}
