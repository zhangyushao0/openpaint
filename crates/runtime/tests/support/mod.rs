//! Shared by the runtime's GPU tests: a store that keeps nothing, and starting the runtime until
//! its canvas is open (the library opens on a worker).

use openpaint_app::{PortError, Project, ProjectId, ProjectStore};
use openpaint_core::Artwork;
use openpaint_runtime::Runtime;

/// A library that is always empty and accepts every save without keeping it.
pub struct Nowhere;

impl ProjectStore for Nowhere {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        Ok(None)
    }

    fn create(&mut self, _artwork: &Artwork) -> Result<ProjectId, PortError> {
        Ok(ProjectId::new(1))
    }

    fn save(&mut self, _id: ProjectId, _artwork: &Artwork) -> Result<(), PortError> {
        Ok(())
    }
}

/// Runs frames of `size` until the canvas is open; panics after 10 seconds.
pub fn open(runtime: &mut Runtime, size: [u32; 2]) {
    let start = std::time::Instant::now();
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(size).expect("frame");
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
