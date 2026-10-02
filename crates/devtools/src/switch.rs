//! `Switched`: the product library behind a switch a scenario flips (`storage_fails`,
//! `storage_works`). While off, every call fails the way a full or removed disk would, so the
//! walk-through can show the failure notice and its retry.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use openpaint_app::{PortError, Project, ProjectId, ProjectStore};
use openpaint_core::Artwork;
use openpaint_store::Library;

pub(crate) struct Switched {
    library: Library,
    off: Arc<AtomicBool>,
}

impl Switched {
    /// The library in `dir`, failing while `off` is set.
    pub(crate) fn new(dir: &Path, off: Arc<AtomicBool>) -> Self {
        Self {
            library: Library::at(dir),
            off,
        }
    }

    fn check(&self) -> Result<(), PortError> {
        if self.off.load(Ordering::Relaxed) {
            Err(PortError::Device {
                detail: "storage switched off by the scenario".into(),
            })
        } else {
            Ok(())
        }
    }
}

impl ProjectStore for Switched {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        self.check()?;
        self.library.open_last()
    }

    fn create(&mut self, artwork: &Artwork) -> Result<ProjectId, PortError> {
        self.check()?;
        self.library.create(artwork)
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<(), PortError> {
        self.check()?;
        self.library.save(id, artwork)
    }
}
