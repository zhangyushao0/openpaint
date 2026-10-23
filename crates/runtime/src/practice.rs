//! Practice sessions on the storage worker (G9): a new session's sheet and record are written in
//! one call and the sheet becomes the open artwork; a session's record is written again after
//! each change on its sheet; a test session is removed with its sheet (`practice_test.rs`). And
//! the job a new session starts with: the next session in the app's practice log, seeded from the
//! strokes' seed source (fixed in the harness), its sheet the size its exercises' layout asks.

use openpaint_app::{PortError, PracticeLog, ProjectId, ProjectStore, Title};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{SessionRecord, Sheet};

use crate::error::RuntimeError;
use crate::seeds::Seeds;
use crate::storage::{Done, Job, Kept};

pub(crate) enum PracticeJob {
    /// A new session: its `sheet`, titled `title`, as a new project that becomes the open one,
    /// and its record.
    Start {
        sheet: Artwork,
        title: Title,
        session: SessionRecord,
    },
    /// A session's record changed.
    Save(SessionRecord),
    /// A test session to remove, its record and sheet.
    Remove(SessionRecord),
}

pub(crate) enum PracticeDone {
    /// The session started on project `id`; its record names it.
    Started {
        sheet: Artwork,
        title: Title,
        id: ProjectId,
        session: SessionRecord,
    },
    Saved(Result<(), PortError>),
    /// The session could not start; nothing was written.
    Failed(PortError),
    /// Session `id` removed, or why not (nothing was).
    Removed(u64, Result<(), PortError>),
}

pub(crate) fn run(store: &mut dyn ProjectStore, kept: &mut Kept, job: PracticeJob) -> Done {
    Done::Practice(match job {
        PracticeJob::Start {
            sheet,
            title,
            session,
        } => match store.start_practice(&sheet, &title, &session) {
            Ok(id) => {
                *kept = Kept {
                    open: Some(id),
                    changed: false,
                };
                let session = SessionRecord {
                    canvas: id.get(),
                    ..session
                };
                PracticeDone::Started {
                    sheet,
                    title,
                    id,
                    session,
                }
            }
            Err(error) => {
                log::error!("a practice session did not start: {error}");
                PracticeDone::Failed(error)
            }
        },
        PracticeJob::Save(session) => PracticeDone::Saved(store.save_practice(&session)),
        PracticeJob::Remove(session) => {
            if kept.open.is_some_and(|open| open.get() == session.canvas) {
                *kept = Kept::default();
            }
            PracticeDone::Removed(session.id, store.remove_practice(&session))
        }
    })
}

/// What `job` answers when the store panicked.
pub(crate) const fn stopped(job: &PracticeJob) -> Done {
    Done::Practice(match job {
        PracticeJob::Start { .. } => PracticeDone::Failed(PortError::Stopped),
        PracticeJob::Save(_) => PracticeDone::Saved(Err(PortError::Stopped)),
        PracticeJob::Remove(session) => PracticeDone::Removed(session.id, Err(PortError::Stopped)),
    })
}

/// The job that starts the next session of `log`, seeded from `seeds`, at `now` (seconds since
/// 1970 UTC).
pub(crate) fn start_job(
    log: &PracticeLog,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = u64::from(seeds.next()) << 32 | u64::from(seeds.next());
    let (session, title) = log.begin(seed, now);
    let [width, height] = Sheet::of(session.exercises.len()).size;
    let sheet = Artwork::new(CanvasSize::new(width, height)?);
    log::info!(
        "practice session {} (stage {}, number {}) seeded {seed:016x}",
        session.id,
        session.stage,
        session.number
    );
    Ok(Job::Practice(PracticeJob::Start {
        sheet,
        title,
        session,
    }))
}
