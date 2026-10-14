//! `Switched`: the product library behind a switch a scenario flips (`storage_fails`,
//! `storage_works`). While off, every call fails the way a full or removed disk would, so the
//! walk-through can show the failure notice and its retry.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, SaveId, Thumbnail,
    Title, VersionId,
};
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

    /// `call` on the library unless the switch is off.
    fn on<T>(
        &mut self,
        call: impl FnOnce(&mut Library) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        if self.off.load(Ordering::Relaxed) {
            Err(PortError::Device {
                detail: "storage switched off by the scenario".into(),
            })
        } else {
            call(&mut self.library)
        }
    }
}

impl ProjectStore for Switched {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        self.on(Library::open_last)
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        self.on(Library::list)
    }

    fn create(&mut self, artwork: &Artwork, title: &Title) -> Result<ProjectId, PortError> {
        self.on(|library| library.create(artwork, title))
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        self.on(|library| library.load(id))
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<SaveId, PortError> {
        self.on(|library| library.save(id, artwork))
    }

    fn save_thumbnail(&mut self, id: ProjectId, thumbnail: &Thumbnail) -> Result<(), PortError> {
        self.on(|library| library.save_thumbnail(id, thumbnail))
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.on(Library::close)
    }

    fn rename(&mut self, id: ProjectId, title: &Title) -> Result<(), PortError> {
        self.on(|library| library.rename(id, title))
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        self.on(|library| library.duplicate(id))
    }

    fn delete(&mut self, id: ProjectId) -> Result<(), PortError> {
        self.on(|library| library.delete(id))
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        self.on(Library::editor)
    }

    fn save_editor(&mut self, state: &EditorState) -> Result<(), PortError> {
        self.on(|library| library.save_editor(state))
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        self.on(|library| library.artwork_file(id))
    }

    fn import_file(&mut self, bytes: &[u8]) -> Result<ProjectId, PortError> {
        self.on(|library| library.import_file(bytes))
    }

    fn keep_version(&mut self, id: ProjectId, kept_at: u64) -> Result<(), PortError> {
        self.on(|library| library.keep_version(id, kept_at))
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        version: VersionId,
        title: &Title,
    ) -> Result<ProjectId, PortError> {
        self.on(|library| library.restore_version(id, version, title))
    }
}
