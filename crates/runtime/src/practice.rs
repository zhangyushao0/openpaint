//! Practice on the storage worker (G9): a new session's or colour copy's sheet and record are
//! written in one call and the sheet becomes the open artwork; a record is written again after
//! each change on its sheet; a colour copy renamed (its sheet's title) or removed with its sheet
//! (S83), and a test session or copy removed so (`practice_test.rs`); the user's own pictures
//! removed or moved to another tier in their pack. And the job a new one starts with: the next
//! session in the app's practice log, or a colour copy of the picture chosen, seeded from the
//! strokes' seed source (fixed in the harness), its sheet the size its layout asks.

use openpaint_app::{PortError, PracticeLog, ProjectId, ProjectStore, SheetRead, Title, sheet_of};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{ItemRef, Pack, SessionRecord, Tier};
use openpaint_store::Packs;

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
    /// A colour copy's sheet titled anew.
    Rename(ProjectId, Title),
    /// A colour copy (or a test session) to remove, its record and sheet.
    Remove(SessionRecord),
    /// One of the user's own pictures (its item id) removed, or moved to a tier.
    RemovePicture(String),
    PictureTier(String, Tier),
    /// The sheets of these records read for the practice book, nothing written (S89).
    Read(Vec<SessionRecord>),
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
    /// The sheet of project `id` titled `title`, or why not (nothing changed).
    Renamed(ProjectId, Title, Result<(), PortError>),
    /// Session `id` removed, or why not (nothing was).
    Removed(u64, Result<(), PortError>),
    /// The user's own pictures as their pack holds them now (none left: none), or why they
    /// stayed as they were.
    Pictures(Result<Option<Pack>, PortError>),
    /// What the practice book shows of the sheets read, by record (one not read left out,
    /// logged).
    Read(Vec<(u64, SheetRead)>),
}

pub(crate) fn run(
    (store, packs): (&mut dyn ProjectStore, Option<&Packs>),
    kept: &mut Kept,
    job: PracticeJob,
) -> Done {
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
        PracticeJob::Rename(sheet, title) => {
            let result = store.rename(sheet, &title);
            PracticeDone::Renamed(sheet, title, result)
        }
        PracticeJob::Remove(session) => {
            if kept.open.is_some_and(|open| open.get() == session.canvas) {
                *kept = Kept::default();
            }
            PracticeDone::Removed(session.id, store.remove_practice(&session))
        }
        PracticeJob::RemovePicture(item) => {
            let removed = packs.map(|packs| packs.remove_picture(&item));
            PracticeDone::Pictures(removed.unwrap_or(Err(PortError::Stopped)))
        }
        PracticeJob::PictureTier(item, tier) => {
            let moved = packs.map(|packs| packs.set_tier(&item, tier));
            PracticeDone::Pictures(moved.unwrap_or(Err(PortError::Stopped)))
        }
        PracticeJob::Read(records) => PracticeDone::Read(read_sheets(store, &records)),
    })
}

/// What the practice book shows of `records`' sheets, each read without writing anything.
fn read_sheets(store: &mut dyn ProjectStore, records: &[SessionRecord]) -> Vec<(u64, SheetRead)> {
    let mut read = Vec::with_capacity(records.len());
    for record in records {
        match store.sheet(ProjectId::new(record.canvas)) {
            Ok(artwork) => read.push((record.id, SheetRead::of(&artwork, record))),
            Err(error) => log::warn!(
                "the sheet of practice record {} not read: {error}",
                record.id
            ),
        }
    }
    read
}

/// What `job` answers when the store panicked.
pub(crate) fn stopped(job: &PracticeJob) -> Done {
    let error = PortError::Stopped;
    Done::Practice(match job {
        PracticeJob::Start { .. } => PracticeDone::Failed(error),
        PracticeJob::Save(_) => PracticeDone::Saved(Err(error)),
        PracticeJob::Rename(sheet, title) => {
            PracticeDone::Renamed(*sheet, title.clone(), Err(error))
        }
        PracticeJob::Remove(session) => PracticeDone::Removed(session.id, Err(error)),
        PracticeJob::RemovePicture(_) | PracticeJob::PictureTier(..) => {
            PracticeDone::Pictures(Err(error))
        }
        PracticeJob::Read(_) => PracticeDone::Read(Vec::new()),
    })
}

/// The job that starts the next session of `stage` in `log` (or the one numbered `number`: a
/// test session's), seeded from `seeds`, at `now` (seconds since 1970 UTC).
pub(crate) fn start_job(
    log: &PracticeLog,
    (stage, number): (u8, Option<u32>),
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (session, title) = log.begin((stage, number), seed, now);
    log::info!(
        "practice session {} (stage {}, number {}) seeded {seed:016x}",
        session.id,
        session.stage,
        session.number
    );
    started(log, session, title)
}

/// The job that begins a long colour copy of `picture` (the user chose it) from the packs of
/// `log`, seeded from `seeds`, at `now`.
///
/// # Errors
/// `NotNow` when no pack kept holds the picture.
pub(crate) fn copy_job(
    log: &PracticeLog,
    picture: &ItemRef,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (copy, title) =
        (log.begin_copy(picture, seed, now)).ok_or(RuntimeError::NotNow("a picture to copy"))?;
    log::info!(
        "colour copy {} (number {}) of {}/{}",
        copy.id,
        copy.number,
        picture.pack,
        picture.item
    );
    started(log, copy, title)
}

pub(crate) fn seed(seeds: &mut Seeds) -> u64 {
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
