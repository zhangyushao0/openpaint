//! The storage worker: owns the `ProjectStore` for the app's life and serves it off the UI
//! thread. It opens the last project once at start, then writes the versions the session hands
//! out. The first save of a new artwork creates its project; every later one saves over it.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use openpaint_app::{PortError, ProjectId, ProjectStore};
use openpaint_core::Artwork;

use crate::error::RuntimeError;
use crate::worker::Latest;

/// What opening the library found.
#[derive(Debug, Clone)]
pub(crate) enum Opened {
    /// The artwork saved last.
    Last(Artwork),
    /// An empty library: start blank.
    Empty,
    /// The last artwork could not be read; it stays in the library untouched and the app
    /// starts blank, saving into a new project.
    Failed(PortError),
}

enum Job {
    Open,
    Save(Artwork),
}

pub(crate) enum Done {
    Opened(Opened),
    Saved(Artwork, Result<(), PortError>),
}

pub(crate) struct Storage {
    worker: Latest<Job, Done>,
}

impl Storage {
    /// Starts the worker with `store` and asks it to open the last project.
    pub(crate) fn start(mut store: Box<dyn ProjectStore>) -> Result<Self, RuntimeError> {
        let mut project = None;
        let worker = Latest::spawn("openpaint-store", move |job| {
            serve(store.as_mut(), &mut project, job)
        })?;
        worker.submit(Job::Open);
        Ok(Self { worker })
    }

    /// Hands `artwork` to the store. The session hands out one version at a time.
    pub(crate) fn save(&self, artwork: Artwork) {
        self.worker.submit(Job::Save(artwork));
    }

    /// The newest finished call, once.
    pub(crate) fn take(&self) -> Option<Done> {
        self.worker.take()
    }

    /// Waits at most `limit` for the store to finish what it was handed. Returns whether it did.
    pub(crate) fn drain(&self, limit: Duration) -> bool {
        self.worker.drain(limit)
    }
}

/// One call on the worker. A panic in the store is a bug: it is logged and reported as
/// `PortError::Stopped`, and the worker serves the next call.
fn serve(store: &mut dyn ProjectStore, project: &mut Option<ProjectId>, job: Job) -> Done {
    let version = match &job {
        Job::Open => None,
        Job::Save(artwork) => Some(artwork.clone()),
    };
    match catch_unwind(AssertUnwindSafe(|| run(store, project, job))) {
        Ok(done) => done,
        Err(_) => {
            log::error!("the store panicked");
            match version {
                Some(artwork) => Done::Saved(artwork, Err(PortError::Stopped)),
                None => Done::Opened(Opened::Failed(PortError::Stopped)),
            }
        }
    }
}

fn run(store: &mut dyn ProjectStore, project: &mut Option<ProjectId>, job: Job) -> Done {
    match job {
        Job::Open => Done::Opened(match store.open_last() {
            Ok(Some(last)) => {
                *project = Some(last.id);
                Opened::Last(last.artwork)
            }
            Ok(None) => Opened::Empty,
            Err(error) => Opened::Failed(error),
        }),
        Job::Save(artwork) => {
            let result = match *project {
                Some(id) => store.save(id, &artwork),
                None => store.create(&artwork).map(|id| *project = Some(id)),
            };
            Done::Saved(artwork, result)
        }
    }
}
