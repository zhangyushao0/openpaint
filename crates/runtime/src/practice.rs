//! Practice sessions on the storage worker (G9): a new session's or colour copy's sheet and record
//! are written in one call and the sheet becomes the open artwork; a record is written again after
//! each change on its sheet; a test session or copy is removed with its sheet
//! (`practice_test.rs`). And the job a new one starts with: the next session in the app's
//! practice log, or a colour copy of a picture from the packs, seeded from the strokes' seed
//! source (fixed in the harness), its sheet the size its layout asks.

use openpaint_app::{CopyFrom, PortError, PracticeLog, ProjectId, ProjectStore, Title, sheet_of};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::SessionRecord;

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

/// The job that starts the next session of `stage` in `log`, seeded from `seeds`, at `now`
/// (seconds since 1970 UTC).
pub(crate) fn start_job(
    log: &PracticeLog,
    stage: u8,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (session, title) = log.begin(stage, seed, now);
    log::info!(
        "practice session {} (stage {}, number {}) seeded {seed:016x}",
        session.id,
        session.stage,
        session.number
    );
    started(log, session, title)
}

/// The job that begins a long colour copy of a picture `from` the packs of `log`, picked by a
/// seed from `seeds`, at `now`.
///
/// # Errors
/// `NotNow` without such a picture (the options offer none then).
pub(crate) fn copy_job(
    log: &PracticeLog,
    from: CopyFrom,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (copy, title) =
        (log.begin_copy(from, seed, now)).ok_or(RuntimeError::NotNow("a picture to copy"))?;
    log::info!("colour copy {} (number {})", copy.id, copy.number);
    started(log, copy, title)
}

fn seed(seeds: &mut Seeds) -> u64 {
    u64::from(seeds.next()) << 32 | u64::from(seeds.next())
}

/// The job that stores `session`'s new sheet, titled `title`, of the size its layout asks.
fn started(log: &PracticeLog, session: SessionRecord, title: Title) -> Result<Job, RuntimeError> {
    let [width, height] = sheet_of(&session, log.packs()).size;
    let sheet = Artwork::new(CanvasSize::new(width, height)?);
    Ok(Job::Practice(PracticeJob::Start {
        sheet,
        title,
        session,
    }))
}
