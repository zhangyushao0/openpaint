//! Practice (G9): sessions of the course's exercises on practice sheets. A practice sheet is
//! stored like an artwork (its strokes and fills through the same history, autosave and
//! library), but the gallery never lists it: it belongs to its session's record, which the store
//! keeps beside the artworks (`ProjectStore::practice`). The open sheet's flow is
//! `practice_sheet.rs`, the copy read from its records `practice_copy.rs`.
//!
//! `PracticeLog` is the app's, kept across artworks like the toolbox: every session's record,
//! for the gallery's Today card (the stage, sessions finished, the one still open) and for the
//! summary's earlier sessions. Progress counts sessions, never days.

use std::sync::Arc;

use openpaint_practice::{
    KindSummary, Metrics, Original, Pair, Rgb8, STAGE_SESSIONS, SessionRecord, Sheet,
};

use crate::project::ProjectId;
use crate::projection::Availability;
use crate::title::Title;
use crate::view::Viewport;

/// What the user can ask of a practice session.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticeIntent {
    /// The instructions for the shown exercise's kind, shown (true) or closed.
    Instructions(bool),
    /// The original laid over the copy and the numbers (true; the first time checks the
    /// exercise and stores its result), or neither.
    Compare(bool),
    /// On to the next exercise, the view moved to it within `viewport` at its zoom; after the
    /// last the session ends and its summary shows over the whole sheet.
    Next(Viewport),
    /// Another session, from the summary.
    OneMore,
    /// Closes the notice.
    Dismiss,
}

/// Why the practice sheet did not take a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeNotice {
    /// The pen landed outside the shown exercise's empty frame.
    DrawInFrame,
    /// The shown exercise was checked: its frame takes no more strokes.
    Checked,
    /// The eyedropper over an original not checked yet.
    PickAfterCompare,
    /// The session is done.
    Done,
}

/// The gallery's Today card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodayProjection {
    pub stage: u8,
    /// Sessions finished in the stage, and about how many it holds.
    pub done: u32,
    pub sessions: u32,
    /// The sheet of the session still open, to continue.
    pub open: Option<ProjectId>,
}

impl TodayProjection {
    /// What stage 1 practises.
    #[must_use]
    pub const fn topic(&self) -> &'static str {
        "Lines and colours"
    }
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
    /// The exercise shown (none once the session is done), whether it is a line, its place
    /// among its kind and how many of its kind the session holds.
    pub current: Option<usize>,
    pub line: bool,
    pub place: (usize, usize),
    /// Whether Next ends the session.
    pub last: bool,
    pub compare: Availability,
    pub comparing: bool,
    /// The shown exercise's stored numbers, once checked.
    pub result: Option<Metrics>,
    /// What lies over the copy while comparing.
    pub overlay: Option<Overlay>,
    /// The instructions shown: for lines (true) or colours.
    pub instructions: Option<bool>,
    /// Once done: this session's means beside the earlier ones', per kind.
    pub summary: Option<Vec<KindSummary>>,
    pub notice: Option<PracticeNotice>,
}

/// A session's sheet as the screen draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetView {
    pub size: [u32; 2],
    pub pairs: Vec<Pair>,
    pub originals: Vec<Original>,
}

impl SheetView {
    #[must_use]
    pub fn of(record: &SessionRecord) -> Self {
        let sheet = Sheet::of(record.exercises.len());
        Self {
            size: sheet.size,
            pairs: sheet.pairs,
            originals: record.exercises.iter().map(|e| e.original()).collect(),
        }
    }
}

/// The comparison over the copy's frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    /// Each key point of the original, moved into the copy's frame, and the nearest point of
    /// the copy, in canvas pixels.
    Line(Vec<([f32; 2], [f32; 2])>),
    /// The swatch and the first and last colours laid.
    Colour {
        original: Rgb8,
        first: Rgb8,
        last: Rgb8,
    },
}

/// Every practice session the library holds, oldest first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PracticeLog {
    sessions: Vec<SessionRecord>,
}

impl PracticeLog {
    #[must_use]
    pub fn new(mut sessions: Vec<SessionRecord>) -> Self {
        sessions.sort_by_key(|session| session.id);
        Self { sessions }
    }

    /// The Today card: stage 1 (the only stage so far), its finished sessions and the open one.
    #[must_use]
    pub fn today(&self) -> TodayProjection {
        let stage = 1;
        let done = self
            .sessions
            .iter()
            .filter(|s| s.stage == stage && s.finished.is_some())
            .count();
        let open = self
            .sessions
            .iter()
            .rev()
            .find(|s| s.finished.is_none() && s.canvas != 0)
            .map(|s| ProjectId::new(s.canvas));
        TodayProjection {
            stage,
            done: u32::try_from(done).unwrap_or(u32::MAX),
            sessions: STAGE_SESSIONS,
            open,
        }
    }

    /// The next session of stage 1, seeded `seed`, started at `now` (seconds since 1970 UTC),
    /// with its sheet's title; it has no canvas until the store gives it one.
    #[must_use]
    pub fn begin(&self, seed: u64, now: u64) -> (SessionRecord, Title) {
        let id = self.sessions.last().map_or(1, |s| s.id + 1);
        let number = self.sessions.iter().filter(|s| s.stage == 1).count() + 1;
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let title = Title::new(&format!("Practice {number}")).unwrap_or_else(|_| Title::untitled());
        (SessionRecord::stage_one(id, number, seed, now), title)
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
}
