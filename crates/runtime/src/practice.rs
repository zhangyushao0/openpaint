//! Practice on the storage worker (G9): a new session's or colour copy's sheet and record are
//! written in one call and the sheet becomes the open artwork; a record is written again after
//! each change on its sheet; a colour copy renamed (its sheet's title) or removed with its sheet
//! (S83), and a test session or copy removed so (`practice_test.rs`); the user's own pictures
//! removed or moved to another tier in their pack. And the job a new one starts with: the next
//! session in the app's practice log, a drill of the kind chosen (S91), or a colour copy of the
//! picture chosen, seeded from the strokes' seed source (fixed in the harness), its sheet the size
//! its layout asks. And the gallery's list, which leaves the practice sheets out (D112).

use std::collections::HashSet;

use openpaint_app::{
    PortError, PracticeLog, ProjectId, ProjectStore, ProjectSummary, SheetRead, Title, sheet_of,
};
use openpaint_core::{Artwork, CanvasSize};
use openpaint_practice::{Family, ItemRef, Pack, SessionRecord, Tier};
use openpaint_practice_store::{Packs, remove, save, sessions, start};

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
        } => match start(store, (&sheet, &title), &session) {
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
        PracticeJob::Save(session) => PracticeDone::Saved(save(store, &session)),
        PracticeJob::Rename(sheet, title) => {
            let result = store.rename(sheet, &title);
            PracticeDone::Renamed(sheet, title, result)
        }
        PracticeJob::Remove(session) => {
            if kept.open.is_some_and(|open| open.get() == session.canvas) {
                *kept = Kept::default();
            }
            PracticeDone::Removed(session.id, remove(store, &session))
        }
        PracticeJob::RemovePicture(item) => {
            let removed = packs.map(|packs| packs.remove_picture(&item));
            PracticeDone::Pictures(removed.unwrap_or(Err(PortError::Stopped)))
        }
        PracticeJob::PictureTier(item, tier) => {
            let moved = packs.map(|packs| packs.set_tier(&item, tier));
            PracticeDone::Pictures(moved.unwrap_or(Err(PortError::Stopped)))
        }
        PracticeJob::Read(records) => PracticeDone::Read(read_sheets((store, packs), &records)),
    })
}

/// What the practice book shows of `records`' sheets, each read without writing anything, their
/// pictures' frames as the packs kept in `packs` hold them.
fn read_sheets(
    (store, packs): (&mut dyn ProjectStore, Option<&Packs>),
    records: &[SessionRecord],
) -> Vec<(u64, SheetRead)> {
    let held = (packs.map(Packs::list).transpose())
        .inspect_err(|error| log::warn!("the practice packs are not listed: {error}"))
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut read = Vec::with_capacity(records.len());
    for record in records {
        match store.artwork(ProjectId::new(record.canvas)) {
            Ok(artwork) => read.push((record.id, SheetRead::of(&artwork, record, &held))),
            Err(error) => log::warn!(
                "the sheet of practice record {} not read: {error}",
                record.id
            ),
        }
    }
    read
}

/// The library's projects as the gallery takes them (`gallery_list`).
pub(crate) struct Listed {
    /// The artworks the gallery lists, the one changed last first.
    pub(crate) artworks: Vec<ProjectSummary>,
    /// The practice sheets, which it never lists, with their titles (a colour copy's is its
    /// name).
    pub(crate) sheets: Vec<(ProjectId, Title)>,
}

/// The gallery's list: the library's projects but the practice sheets, which only their records
/// open (D112), and the sheets apart. The sheets are the projects the practice records name; a
/// record that cannot be read names none.
pub(crate) fn gallery_list(store: &mut dyn ProjectStore) -> Result<Listed, PortError> {
    let sheets: HashSet<u64> = (sessions(store)?.iter())
        .map(|session| session.canvas)
        .filter(|canvas| *canvas != 0)
        .collect();
    let (sheets, artworks): (Vec<ProjectSummary>, Vec<ProjectSummary>) = (store.list()?)
        .into_iter()
        .partition(|project| sheets.contains(&project.id.get()));
    let sheets = sheets.into_iter().map(|sheet| (sheet.id, sheet.title));
    Ok(Listed {
        artworks,
        sheets: sheets.collect(),
    })
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

/// What One more starts once the sheet left is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Next {
    /// The next session of the stage practised.
    Session,
    /// Another drill of the kind (S91).
    Drill(Family),
}

/// The job that begins the next drill of `family` in `log` (S91), seeded from `seeds`, at `now`.
///
/// # Errors
/// `NotNow` for a kind no stage drills, or shapes while no pack holds a picture.
pub(crate) fn drill_job(
    log: &PracticeLog,
    family: Family,
    seeds: &mut Seeds,
    now: u64,
) -> Result<Job, RuntimeError> {
    let seed = seed(seeds);
    let (drill, title) = (log.begin_drill(family, seed, now))
        .ok_or(RuntimeError::NotNow("a kind to drill with its pictures"))?;
    log::info!(
        "practice drill {} of {family:?} (number {}) seeded {seed:016x}",
        drill.id,
        drill.number
    );
    started(log, drill, title)
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
