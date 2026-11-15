//! Practice on the storage worker (G9): a new lesson's or colour copy's sheet and record are
//! written in one call and the sheet becomes the open artwork, or a lesson goes onto today's
//! sheet, or One more's lesson is appended in place (`day.rs`); a record is written again after
//! each change on its sheet; a colour copy renamed (its sheet's title) or removed with its sheet
//! (S83), and a test lesson or copy removed so (`test.rs`); a pack imported, a picture of the
//! user's own added (`files.rs`), removed or moved to another tier in their pack; a picture read
//! for the screen; what a sheet's exercises printed, read when it opens and for the book (S120d).
//! And the job a colour copy of the picture chosen starts with, seeded from the strokes' seed
//! source (fixed in the harness), its sheet the size its layout asks (a lesson's: `lesson.rs`).
//! And the home's list, which leaves the practice sheets out (D112).

use std::collections::HashSet;

use openpaint_app::{ImageSource, PortError, ProjectId, ProjectStore, ProjectSummary, Title};
use openpaint_core::Artwork;
use openpaint_practice::{Given, Givens, ItemRef, Pack, Sheet, Tier, TimerWrite};
use openpaint_practice_app::{PictureImage, PracticeLog, SheetRead};
use openpaint_practice_store::{
    Packs, append, forget_blocks, printed, remove, save, save_timer, sheets, start, trim,
};
use openpaint_store::DisplayCache;

use super::lesson::started;
use super::{day, files};
use crate::error::RuntimeError;
use crate::seeds::Seeds;
use crate::storage::{Done, Job, Kept, Loaded};

pub(crate) enum PracticeJob {
    /// A new sheet: its artwork `sheet`, titled `title`, as a new project that becomes the open
    /// one, and its `record`.
    Start {
        sheet: Artwork,
        title: Title,
        record: Sheet,
        /// What its lesson prints, kept beside it once (S120), and each exercise's of it in order.
        givens: Givens,
        printed: Vec<Given>,
    },
    /// A new lesson (`start`, a `Start`) onto today's sheet `day` from a new row, where
    /// its canvas holds it, else as `start` (`day.rs`).
    Continue {
        day: Sheet,
        start: Box<PracticeJob>,
    },
    /// One more in place: `sheet` with its last lesson appended and `givens`, what it prints.
    Append {
        sheet: Sheet,
        givens: Givens,
    },
    /// A sheet's record changed.
    Save(Sheet),
    /// A sheet's record cut by the Pomodoro's End, with its last lesson's givens (S124).
    Trim(Sheet),
    /// The Pomodoro's settings, its block under way and a block completed (S124).
    Timer(TimerWrite),
    /// The Pomodoro's blocks begun at or after this second removed (the test chore's end).
    ForgetBlocks(u64),
    /// A colour copy's sheet titled anew.
    Rename(ProjectId, Title),
    /// A colour copy (or a test lesson) to remove, its record and sheet.
    Remove(Sheet),
    /// One of the user's own pictures (its item id) removed, or moved to a tier.
    RemovePicture(String),
    PictureTier(String, Tier),
    /// The sheets of these records read for the practice book, nothing written (S89).
    Read(Vec<Sheet>),
    /// The file chosen kept as a practice pack.
    ImportPack(Box<dyn ImageSource>),
    /// The picture chosen added to the user's own at the tier, its outline derived.
    AddPicture(Box<dyn ImageSource>, Tier),
    /// A pack's picture read for the screen.
    Picture(ItemRef),
}

pub(crate) enum PracticeDone {
    /// The sheet `record` started on project `id`; the record names it; what its exercises print.
    Started {
        sheet: Artwork,
        title: Title,
        id: ProjectId,
        record: Sheet,
        printed: Vec<Given>,
    },
    /// A lesson went onto today's sheet, its `record`, opened as `loaded`.
    Continued {
        record: Sheet,
        loaded: Box<Loaded>,
    },
    /// The lesson appended to sheet `id` (its lessons with it) stored, or why not (nothing was).
    Appended {
        id: u64,
        lessons: usize,
        result: Result<(), PortError>,
    },
    Saved(Result<(), PortError>),
    /// The sheet could not start; nothing was written.
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
    /// The pack kept, or why not.
    PackImported(Result<Pack, PortError>),
    /// The user's own pictures with the one added last, or why it was not.
    PictureAdded(Result<Pack, PortError>),
    /// A pack's picture as read for the screen; none when it could not be (logged).
    Picture(ItemRef, Option<PictureImage>),
}

pub(crate) fn run(
    (store, cache, packs): (&mut dyn ProjectStore, Option<&DisplayCache>, Option<&Packs>),
    kept: &mut Kept,
    job: PracticeJob,
) -> Done {
    Done::Practice(match job {
        PracticeJob::Continue { day: sheet, start } => {
            return day::continued((store, cache, packs), kept, (sheet, *start));
        }
        PracticeJob::Append { sheet, givens } => PracticeDone::Appended {
            id: sheet.id,
            lessons: sheet.lessons.len(),
            result: append(store, &sheet, &givens),
        },
        job @ PracticeJob::Start { .. } => begun(store, kept, job),
        PracticeJob::Save(record) => PracticeDone::Saved(save(store, &record)),
        PracticeJob::Trim(record) => PracticeDone::Saved(trim(store, &record)),
        PracticeJob::Timer(write) => PracticeDone::Saved(save_timer(store, &write)),
        PracticeJob::ForgetBlocks(since) => PracticeDone::Saved(
            forget_blocks(store, since).map(|gone| log::info!("{gone} test blocks gone")),
        ),
        PracticeJob::Rename(sheet, title) => {
            let result = store.rename(sheet, &title);
            PracticeDone::Renamed(sheet, title, result)
        }
        PracticeJob::Remove(record) => {
            if kept.open.is_some_and(|open| open.get() == record.canvas) {
                *kept = Kept::default();
            }
            PracticeDone::Removed(record.id, remove(store, &record))
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
        PracticeJob::ImportPack(source) => PracticeDone::PackImported(files::import(packs, source)),
        PracticeJob::AddPicture(source, tier) => {
            PracticeDone::PictureAdded(files::add(packs, source, tier))
        }
        PracticeJob::Picture(item) => {
            let image = files::picture(packs, &item);
            PracticeDone::Picture(item, image)
        }
    })
}

/// A new sheet started: its project and record written in one call, the project the open one.
fn begun(store: &mut dyn ProjectStore, kept: &mut Kept, job: PracticeJob) -> PracticeDone {
    let PracticeJob::Start {
        sheet,
        title,
        record,
        givens,
        printed,
    } = job
    else {
        return PracticeDone::Failed(PortError::Stopped);
    };
    match start(store, (&sheet, &title), &record, &givens) {
        Ok(id) => {
            kept.open = Some(id);
            kept.changed = false;
            let record = Sheet {
                canvas: id.get(),
                ..record
            };
            PracticeDone::Started {
                sheet,
                title,
                id,
                record,
                printed,
            }
        }
        Err(error) => {
            log::error!("a practice session did not start: {error}");
            PracticeDone::Failed(error)
        }
    }
}

/// What the practice book shows of `records`' sheets, each read without writing anything, their
/// frames as their exercises were laid and their originals as they printed; a sheet whose givens
/// cannot be read is refused (logged).
fn read_sheets(store: &mut dyn ProjectStore, records: &[Sheet]) -> Vec<(u64, SheetRead)> {
    let mut read = Vec::with_capacity(records.len());
    for record in records {
        let printed = match printed(store, record) {
            Ok(printed) => printed,
            Err(error) => {
                log::error!(
                    "the givens of practice sheet {} refused: {error}",
                    record.id
                );
                continue;
            }
        };
        match store.artwork(ProjectId::new(record.canvas)) {
            Ok(artwork) => read.push((record.id, SheetRead::of(&artwork, record, &printed))),
            Err(error) => log::warn!(
                "the sheet of practice record {} not read: {error}",
                record.id
            ),
        }
    }
    read
}

/// What each exercise of the practice sheet on `project` printed (S120d); none when no sheet is
/// on it (an artwork).
///
/// # Errors
/// Whatever reading the records fails with; `Corrupt` for givens missing or unreadable (logged):
/// the sheet is refused, never remade.
pub(crate) fn sheet_printed(
    store: &mut dyn ProjectStore,
    project: ProjectId,
) -> Result<Option<Vec<Given>>, PortError> {
    let Some(sheet) = sheets(store)?
        .into_iter()
        .find(|s| s.canvas == project.get())
    else {
        return Ok(None);
    };
    printed(store, &sheet)
        .inspect_err(|error| {
            log::error!("the givens of practice sheet {} refused: {error}", sheet.id)
        })
        .map(Some)
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
    let sheets: HashSet<u64> = (sheets(store)?.iter())
        .map(|record| record.canvas)
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
        PracticeJob::Start { .. } | PracticeJob::Continue { .. } => PracticeDone::Failed(error),
        PracticeJob::Append { sheet, .. } => PracticeDone::Appended {
            id: sheet.id,
            lessons: sheet.lessons.len(),
            result: Err(error),
        },
        PracticeJob::Save(_)
        | PracticeJob::Trim(_)
        | PracticeJob::Timer(_)
        | PracticeJob::ForgetBlocks(_) => PracticeDone::Saved(Err(error)),
        PracticeJob::Rename(sheet, title) => {
            PracticeDone::Renamed(*sheet, title.clone(), Err(error))
        }
        PracticeJob::Remove(record) => PracticeDone::Removed(record.id, Err(error)),
        PracticeJob::RemovePicture(_) | PracticeJob::PictureTier(..) => {
            PracticeDone::Pictures(Err(error))
        }
        PracticeJob::Read(_) => PracticeDone::Read(Vec::new()),
        PracticeJob::ImportPack(_) => PracticeDone::PackImported(Err(error)),
        PracticeJob::AddPicture(..) => PracticeDone::PictureAdded(Err(error)),
        PracticeJob::Picture(item) => PracticeDone::Picture(item.clone(), None),
    })
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
        "colour copy {} of {}/{}",
        copy.id,
        picture.pack,
        picture.item
    );
    started(log, copy, title)
}

pub(crate) fn seed(seeds: &mut Seeds) -> u64 {
    u64::from(seeds.next()) << 32 | u64::from(seeds.next())
}
