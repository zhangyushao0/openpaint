//! A library for the GPU tests that need none on disk: it holds at most one artwork, the one it
//! opens last, and keeps the version saved last.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Record, SaveId,
    Space, Thumbnail, Title, VersionId,
};
use openpaint_core::Artwork;

/// Lists nothing and opens `last` (none: a new canvas); creating and saving keep the artwork in
/// `saved` and fail, as a full or removed disk would, while `fail` is set; everything else
/// succeeds without keeping anything.
#[derive(Default)]
pub struct Nowhere {
    pub last: Option<Artwork>,
    pub fail: Arc<AtomicBool>,
    pub saved: Arc<Mutex<Option<Artwork>>>,
}

impl Nowhere {
    fn keep(&self, artwork: &Artwork) -> Result<(), PortError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(PortError::Device {
                detail: "switched off by the test".into(),
            });
        }
        *self.saved.lock().expect("saved") = Some(artwork.clone());
        Ok(())
    }
}

impl ProjectStore for Nowhere {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        Ok(self.last.clone().map(|artwork| Project {
            id: ProjectId::new(1),
            title: Title::untitled(),
            artwork,
            saved: SaveId::new(1).expect("a save id"),
            thumbnail: None,
        }))
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        Ok(Vec::new())
    }

    fn create(&mut self, artwork: &Artwork, _title: &Title) -> Result<ProjectId, PortError> {
        self.keep(artwork).map(|()| ProjectId::new(2))
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        Err(PortError::Missing(id))
    }

    fn save(&mut self, _id: ProjectId, artwork: &Artwork) -> Result<SaveId, PortError> {
        self.keep(artwork)
            .map(|()| SaveId::new(2).expect("a save id"))
    }

    fn save_thumbnail(&mut self, _id: ProjectId, _thumbnail: &Thumbnail) -> Result<(), PortError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), PortError> {
        Ok(())
    }

    fn rename(&mut self, _id: ProjectId, _title: &Title) -> Result<(), PortError> {
        Ok(())
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        Err(PortError::Missing(id))
    }

    fn delete(&mut self, _id: ProjectId) -> Result<(), PortError> {
        Ok(())
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        Ok(None)
    }

    fn save_editor(&mut self, _state: &EditorState) -> Result<(), PortError> {
        Ok(())
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        Err(PortError::Missing(id))
    }

    fn import_file(&mut self, _bytes: &[u8]) -> Result<ProjectId, PortError> {
        Err(PortError::NotArtworkFile)
    }

    fn keep_version(&mut self, _id: ProjectId, _kept_at: u64) -> Result<(), PortError> {
        Ok(())
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        _version: VersionId,
        _title: &Title,
    ) -> Result<ProjectId, PortError> {
        Err(PortError::Missing(id))
    }

    fn artwork(&mut self, id: ProjectId) -> Result<Artwork, PortError> {
        Err(PortError::Missing(id))
    }

    fn records(&mut self, _space: Space, _under: &[u8]) -> Result<Vec<Record>, PortError> {
        Ok(Vec::new())
    }

    fn put_records(&mut self, _records: &[(Space, Record)]) -> Result<(), PortError> {
        Ok(())
    }

    fn rewrite_records(
        &mut self,
        _put: &[(Space, Record)],
        _gone: &[(Space, Vec<u8>)],
    ) -> Result<(), PortError> {
        Ok(())
    }

    fn create_owned(
        &mut self,
        (artwork, _title): (&Artwork, &Title),
        _owned: &dyn Fn(ProjectId) -> Vec<(Space, Record)>,
    ) -> Result<ProjectId, PortError> {
        self.keep(artwork).map(|()| ProjectId::new(2))
    }

    fn remove_owned(
        &mut self,
        _keys: &[(Space, Vec<u8>)],
        _projects: &[ProjectId],
    ) -> Result<(), PortError> {
        Ok(())
    }
}
