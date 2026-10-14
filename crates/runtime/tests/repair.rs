//! S50: an artwork whose records could not be upgraded, through the whole frame loop. The
//! gallery lists it with its picture marked; opening it shows the notice that it needs repair,
//! with OK; its options still offer to save it as an OpenPaint file, which writes its bytes.

mod gpu;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use openpaint_app::{
    EditorState, GalleryNotice, HostAnswer, HostRequest, ImageSink, PortError, Project, ProjectId,
    ProjectStore, ProjectSummary, SaveId, ScreenProjection, Thumbnail, Title, VersionId,
};
use openpaint_core::Artwork;
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};

const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// The broken artwork's bytes as its file.
const FILE: &[u8] = b"OpenPaint\0 the records as they were";

/// A library of one artwork that needs repair: listed, refusing to open, its file written; with
/// `last`, the app was left on it.
struct Broken {
    last: bool,
}

fn refused() -> PortError {
    PortError::NotUpgraded { schema: 21 }
}

impl ProjectStore for Broken {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        if self.last {
            return Err(refused());
        }
        Ok(None)
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        Ok(vec![ProjectSummary {
            id: ProjectId::new(7),
            title: Title::new("Harbour").expect("title"),
            thumbnail: Thumbnail::new([4, 3], vec![90; 48]),
            versions: Vec::new(),
            needs_repair: true,
        }])
    }

    fn create(&mut self, _artwork: &Artwork, _title: &Title) -> Result<ProjectId, PortError> {
        Err(refused())
    }

    fn load(&mut self, _id: ProjectId) -> Result<Project, PortError> {
        Err(refused())
    }

    fn save(&mut self, _id: ProjectId, _artwork: &Artwork) -> Result<SaveId, PortError> {
        Err(refused())
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

    fn artwork_file(&mut self, _id: ProjectId) -> Result<Vec<u8>, PortError> {
        Ok(FILE.to_vec())
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
}

/// Where the saved file goes: the bytes written, kept for the test.
struct Sink(Arc<Mutex<Vec<u8>>>);

impl ImageSink for Sink {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError> {
        self.0.lock().expect("written").extend_from_slice(bytes);
        Ok(())
    }
}

/// Frames until the runtime asks for no more and nothing waits; panics after 10 seconds.
fn settle(runtime: &mut Runtime) {
    let start = Instant::now();
    while runtime.frame_offscreen(SIZE).expect("frame") || runtime.storing() {
        assert!(start.elapsed().as_secs() < 10, "never settled");
    }
}

/// A finger tap on the middle of the control called `name`, then frames until it rests.
fn tap(runtime: &mut Runtime, name: &str) {
    let control = runtime.controls().find(name);
    let at = control
        .unwrap_or_else(|why| panic!("{name}: {why:?}"))
        .at([0.5, 0.5]);
    for (phase, time_ms) in [(MotionPhase::Down, 0), (MotionPhase::Up, 60)] {
        runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool: ToolKind::Finger,
                samples: vec![PointerSample {
                    position: at,
                    pressure: 1.0,
                    tilt: 0.0,
                    orientation: 0.0,
                    time_ms,
                }],
            }],
        });
        runtime.frame_offscreen(SIZE).expect("frame");
    }
    settle(runtime);
}

fn notice(runtime: &Runtime) -> GalleryNotice {
    match runtime.screen() {
        ScreenProjection::Gallery(gallery) => gallery.notice(),
        _ => panic!("not the gallery"),
    }
}

/// S53 (author, 2026-10-13): left on an artwork that needs repair, the app starts in the gallery,
/// its card there marked, not on a blank canvas.
#[test]
fn a_last_artwork_needing_repair_starts_the_app_in_the_gallery() {
    let store = Box::new(Broken { last: true });
    let mut runtime = Runtime::start_on(gpu::core(), PIXELS_PER_POINT, store).expect("starts");
    settle(&mut runtime);
    let ScreenProjection::Gallery(gallery) = runtime.screen() else {
        panic!("not the gallery: {:?}", runtime.screen());
    };
    assert!(gallery.items().iter().any(|item| item.needs_repair));
    assert_eq!(gallery.notice(), GalleryNotice::None);
    assert!(runtime.artwork().is_none(), "no canvas opened");
}

#[test]
fn an_artwork_needing_repair_says_so_and_still_saves_as_a_file() {
    let store = Box::new(Broken { last: false });
    let mut runtime = Runtime::start_on(gpu::core(), PIXELS_PER_POINT, store).expect("starts");
    settle(&mut runtime);
    tap(&mut runtime, "Open artwork");
    assert_eq!(notice(&runtime), GalleryNotice::NeedsRepair);
    assert!(runtime.artwork().is_none(), "it does not open");
    tap(&mut runtime, "OK");
    assert_eq!(notice(&runtime), GalleryNotice::None);
    tap(&mut runtime, "Artwork options");
    tap(&mut runtime, "Save as OpenPaint file…");
    let asked = runtime.host_request();
    assert!(
        matches!(asked, Some(HostRequest::SaveArtwork { .. })),
        "the system's save dialog: {asked:?}"
    );
    let written = Arc::new(Mutex::new(Vec::new()));
    let sink = Box::new(Sink(Arc::clone(&written)));
    runtime.answer(HostAnswer::File(sink));
    settle(&mut runtime);
    assert_eq!(
        *written.lock().expect("written"),
        FILE,
        "its bytes as they were"
    );
    assert_eq!(notice(&runtime), GalleryNotice::FileSaved);
}
