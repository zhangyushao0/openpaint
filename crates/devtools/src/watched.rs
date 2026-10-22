//! `Watched`: the product library as the development tools use it. Behind a switch a scenario
//! flips (`storage_fails`, `storage_works`): while it is off every call fails the way a full or
//! removed disk would, so a walk-through can show the failure notice and its retry. And looked
//! at after each save and each version kept (`after_writes`): a long session reads what the
//! library holds then.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, SaveId, Thumbnail,
    Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_store::Library;

/// What the library just wrote, for the look after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrote {
    /// The artwork was saved.
    Saved(ProjectId),
    /// A version of the artwork was kept.
    Kept(ProjectId),
}

type Look = Box<dyn FnMut(&mut Library, Wrote) + Send>;

pub struct Watched {
    library: Library,
    off: Arc<AtomicBool>,
    look: Option<Look>,
}

impl Watched {
    /// The library in `dir`, failing while `off` is set.
    #[must_use]
    pub fn new(dir: &Path, off: Arc<AtomicBool>) -> Self {
        Self {
            library: Library::at(dir),
            off,
            look: None,
        }
    }

    /// `look` runs on the library after each save and each version kept.
    #[must_use]
    pub fn after_writes(mut self, look: impl FnMut(&mut Library, Wrote) + Send + 'static) -> Self {
        self.look = Some(Box::new(look));
        self
    }

    /// `call` on the library unless the switch is off.
    fn on<T>(
        &mut self,
        call: impl FnOnce(&mut Library) -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        if self.off.load(Ordering::Relaxed) {
            return Err(PortError::Device {
                detail: "storage switched off by the scenario".into(),
            });
        }
        call(&mut self.library)
    }

    fn looked(&mut self, wrote: Wrote) {
        if let Some(look) = &mut self.look {
            look(&mut self.library, wrote);
        }
    }
}

impl ProjectStore for Watched {
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
        let saved = self.on(|library| library.save(id, artwork))?;
        self.looked(Wrote::Saved(id));
        Ok(saved)
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
        self.on(|library| library.keep_version(id, kept_at))?;
        self.looked(Wrote::Kept(id));
        Ok(())
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
