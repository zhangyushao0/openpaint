//! A library that keeps how many records each of its writes of records held, and refuses any
//! write holding a record `refused` picks, as a full disk would refuse the batch: what a write in
//! two parts would let half through.

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Record, SaveId,
    Space, Thumbnail, Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_store::Library;

pub struct Batches {
    pub library: Library,
    pub refused: fn(&(Space, Record)) -> bool,
    /// Each write of records let through: how many records it held.
    pub writes: Vec<usize>,
}

impl Batches {
    /// `library`, refusing nothing yet.
    pub fn of(library: Library) -> Self {
        Self {
            library,
            refused: |_| false,
            writes: Vec::new(),
        }
    }
}

impl ProjectStore for Batches {
    fn put_records(&mut self, records: &[(Space, Record)]) -> Result<(), PortError> {
        if records.iter().any(self.refused) {
            return Err(PortError::Device {
                detail: "refused by the test".into(),
            });
        }
        self.library.put_records(records)?;
        self.writes.push(records.len());
        Ok(())
    }

    fn records(&mut self, space: Space, under: &[u8]) -> Result<Vec<Record>, PortError> {
        self.library.records(space, under)
    }

    fn create_owned(
        &mut self,
        project: (&Artwork, &Title),
        owned: &dyn Fn(ProjectId) -> Vec<(Space, Record)>,
    ) -> Result<ProjectId, PortError> {
        self.library.create_owned(project, owned)
    }

    fn remove_owned(
        &mut self,
        keys: &[(Space, Vec<u8>)],
        project: Option<ProjectId>,
    ) -> Result<(), PortError> {
        self.library.remove_owned(keys, project)
    }

    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        self.library.open_last()
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        self.library.list()
    }

    fn create(&mut self, artwork: &Artwork, title: &Title) -> Result<ProjectId, PortError> {
        self.library.create(artwork, title)
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        self.library.load(id)
    }

    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<SaveId, PortError> {
        self.library.save(id, artwork)
    }

    fn save_thumbnail(&mut self, id: ProjectId, thumbnail: &Thumbnail) -> Result<(), PortError> {
        self.library.save_thumbnail(id, thumbnail)
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.library.close()
    }

    fn rename(&mut self, id: ProjectId, title: &Title) -> Result<(), PortError> {
        self.library.rename(id, title)
    }

    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError> {
        self.library.duplicate(id)
    }

    fn delete(&mut self, id: ProjectId) -> Result<(), PortError> {
        self.library.delete(id)
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        self.library.editor()
    }

    fn save_editor(&mut self, state: &EditorState) -> Result<(), PortError> {
        self.library.save_editor(state)
    }

    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError> {
        self.library.artwork_file(id)
    }

    fn import_file(&mut self, bytes: &[u8]) -> Result<ProjectId, PortError> {
        self.library.import_file(bytes)
    }

    fn keep_version(&mut self, id: ProjectId, kept_at: u64) -> Result<(), PortError> {
        self.library.keep_version(id, kept_at)
    }

    fn restore_version(
        &mut self,
        id: ProjectId,
        version: VersionId,
        title: &Title,
    ) -> Result<ProjectId, PortError> {
        self.library.restore_version(id, version, title)
    }

    fn artwork(&mut self, id: ProjectId) -> Result<Artwork, PortError> {
        self.library.artwork(id)
    }
}
