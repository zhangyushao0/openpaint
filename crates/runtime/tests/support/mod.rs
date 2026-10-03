//! Shared by the runtime's GPU tests: a store that keeps nothing, and starting the runtime until
//! its canvas is open (the library opens on a worker).

use openpaint_app::{
    EditorState, PortError, Project, ProjectId, ProjectStore, ProjectSummary, Thumbnail, Title,
};
use openpaint_core::Artwork;
use openpaint_runtime::Runtime;

/// A library that is always empty and accepts every write without keeping it.
pub struct Nowhere;

impl ProjectStore for Nowhere {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        Ok(None)
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        Ok(Vec::new())
    }

    fn create(&mut self, _artwork: &Artwork, _title: &Title) -> Result<ProjectId, PortError> {
        Ok(ProjectId::new(1))
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        Err(PortError::Missing(id))
    }

    fn save(&mut self, _id: ProjectId, _artwork: &Artwork) -> Result<(), PortError> {
        Ok(())
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
}

/// Runs frames of `size` until the canvas is open and the runtime asks for no more (the
/// canvas's bar is laid out, so the next tap lands on it); panics after 10 seconds.
pub fn open(runtime: &mut Runtime, size: [u32; 2]) {
    let start = std::time::Instant::now();
    while runtime.frame_offscreen(size).expect("frame") || runtime.artwork().is_none() {
        assert!(
            start.elapsed().as_secs() < 10,
            "the library did not open in time"
        );
    }
}

/// A runtime on `store` with its canvas open.
pub fn start(pixels_per_point: f32, size: [u32; 2], store: Box<dyn ProjectStore>) -> Runtime {
    let mut runtime = Runtime::start(pixels_per_point, store)
        .expect("a GPU adapter is required for runtime tests");
    open(&mut runtime, size);
    runtime
}
