//! Practice (G9): sessions of the course's exercises on practice sheets. A practice sheet is
//! stored like an artwork (its strokes and fills through the same history, autosave and
//! library), but the gallery never lists it: it belongs to its session's record, which the store
//! keeps beside the artworks (`ProjectStore::practice`). The open sheet's flow is
//! `practice_sheet.rs` (Compare in `practice_compare.rs`, its views in `practice_view.rs`), its
//! pens `practice_pens.rs`, the copy read from its records `practice_copy.rs`.
//!
//! `PracticeLog` is the app's, kept across artworks like the toolbox: every session's record,
//! for the gallery's Today card (the stage, sessions finished, the one still open) and for the
//! summary's earlier sessions, and the practice packs kept (their manifests: the pictures single
//! shapes copy, `practice_pictures.rs`). Progress counts sessions, never days; the stage is the
//! last session's, the next one offered once a stage's sessions are done and earlier at the
//! user's choice, an earlier one revisited.

use std::sync::Arc;

use openpaint_core::Color;
use openpaint_practice::{
    Before, ExerciseResult, Family, KindSummary, Original, Pack, Pair, Pictures, Rgb8,
    STAGE_SESSIONS, STAGES_READY, SessionRecord, Sheet, met, plan, used,
};

use crate::layer_rules::LayerRefusal;
use crate::practice_pictures::{PackShown, PictureImage, SheetPicture};
use crate::project::ProjectId;
use crate::projection::Availability;
use crate::title::Title;
use crate::view::Viewport;

/// What the user can ask of a practice session.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticeIntent {
    /// The instructions for the shown exercise's kind, shown (true) or closed.
    Instructions(bool),
    /// The original laid over the copy (true; the first time stores the exercise's result, every
    /// time counts), or taken away.
    Compare(bool),
    /// A line's original laid over the copy aligned onto it (true) or in place.
    Aligned(bool),
    /// The pen the exercise's own tools draw lines with.
    Pen(Pen),
    /// The full interface over the sheet (true), or the exercise's own tools.
    AllTools(bool),
    /// The shown colour's empty frame filled with `Color` (the wheel beside it).
    Fill(Color),
    /// On to the next exercise, the view moved to it within `viewport` at its zoom; after the
    /// last the session ends and its summary shows over the whole sheet.
    Next(Viewport),
    /// Another session, from the summary.
    OneMore,
    /// Closes the notice.
    Dismiss,
}

/// The pens a line exercise's own tools offer (`practice_pens.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pen {
    /// A light pencil: marks and construction lines, left out of the aligned fit.
    Sketch,
    /// The ink pen the copy is drawn with.
    Draw,
    /// The blue pen that corrects a compared copy.
    Correct,
}

/// Why the practice sheet did not take a contact or a fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeNotice {
    /// The pen landed outside the shown exercise's empty frame.
    DrawInFrame,
    /// The eyedropper over an original not compared yet.
    PickAfterCompare,
    /// The frame could not be filled: the layer is hidden or locked.
    CannotFill(LayerRefusal),
    /// The session is done.
    Done,
}

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
}

/// The open practice sheet for one UI frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeProjection {
    /// The session's frames and originals; the first `revealed` pairs show.
    pub sheet: Arc<SheetView>,
    pub revealed: usize,
    pub stage: u8,
    /// The session's number in its stage.
    pub number: u32,
    /// The exercise shown (none once the session is done), its family (none once done), whether
    /// it is part of the warm-up, its place among the warm-up's exercises or those of its family
    /// after it and how many those are.
    pub current: Option<usize>,
    pub family: Option<Family>,
    pub warming_up: bool,
    pub place: (usize, usize),
    /// Whether Next ends the session.
    pub last: bool,
    pub compare: Availability,
    pub comparing: bool,
    /// A line's original lies over the copy aligned onto it, else in place.
    pub aligned: bool,
    /// The full interface shows over the sheet, else the exercise's own tools.
    pub all_tools: bool,
    /// The pen a line exercise's own tools draw with now and its diameter (canvas pixels);
    /// `None` while they do not show or the toolbox's eraser is used.
    pub pen: Option<(Pen, f32)>,
    /// The shown exercise's stored result, once compared.
    pub result: Option<ExerciseResult>,
    /// What lies over the copy while comparing.
    pub overlay: Option<Overlay>,
    /// The instructions shown, for a family.
    pub instructions: Option<Family>,
    /// Once done: this session's means beside the earlier ones', per kind.
    pub summary: Option<Vec<KindSummary>>,
    pub notice: Option<PracticeNotice>,
    /// Whether Aligned can be chosen for the shown exercise: lines, or a picture with an outline.
    pub aligned_offered: bool,
    /// Aligned is chosen and its fit is still being found (off the frame loop).
    pub fitting: bool,
    /// The session's pictures read so far (none: one that could not be read).
    pub images: Vec<(openpaint_practice::ItemRef, Option<Arc<PictureImage>>)>,
}

/// A session's sheet as the screen draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetView {
    pub size: [u32; 2],
    pub pairs: Vec<Pair>,
    pub originals: Vec<Original>,
    /// Each exercise's picture, a single shape's (none: another kind, or a picture no pack
    /// holds now).
    pub pictures: Vec<Option<Arc<SheetPicture>>>,
}

impl SheetView {
    /// The sheet of `record`, its shapes' pictures as `packs` hold them.
    #[must_use]
    pub fn of(record: &SessionRecord, packs: &[Pack]) -> Self {
        let sheet = Sheet::of(record.exercises.len());
        Self {
            size: sheet.size,
            pairs: sheet.pairs,
            originals: record.exercises.iter().map(|e| e.original()).collect(),
            pictures: (record.exercises.iter())
                .map(|e| SheetPicture::of(e.picture.as_ref()?, packs).map(Arc::new))
                .collect(),
        }
    }
}

/// The comparison over the copy's frame, as the last Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    /// The original's lines where they lie over the copy (in place or aligned), in canvas pixels.
    Lines(Vec<Vec<[f32; 2]>>),
    /// A single shape's picture over the copy, faint, its corners (top left, top right, bottom
    /// right, bottom left) in canvas pixels, with its outline's lines (none without one), in
    /// place or aligned.
    Picture {
        corners: [[f32; 2]; 4],
        outline: Vec<Vec<[f32; 2]>>,
    },
    /// The swatch and the first and last colours laid.
    Colour {
        original: Rgb8,
        first: Rgb8,
        last: Rgb8,
    },
}

/// Every practice session the library holds, oldest first, and the practice packs kept.
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

    /// The stage practised: the last session's, 1 before any.
    #[must_use]
    pub fn stage(&self) -> u8 {
        self.sessions
            .last()
            .map_or(1, |s| s.stage.clamp(1, STAGES_READY))
    }

    /// The Today card: the stage practised, its finished sessions, the open one, the next stage
    /// once this one's sessions are done, and the packs kept.
    #[must_use]
    pub fn today(&self) -> TodayProjection {
        let stage = self.stage();
        let done = self
            .sessions
            .iter()
            .filter(|s| s.stage == stage && s.finished.is_some())
            .count();
        let done = u32::try_from(done).unwrap_or(u32::MAX);
        let open = self
            .sessions
            .iter()
            .rev()
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
        }
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
        let number = self.sessions.iter().filter(|s| s.stage == stage).count() + 1;
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

    /// The id the next session begun takes.
    #[must_use]
    pub fn next_id(&self) -> u64 {
        self.sessions.last().map_or(1, |s| s.id + 1)
    }

    /// Session `id`, if the log holds it.
    #[must_use]
    pub fn session(&self, id: u64) -> Option<&SessionRecord> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// The session whose sheet is project `id`.
    #[must_use]
    pub fn of_canvas(&self, id: ProjectId) -> Option<&SessionRecord> {
        self.sessions.iter().find(|s| s.canvas == id.get())
    }

    /// The finished sessions of `session`'s stage before it.
    #[must_use]
    pub fn before(&self, session: &SessionRecord) -> Vec<SessionRecord> {
        self.sessions
            .iter()
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
