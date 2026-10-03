//! The storage worker: owns the `ProjectStore` for the app's life and serves it off the UI
//! thread, one call after another in the order the frame loop submitted them. It knows which
//! project is open: the one opened at start, loaded or created from the gallery, or created by
//! the first save of a new artwork; leaving an artwork for the gallery closes it.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use openpaint_app::{
    PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail, Title,
};
use openpaint_core::Artwork;

use crate::error::RuntimeError;
use crate::worker::Queue;

/// What opening the library found.
#[derive(Debug, Clone)]
pub(crate) enum Opened {
    /// The artwork open when the app was left.
    Last(Artwork),
    /// No artwork was open: the user was in the gallery, which lists these.
    Gallery(Result<Vec<ProjectSummary>, PortError>),
    /// An empty library: start blank.
    Empty,
    /// The last artwork could not be read; it stays in the library untouched and the app
    /// starts blank, saving into a new project.
    Failed(PortError),
}

pub(crate) enum Job {
    Start,
    /// A version of the open artwork; the first save of an artwork not in the library yet
    /// creates its project.
    Save(Artwork),
    /// Leaving the open artwork: its `version` if not stored yet, then its thumbnail; with
    /// `close`, no artwork is open afterwards. Nothing after a failed save happens.
    Leave {
        version: Option<Artwork>,
        thumbnail: Option<Thumbnail>,
        close: bool,
    },
    /// A new artwork, which becomes the open one.
    Create {
        artwork: Artwork,
        title: Title,
    },
    /// A gallery call other than create, which needs the blank artwork from the frame loop.
    Library(LibraryCall),
}

/// The gallery's calls the store serves as they are.
pub(crate) enum LibraryCall {
    List,
    Load(ProjectId),
    Rename(ProjectId, Title),
    Duplicate(ProjectId),
    Delete(ProjectId),
}

pub(crate) enum Done {
    Started(Opened),
    Saved(Artwork, Result<(), PortError>),
    Left {
        version: Option<Artwork>,
        result: Result<(), PortError>,
        closed: bool,
    },
    Listed(Result<Vec<ProjectSummary>, PortError>),
    Loaded(Result<Project, PortError>),
    Created(Artwork, Result<ProjectId, PortError>),
    Changed(Result<(), PortError>),
}

pub(crate) struct Storage {
    worker: Queue<Job, Done>,
}

impl Storage {
    /// Starts the worker with `store` and asks it to open the library.
    pub(crate) fn start(mut store: Box<dyn ProjectStore>) -> Result<Self, RuntimeError> {
        let mut open = None;
        let mut worker = Queue::spawn("openpaint-store", move |job| {
            serve(store.as_mut(), &mut open, job)
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

/// One call on the worker. A panic in the store is a bug: it is logged and reported as
/// `PortError::Stopped`, and the worker serves the next call.
fn serve(store: &mut dyn ProjectStore, open: &mut Option<ProjectId>, job: Job) -> Done {
    let stopped = stopped(&job);
    match catch_unwind(AssertUnwindSafe(|| run(store, open, job))) {
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
        Job::Start => Done::Started(Opened::Failed(error)),
        Job::Save(artwork) => Done::Saved(artwork.clone(), Err(error)),
        Job::Leave { version, .. } => Done::Left {
            version: version.clone(),
            result: Err(error),
            closed: false,
        },
        Job::Create { artwork, .. } => Done::Created(artwork.clone(), Err(error)),
        Job::Library(LibraryCall::List) => Done::Listed(Err(error)),
        Job::Library(LibraryCall::Load(_)) => Done::Loaded(Err(error)),
        Job::Library(_) => Done::Changed(Err(error)),
    }
}

fn run(store: &mut dyn ProjectStore, open: &mut Option<ProjectId>, job: Job) -> Done {
    match job {
        Job::Start => Done::Started(start(store, open)),
        Job::Save(artwork) => {
            let result = save(store, open, &artwork);
            Done::Saved(artwork, result)
        }
        Job::Leave {
            version,
            thumbnail,
            close,
        } => leave(store, open, version, thumbnail.as_ref(), close),
        Job::Create { artwork, title } => {
            let result = store.create(&artwork, &title);
            *open = result.as_ref().ok().copied().or(*open);
            Done::Created(artwork, result)
        }
        Job::Library(request) => library(store, open, request),
    }
}

fn start(store: &mut dyn ProjectStore, open: &mut Option<ProjectId>) -> Opened {
    match store.open_last() {
        Ok(Some(last)) => {
            *open = Some(last.id);
            Opened::Last(last.artwork)
        }
        Ok(None) => match store.list() {
            Ok(items) if items.is_empty() => Opened::Empty,
            listed => Opened::Gallery(listed),
        },
        Err(error) => Opened::Failed(error),
    }
}

fn save(
    store: &mut dyn ProjectStore,
    open: &mut Option<ProjectId>,
    artwork: &Artwork,
) -> Result<(), PortError> {
    match *open {
        Some(id) => store.save(id, artwork),
        None => store
            .create(artwork, &Title::untitled())
            .map(|id| *open = Some(id)),
    }
}

fn leave(
    store: &mut dyn ProjectStore,
    open: &mut Option<ProjectId>,
    version: Option<Artwork>,
    thumbnail: Option<&Thumbnail>,
    close: bool,
) -> Done {
    let result = match &version {
        Some(artwork) => save(store, open, artwork),
        None => Ok(()),
    };
    if result.is_ok() {
        if let (Some(id), Some(thumbnail)) = (*open, thumbnail)
            && let Err(error) = store.save_thumbnail(id, thumbnail)
        {
            log::warn!("storing the thumbnail failed: {error}");
        }
        if close {
            if let Err(error) = store.close() {
                log::warn!("closing the artwork failed: {error}");
            }
            *open = None;
        }
    }
    let closed = close && result.is_ok();
    Done::Left {
        version,
        result,
        closed,
    }
}

fn library(store: &mut dyn ProjectStore, open: &mut Option<ProjectId>, call: LibraryCall) -> Done {
    match call {
        LibraryCall::List => Done::Listed(store.list()),
        LibraryCall::Load(id) => {
            let loaded = store.load(id);
            if loaded.is_ok() {
                *open = Some(id);
            }
            Done::Loaded(loaded)
        }
        LibraryCall::Rename(id, title) => Done::Changed(store.rename(id, &title)),
        LibraryCall::Duplicate(id) => Done::Changed(store.duplicate(id).map(|_| ())),
        LibraryCall::Delete(id) => Done::Changed(store.delete(id)),
    }
}
