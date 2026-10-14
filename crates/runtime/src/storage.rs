//! The storage worker: owns the `ProjectStore` for the app's life and serves it off the UI
//! thread, one call after another in the order the frame loop submitted them. It knows which
//! project is open: the one opened at start, loaded or created from the gallery, or created by
//! the first save of a new artwork; leaving an artwork for the gallery closes it. Leaving an
//! artwork that was saved since it opened or since it was last left keeps a version of it
//! (DECISIONS D63).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use openpaint_app::{
    EditorState, FileJob, MAX_ARTWORK_FILE_BYTES, PortError, Project, ProjectId, ProjectStore,
    ProjectSummary, SaveId, Thumbnail, Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_store::{Cached, DisplayCache};

use crate::error::RuntimeError;
use crate::worker::Queue;

/// An artwork the library opened, with its cached canvas if the display cache holds one (S54).
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) project: Project,
    pub(crate) cached: Option<Cached>,
}

/// What a save stored: the project it went to and the save id its head holds now (S54).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stored {
    pub(crate) project: ProjectId,
    pub(crate) saved: SaveId,
}

/// What opening the library found.
#[derive(Debug)]
pub(crate) enum Opened {
    /// The artwork open when the app was left.
    Last(Box<Loaded>),
    /// No artwork was open: the user was in the gallery, which lists these.
    Gallery(Result<Vec<ProjectSummary>, PortError>),
    /// An empty library: start blank.
    Empty,
    /// The last artwork could not be read; it stays in the library untouched and the app
    /// starts blank, saving into a new project.
    Failed(PortError),
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
    /// kept version dated `kept_at` (seconds since 1970 UTC) if it changed; with `close`, no
    /// artwork is open afterwards. Nothing after a failed save happens.
    Leave {
        version: Option<Artwork>,
        thumbnail: Option<Thumbnail>,
        kept_at: u64,
        close: bool,
    },
    /// A new artwork, which becomes the open one.
    Create { artwork: Artwork, title: Title },
    /// A gallery call other than create, which needs the blank artwork from the frame loop.
    Library(LibraryCall),
    /// An OpenPaint file to write or to read, once the system gave it.
    File(FileJob),
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
    /// What opening found, and the stored toolbox; `None` when there is none or it could not
    /// be read (logged here): the brushes keep their defaults.
    Started(Opened, Option<Box<EditorState>>),
    EditorSaved(Result<(), PortError>),
    /// A version saved, and where to: `None` for the first save of an artwork started blank,
    /// which created its project (its head holds no save id yet).
    Saved(Artwork, Result<Option<Stored>, PortError>),
    Left {
        version: Option<Artwork>,
        result: Result<Option<Stored>, PortError>,
        closed: bool,
    },
    Listed(Result<Vec<ProjectSummary>, PortError>),
    Loaded(Result<Box<Loaded>, PortError>),
    Created(Artwork, Title, Result<ProjectId, PortError>),
    Changed(Result<(), PortError>),
    Restored(Result<(), PortError>),
    FileSaved(Result<(), PortError>),
    FileOpened(Result<(), PortError>),
}

/// What the worker knows between calls.
#[derive(Debug, Default)]
struct Kept {
    open: Option<ProjectId>,
    /// The open artwork was saved since it opened or since a version of it was last kept.
    changed: bool,
}

pub(crate) struct Storage {
    worker: Queue<Job, Done>,
}

impl Storage {
    /// Starts the worker with `store` and asks it to open the library. An artwork it opens comes
    /// with its canvas from `cache` (S54), if one is kept there.
    pub(crate) fn start(
        mut store: Box<dyn ProjectStore>,
        cache: Option<DisplayCache>,
    ) -> Result<Self, RuntimeError> {
        let mut kept = Kept::default();
        let mut worker = Queue::spawn("openpaint-store", move |job| {
            serve((store.as_mut(), cache.as_ref()), &mut kept, job)
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

/// The store and the display cache the worker serves from.
type Served<'a> = (&'a mut dyn ProjectStore, Option<&'a DisplayCache>);

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
        Job::Start => Done::Started(Opened::Failed(error), None),
        Job::SaveEditor(_) => Done::EditorSaved(Err(error)),
        Job::Save(artwork) => Done::Saved(artwork.clone(), Err(error)),
        Job::Leave { version, .. } => Done::Left {
            version: version.clone(),
            result: Err(error),
            closed: false,
        },
        Job::Create { artwork, title } => Done::Created(artwork.clone(), title.clone(), Err(error)),
        Job::Library(LibraryCall::List) => Done::Listed(Err(error)),
        Job::Library(LibraryCall::Load(_)) => Done::Loaded(Err(error)),
        Job::Library(LibraryCall::Restore(..)) => Done::Restored(Err(error)),
        Job::Library(_) => Done::Changed(Err(error)),
        Job::File(FileJob::Save { .. }) => Done::FileSaved(Err(error)),
        Job::File(FileJob::Open { .. }) => Done::FileOpened(Err(error)),
    }
}

fn run((store, cache): Served<'_>, kept: &mut Kept, job: Job) -> Done {
    match job {
        Job::Start => {
            let opened = start(store, kept, cache);
            let editor = store
                .editor()
                .inspect_err(|error| log::warn!("the stored toolbox is unreadable: {error}"))
                .ok()
                .flatten();
            Done::Started(opened, editor.map(Box::new))
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
        Job::Create { artwork, title } => {
            let result = store.create(&artwork, &title);
            if let Ok(id) = result {
                *kept = Kept {
                    open: Some(id),
                    changed: false,
                };
            }
            Done::Created(artwork, title, result)
        }
        Job::Library(request) => library((store, cache), kept, request),
        Job::File(job) => file(store, job),
    }
}

/// `project` with its cached canvas, if `cache` holds one; a cache that cannot be read is
/// logged and left out (derived data: the canvas is evaluated).
fn loaded(cache: Option<&DisplayCache>, project: Project) -> Box<Loaded> {
    let started = Instant::now();
    let cached = cache.and_then(|cache| {
        cache
            .read(project.id)
            .inspect_err(|error| log::warn!("the display cache is not read: {error}"))
            .ok()
            .flatten()
    });
    if let Some(cached) = &cached {
        log::info!(
            "display cache of project {} read in {} ms",
            project.id.get(),
            started.elapsed().as_millis()
        );
        log::debug!("{cached:?}");
    }
    Box::new(Loaded { project, cached })
}

fn start(store: &mut dyn ProjectStore, kept: &mut Kept, cache: Option<&DisplayCache>) -> Opened {
    match store.open_last() {
        Ok(Some(last)) => {
            kept.open = Some(last.id);
            Opened::Last(loaded(cache, last))
        }
        Ok(None) => match store.list() {
            Ok(items) if items.is_empty() => Opened::Empty,
            listed => Opened::Gallery(listed),
        },
        // S53 (author, 2026-10-13): its card says so in the gallery.
        Err(PortError::NotUpgraded { schema }) => {
            log::warn!("the last artwork needs repair (schema {schema}): starting in the gallery");
            Opened::Gallery(store.list())
        }
        Err(error) => Opened::Failed(error),
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
    kept_at: u64,
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
        if let Some(id) = kept.open.filter(|_| kept.changed) {
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

fn library((store, cache): Served<'_>, kept: &mut Kept, call: LibraryCall) -> Done {
    match call {
        LibraryCall::List => Done::Listed(store.list()),
        LibraryCall::Load(id) => {
            let project = store.load(id);
            if project.is_ok() {
                *kept = Kept {
                    open: Some(id),
                    changed: false,
                };
            }
            Done::Loaded(project.map(|project| loaded(cache, project)))
        }
        LibraryCall::Rename(id, title) => Done::Changed(store.rename(id, &title)),
        LibraryCall::Duplicate(id) => Done::Changed(store.duplicate(id).map(|_| ())),
        LibraryCall::Delete(id) => Done::Changed(store.delete(id)),
        LibraryCall::Restore(id, version, title) => {
            Done::Restored(store.restore_version(id, version, &title).map(|_| ()))
        }
    }
}

/// Writes an artwork into the file the user chose, or adds the chosen file as a new artwork.
fn file(store: &mut dyn ProjectStore, job: FileJob) -> Done {
    match job {
        FileJob::Save { id, sink } => {
            let result = store.artwork_file(id).and_then(|bytes| sink.write(&bytes));
            if let Err(error) = &result {
                log::error!("saving an OpenPaint file failed: {error}");
            }
            Done::FileSaved(result)
        }
        FileJob::Open { source } => {
            let result = source.read(MAX_ARTWORK_FILE_BYTES).and_then(|bytes| {
                if bytes.len() as u64 > MAX_ARTWORK_FILE_BYTES {
                    return Err(PortError::Device {
                        detail: format!("larger than {MAX_ARTWORK_FILE_BYTES} bytes"),
                    });
                }
                store.import_file(&bytes).map(|_| ())
            });
            if let Err(error) = &result {
                log::warn!("the chosen file was not opened: {error}");
            }
            Done::FileOpened(result)
        }
    }
}
