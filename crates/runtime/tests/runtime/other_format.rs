//! A library of another format (DECISIONS D95) through the whole frame loop: the app starts in
//! the gallery, says so plainly, opens no canvas and asks the library to write nothing.

use std::sync::{Arc, Mutex};

use openpaint_app::{
    EditorState, GalleryNotice, PortError, Project, ProjectId, ProjectStore, ProjectSummary,
    SaveId, ScreenProjection, Thumbnail, Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_practice::SessionRecord;
use openpaint_runtime::{Runtime, StartOptions};

use crate::support::{core, settle};

const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

/// A library of another format: every call refused, each one that would write counted.
#[derive(Default)]
struct Other {
    writes: Arc<Mutex<Vec<&'static str>>>,
}

fn refused() -> PortError {
    PortError::OtherFormat { found: None }
}

impl Other {
    fn wrote<T>(&self, call: &'static str) -> Result<T, PortError> {
        self.writes.lock().expect("calls").push(call);
        Err(refused())
    }
}

impl ProjectStore for Other {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        Err(refused())
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        Err(refused())
    }

    fn create(&mut self, _artwork: &Artwork, _title: &Title) -> Result<ProjectId, PortError> {
        self.wrote("create")
    }

    fn load(&mut self, _id: ProjectId) -> Result<Project, PortError> {
        Err(refused())
    }

    fn save(&mut self, _id: ProjectId, _artwork: &Artwork) -> Result<SaveId, PortError> {
        self.wrote("save")
    }

    fn save_thumbnail(&mut self, _id: ProjectId, _thumbnail: &Thumbnail) -> Result<(), PortError> {
        self.wrote("thumbnail")
    }

    fn close(&mut self) -> Result<(), PortError> {
        self.wrote("close")
    }

    fn rename(&mut self, _id: ProjectId, _title: &Title) -> Result<(), PortError> {
        self.wrote("rename")
    }

    fn duplicate(&mut self, _id: ProjectId) -> Result<ProjectId, PortError> {
        self.wrote("duplicate")
    }

    fn delete(&mut self, _id: ProjectId) -> Result<(), PortError> {
        self.wrote("delete")
    }

    fn editor(&mut self) -> Result<Option<EditorState>, PortError> {
        Err(refused())
    }

    fn save_editor(&mut self, _state: &EditorState) -> Result<(), PortError> {
        self.wrote("toolbox")
    }

    fn artwork_file(&mut self, _id: ProjectId) -> Result<Vec<u8>, PortError> {
        Err(refused())
    }

    fn import_file(&mut self, _bytes: &[u8]) -> Result<ProjectId, PortError> {
        self.wrote("import")
    }

    fn keep_version(&mut self, _id: ProjectId, _kept_at: u64) -> Result<(), PortError> {
        self.wrote("version")
    }

    fn restore_version(
        &mut self,
        _id: ProjectId,
        _version: VersionId,
        _title: &Title,
    ) -> Result<ProjectId, PortError> {
        self.wrote("restore")
    }

    fn practice(&mut self) -> Result<Vec<SessionRecord>, PortError> {
        Err(refused())
    }

    fn sheet_titles(&mut self) -> Result<Vec<(ProjectId, Title)>, PortError> {
        Err(refused())
    }

    fn start_practice(
        &mut self,
        _sheet: &Artwork,
        _title: &Title,
        _session: &SessionRecord,
    ) -> Result<ProjectId, PortError> {
        self.wrote("practice")
    }

    fn save_practice(&mut self, _session: &SessionRecord) -> Result<(), PortError> {
        self.wrote("practice record")
    }

    fn remove_practice(&mut self, _session: &SessionRecord) -> Result<(), PortError> {
        self.wrote("practice removal")
    }
}

#[test]
fn a_library_of_another_format_starts_the_app_in_the_gallery_saying_so_and_writes_nothing() {
    let store = Other::default();
    let writes = Arc::clone(&store.writes);
    let options = StartOptions {
        core: Some(core()),
        ..StartOptions::default()
    };
    let mut runtime = Runtime::start(PIXELS_PER_POINT, Box::new(store), options).expect("starts");
    settle(&mut runtime, SIZE);
    let ScreenProjection::Gallery(gallery) = runtime.screen() else {
        panic!("not the gallery: {:?}", runtime.screen());
    };
    assert_eq!(gallery.notice(), GalleryNotice::OtherLibrary);
    assert!(gallery.items().is_empty());
    assert!(runtime.artwork().is_none(), "no canvas opened");
    runtime.leave();
    settle(&mut runtime, SIZE);
    assert!(writes.lock().expect("calls").is_empty(), "{writes:?}");
}
