//! Opening on the storage worker (`mod.rs`): what the library opened at start, and an artwork
//! with its cached canvas and, for a practice sheet, what its exercises printed (S120d).

use std::time::Instant;

use openpaint_app::{PortError, Project, ProjectStore, ProjectSummary};
use openpaint_practice::Given;
use openpaint_store::{Cached, DisplayCache};

use super::Kept;
use crate::practice;

/// An artwork the library opened, with its cached canvas if the display cache holds one and, for
/// a practice sheet, what its exercises printed.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) project: Project,
    pub(crate) cached: Option<Cached>,
    pub(crate) printed: Option<Vec<Given>>,
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

/// `project` with its cached canvas, if `cache` holds one, and a practice sheet's givens from
/// `store`; a cache that cannot be read is logged and left out (derived data: the canvas is
/// evaluated); givens that cannot be read refuse the sheet.
pub(crate) fn loaded(
    (store, cache): (&mut dyn ProjectStore, Option<&DisplayCache>),
    project: Project,
) -> Result<Box<Loaded>, PortError> {
    let printed = practice::sheet_printed(store, project.id)?;
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
    Ok(Box::new(Loaded {
        project,
        cached,
        printed,
    }))
}

/// What opening the library finds: the artwork open when the app was left (it becomes the open
/// one), else the gallery's list.
pub(super) fn start(
    store: &mut dyn ProjectStore,
    kept: &mut Kept,
    cache: Option<&DisplayCache>,
) -> Opened {
    match store.open_last() {
        Ok(Some(last)) => match loaded((store, cache), last) {
            Ok(loaded) => {
                kept.open = Some(loaded.project.id);
                Opened::Last(loaded)
            }
            Err(error) => Opened::Failed(error),
        },
        Ok(None) => match practice::gallery_list(store).map(|listed| listed.artworks) {
            Ok(items) if items.is_empty() => Opened::Empty,
            listed => Opened::Gallery(listed),
        },
        // A library of another format: the gallery says so; nothing is written to it.
        Err(error @ PortError::OtherFormat { .. }) => Opened::Gallery(Err(error)),
        Err(error) => Opened::Failed(error),
    }
}
