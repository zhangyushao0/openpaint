//! The storage worker: owns the `ProjectStore` for the app's life and serves it off the UI
//! thread, one call after another in the order the frame loop submitted them. It knows which
//! project is open: the one opened at start, loaded or created from the gallery, or created by
//! the first save of a new artwork; leaving an artwork for the gallery closes it. Leaving an
//! artwork that was saved since it opened or since it was last left keeps a version of it
//! (DECISIONS D63). Opening is `open.rs`, the files the user chooses `files.rs`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use openpaint_app::{
    EditorState, FileJob, LibraryAnswer, PortError, ProjectId, ProjectStore, SaveId, Thumbnail,
    Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_practice::{Pack, Sheet};
use openpaint_practice_store::Packs;
use openpaint_store::DisplayCache;

use crate::error::RuntimeError;
use crate::practice::{self, PracticeDone, PracticeJob};
use crate::worker::Queue;

mod files;
mod open;
pub(crate) use files::read;
pub(crate) use open::{Loaded, Opened};
use open::{loaded, start};

/// What a save stored: the project it went to and the save id its head holds now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stored {
    pub(crate) project: ProjectId,
    pub(crate) saved: SaveId,
}

pub(crate) enum Job {
    /// Opens the library and reads the stored toolbox.
    Start,
    /// The toolbox changed (`Toolbox::save_request`).
    SaveEditor(Box<EditorState>),
    /// A version of the open artwork; the first save of an artwork not in the library yet
    /// creates its project.
    Save(Artwork),
    /// Leaving the open artwork: its `version` if not stored yet, then its thumbnail, then a
    /// kept version dated `kept_at` (seconds since 1970 UTC; none for a practice sheet) if it
    /// changed; with `close`, no artwork is open afterwards. Nothing after a failed save
    /// happens.
    Leave {
        version: Option<Artwork>,
        thumbnail: Option<Thumbnail>,
        kept_at: Option<u64>,
        close: bool,
    },
    /// A new artwork, which becomes the open one.
    Create { artwork: Artwork, title: Title },
    /// A gallery call other than create, which needs the blank artwork from the frame loop.
    Library(LibraryCall),
    /// An OpenPaint file to write or to read, once the system gave it.
    File(FileJob),
    /// A practice session started or its record changed; a colour copy renamed or removed; a
    /// pack imported; one of the user's own pictures added, removed or moved to a tier; a pack's
    /// picture read for the screen.
    Practice(PracticeJob),
}

/// The gallery's calls the store serves as they are.
pub(crate) enum LibraryCall {
    List,
    Load(ProjectId),
    Rename(ProjectId, Title),
    Duplicate(ProjectId),
    Delete(ProjectId),
    Restore(ProjectId, VersionId, Title),
}

pub(crate) enum Done {
    /// What opening found, the stored toolbox (`None` when there is none or it could not be
    /// read, logged here: the brushes keep their defaults), the practice sessions, their sheets'
    /// titles and the packs kept (none when they could not be read, logged).
    Started(Opened, Option<Box<EditorState>>, Practised),
    EditorSaved(Result<(), PortError>),
    /// A version saved, and where to: `None` for the first save of an artwork started blank,
    /// which created its project (its head holds no save id yet).
    Saved(Artwork, Result<Option<Stored>, PortError>),
    Left {
        version: Option<Artwork>,
        result: Result<Option<Stored>, PortError>,
        closed: bool,
    },
    /// An artwork the gallery opened.
    Loaded(Box<Loaded>),
    /// A new artwork the gallery made, now the open one.
    Created(Artwork, Title, ProjectId),
    /// Everything else the gallery asked for, and the failures of the two above.
    Gallery(LibraryAnswer),
    Practice(PracticeDone),
}

/// The practice sheets, their titles and the packs kept, as opening read them.
pub(crate) type Practised = (Vec<Sheet>, Vec<(ProjectId, Title)>, Vec<Pack>);

/// What the worker knows between calls.
#[derive(Debug, Default)]
pub(crate) struct Kept {
    pub(crate) open: Option<ProjectId>,
    /// The open artwork was saved since it opened or since a version of it was last kept.
    pub(crate) changed: bool,
}

pub(crate) struct Storage {
    worker: Queue<Job, Done>,
}

impl Storage {
    /// Starts the worker with `store` and asks it to open the library. An artwork it opens comes
    /// with its canvas from `cache`, if one is kept there; practice packs are kept in `packs`
    /// (none: there are none, and none can be imported).
    pub(crate) fn start(
        mut store: Box<dyn ProjectStore>,
        (cache, packs): (Option<DisplayCache>, Option<Packs>),
    ) -> Result<Self, RuntimeError> {
        let mut kept = Kept::default();
        let mut worker = Queue::spawn("openpaint-store", move |job| {
            serve(
                (store.as_mut(), cache.as_ref(), packs.as_ref()),
                &mut kept,
                job,
            )
        })?;
        worker.submit(Job::Start);
        Ok(Self { worker })
    }

    pub(crate) fn submit(&mut self, job: Job) {
        self.worker.submit(job);
    }

    /// The oldest finished call, once.
    pub(crate) fn take(&mut self) -> Option<Done> {
        self.worker.take()
    }

    /// True while a call is queued or running, or its result waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// Waits at most `limit` for the store to finish what it was handed. Returns whether it did.
    pub(crate) fn drain(&mut self, limit: Duration) -> bool {
        self.worker.drain(limit)
    }
}

/// The store, the display cache and the packs the worker serves from.
type Served<'a> = (
    &'a mut dyn ProjectStore,
    Option<&'a DisplayCache>,
    Option<&'a Packs>,
);

/// One call on the worker. A panic in the store is a bug: it is logged and reported as
/// `PortError::Stopped`, and the worker serves the next call.
fn serve(served: Served<'_>, kept: &mut Kept, job: Job) -> Done {
    let stopped = stopped(&job);
    match catch_unwind(AssertUnwindSafe(|| run(served, kept, job))) {
        Ok(done) => done,
        Err(_) => {
            log::error!("the store panicked");
            stopped
        }
    }
}

/// What `job` answers when the store panicked.
fn stopped(job: &Job) -> Done {
    let error = PortError::Stopped;
    match job {
        Job::Start => Done::Started(Opened::Failed(error), None, Practised::default()),
        Job::SaveEditor(_) => Done::EditorSaved(Err(error)),
        Job::Save(artwork) => Done::Saved(artwork.clone(), Err(error)),
        Job::Leave { version, .. } => Done::Left {
            version: version.clone(),
            result: Err(error),
            closed: false,
        },
        Job::Create { .. } => Done::Gallery(LibraryAnswer::CreateFailed(error)),
        Job::Library(LibraryCall::List) => Done::Gallery(LibraryAnswer::Listed(Err(error))),
        Job::Library(LibraryCall::Load(_)) => Done::Gallery(LibraryAnswer::OpenFailed(error)),
        Job::Library(LibraryCall::Restore(..)) => {
            Done::Gallery(LibraryAnswer::Restored(Err(error)))
        }
        Job::Library(_) => Done::Gallery(LibraryAnswer::Changed(Err(error))),
        Job::File(job) => Done::Gallery(files::stopped(job)),
        Job::Practice(job) => practice::stopped(job),
    }
}

fn run((store, cache, packs): Served<'_>, kept: &mut Kept, job: Job) -> Done {
    match job {
        Job::Start => {
            let packs = (packs.map(Packs::list).transpose())
                .inspect_err(|error| log::warn!("the practice packs are not listed: {error}"))
                .ok()
                .flatten()
                .unwrap_or_default();
            // The session records of 1.21.6 and before become sheets, once (S120c), before a
            // sheet left open reads its givens.
            if let Err(error) = openpaint_practice_store::convert(store, &packs) {
                log::error!("the practice records were not converted: {error}");
            }
            let opened = start(store, kept, cache);
            let editor = store
                .editor()
                .inspect_err(|error| log::warn!("the stored toolbox is unreadable: {error}"))
                .ok()
                .flatten();
            let sheets = openpaint_practice_store::sheets(store)
                .inspect_err(|error| log::warn!("the practice sheets are unreadable: {error}"))
                .unwrap_or_default();
            let titles = practice::gallery_list(store)
                .map(|listed| listed.sheets)
                .inspect_err(|error| log::warn!("the practice sheets' titles are unread: {error}"))
                .unwrap_or_default();
            Done::Started(opened, editor.map(Box::new), (sheets, titles, packs))
        }
        Job::SaveEditor(state) => Done::EditorSaved(store.save_editor(&state)),
        Job::Save(artwork) => {
            let result = save(store, kept, &artwork);
            Done::Saved(artwork, result)
        }
        Job::Leave {
            version,
            thumbnail,
            kept_at,
            close,
        } => leave(store, kept, (version, thumbnail.as_ref()), kept_at, close),
        Job::Create { artwork, title } => match store.create(&artwork, &title) {
            Ok(id) => {
                *kept = Kept {
                    open: Some(id),
                    changed: false,
                };
                Done::Created(artwork, title, id)
            }
            Err(error) => Done::Gallery(LibraryAnswer::CreateFailed(error)),
        },
        Job::Library(request) => library((store, cache), kept, request),
        Job::File(job) => Done::Gallery(files::run(store, job)),
        Job::Practice(job) => practice::run((store, packs), kept, job),
    }
}

fn save(
    store: &mut dyn ProjectStore,
    kept: &mut Kept,
    artwork: &Artwork,
) -> Result<Option<Stored>, PortError> {
    let stored = match kept.open {
        Some(project) => store
            .save(project, artwork)
            .map(|saved| Some(Stored { project, saved })),
        None => store.create(artwork, &Title::untitled()).map(|id| {
            kept.open = Some(id);
            None
        }),
    }?;
    kept.changed = true;
    Ok(stored)
}

fn leave(
    store: &mut dyn ProjectStore,
    kept: &mut Kept,
    (version, thumbnail): (Option<Artwork>, Option<&Thumbnail>),
    kept_at: Option<u64>,
    close: bool,
) -> Done {
    let result = match &version {
        Some(artwork) => save(store, kept, artwork),
        None => Ok(None),
    };
    if result.is_ok() {
        if let (Some(id), Some(thumbnail)) = (kept.open, thumbnail)
            && let Err(error) = store.save_thumbnail(id, thumbnail)
        {
            log::warn!("storing the thumbnail failed: {error}");
        }
        if let (Some(id), Some(kept_at)) = (kept.open.filter(|_| kept.changed), kept_at) {
            match store.keep_version(id, kept_at) {
                Ok(()) => kept.changed = false,
                Err(error) => log::warn!("keeping a version failed: {error}"),
            }
        }
        if close {
            if let Err(error) = store.close() {
                log::warn!("closing the artwork failed: {error}");
            }
            *kept = Kept::default();
        }
    }
    let closed = close && result.is_ok();
    Done::Left {
        version,
        result,
        closed,
    }
}

fn library(
    (store, cache): (&mut dyn ProjectStore, Option<&DisplayCache>),
    kept: &mut Kept,
    call: LibraryCall,
) -> Done {
    Done::Gallery(match call {
        LibraryCall::List => {
            LibraryAnswer::Listed(practice::gallery_list(store).map(|listed| listed.artworks))
        }
        LibraryCall::Load(id) => match (store.load(id)).and_then(|p| loaded((store, cache), p)) {
            Ok(loaded) => {
                *kept = Kept {
                    open: Some(id),
                    changed: false,
                };
                return Done::Loaded(loaded);
            }
            Err(error) => LibraryAnswer::OpenFailed(error),
        },
        LibraryCall::Rename(id, title) => LibraryAnswer::Changed(store.rename(id, &title)),
        LibraryCall::Duplicate(id) => LibraryAnswer::Changed(store.duplicate(id).map(drop)),
        LibraryCall::Delete(id) => LibraryAnswer::Changed(store.delete(id)),
        LibraryCall::Restore(id, version, title) => {
            LibraryAnswer::Restored(store.restore_version(id, version, &title).map(drop))
        }
    })
}
