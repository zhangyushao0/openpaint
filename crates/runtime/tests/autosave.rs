//! GPU test of autosave through the whole frame loop: strokes are stored on the worker and a new
//! runtime on the same library opens them with identical pixels; a failing store shows the
//! notice, Retry tapped in its pill tries again and clears it once the store works.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use openpaint_app::{
    PortError, Project, ProjectId, ProjectStore, ProjectSummary, StorageNotice, Thumbnail, Title,
};
use openpaint_core::Artwork;
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use openpaint_store::Library;

/// Wide enough for the notice pill beside the right island.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;
/// Pixel rows through the top edge of a text button's ink outline in the top islands and pills
/// (the island's top, its outline and the button's inset, 13.25 points, give or take a pixel).
const BUTTON_TOP: [usize; 3] = [26, 27, 28];
/// Shortest run of ink along a row that is a button's top edge, in pixels.
const BUTTON_EDGE: usize = 40;

fn gesture(runtime: &mut Runtime, tool: ToolKind, points: &[[f32; 2]]) {
    let last = points.len() - 1;
    for (index, position) in points.iter().enumerate() {
        let phase = match index {
            0 => MotionPhase::Down,
            i if i == last => MotionPhase::Up,
            _ => MotionPhase::Move,
        };
        let sample = PointerSample {
            position: *position,
            pressure: 0.7,
            tilt: 0.0,
            orientation: 0.0,
            time_ms: 8 * index as i64,
        };
        runtime.motion(&MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples: vec![sample],
            }],
        });
        runtime.frame_offscreen(SIZE).expect("frame");
    }
}

/// Right of the side bar, which covers the canvas's left edge.
fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [180.0 + 20.0 * f32::from(i), y])
        .collect()
}

/// A runtime on `store`, with frames until the library has opened the canvas.
fn start(store: Box<dyn ProjectStore>) -> Runtime {
    let mut runtime = Runtime::start(PIXELS_PER_POINT, store)
        .expect("a GPU adapter is required for runtime tests");
    let start = Instant::now();
    while runtime.artwork().is_none() {
        runtime.frame_offscreen(SIZE).expect("frame");
        assert!(start.elapsed() < Duration::from_secs(10), "never opened");
    }
    runtime
}

/// Frames until the runtime stops asking for them (the store finished).
fn settle(runtime: &mut Runtime) {
    let start = Instant::now();
    while runtime.frame_offscreen(SIZE).expect("frame") {
        assert!(start.elapsed() < Duration::from_secs(10), "never settled");
    }
}

fn library_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("runtime-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clear an old library");
    }
    dir
}

#[test]
fn strokes_survive_a_restart_with_identical_pixels() {
    let dir = library_dir("restart");
    let mut runtime = start(Box::new(Library::at(&dir)));
    gesture(&mut runtime, ToolKind::Stylus, &line(160.0));
    gesture(&mut runtime, ToolKind::Stylus, &line(260.0));
    settle(&mut runtime);
    let before = runtime.read_canvas().expect("readback").expect("canvas");
    let parts = runtime.artwork().expect("artwork").to_parts();
    drop(runtime);
    let reopened = start(Box::new(Library::at(&dir)));
    let after = reopened.read_canvas().expect("readback").expect("canvas");
    assert_eq!(reopened.artwork().expect("artwork").to_parts(), parts);
    assert_eq!(after.content_hash(), before.content_hash());
}

/// Fails every call while `fail` is set; keeps the version saved last.
struct Flaky {
    fail: Arc<AtomicBool>,
    saved: Arc<Mutex<Option<Artwork>>>,
}

impl Flaky {
    fn call(&self, artwork: &Artwork) -> Result<(), PortError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(PortError::Device {
                detail: "switched off by the test".into(),
            });
        }
        *self.saved.lock().expect("saved") = Some(artwork.clone());
        Ok(())
    }
}

impl ProjectStore for Flaky {
    fn open_last(&mut self) -> Result<Option<Project>, PortError> {
        Ok(None)
    }

    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError> {
        Ok(Vec::new())
    }

    fn create(&mut self, artwork: &Artwork, _title: &Title) -> Result<ProjectId, PortError> {
        self.call(artwork).map(|()| ProjectId::new(1))
    }

    fn load(&mut self, id: ProjectId) -> Result<Project, PortError> {
        Err(PortError::Missing(id))
    }

    fn save(&mut self, _id: ProjectId, artwork: &Artwork) -> Result<(), PortError> {
        self.call(artwork)
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

    fn editor(&mut self) -> Result<Option<openpaint_app::EditorState>, PortError> {
        Ok(None)
    }

    fn save_editor(&mut self, _state: &openpaint_app::EditorState) -> Result<(), PortError> {
        Ok(())
    }
}

/// The centre of the text button in the right half of the top band (Gallery is at the left),
/// found by the ink of its outline's top edge: the longest dark run along that row.
fn retry_button(runtime: &mut Runtime) -> [f32; 2] {
    let screen = runtime.screenshot(SIZE).expect("screenshot");
    let width = SIZE[0] as usize;
    for y in BUTTON_TOP {
        let row = &screen.pixels()[y * width * 4..(y + 1) * width * 4];
        let (pixels, _) = row.as_chunks::<4>();
        let dark = |x: usize| pixels[x][..3].iter().all(|channel| *channel < 160);
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for x in (width / 2..width).filter(|&x| dark(x)) {
            match runs.last_mut() {
                Some(run) if run.1 + 1 == x => run.1 = x,
                _ => runs.push((x, x)),
            }
        }
        if let Some((first, last)) = runs
            .into_iter()
            .filter(|(first, last)| last - first >= BUTTON_EDGE)
            .max_by_key(|(first, last)| last - first)
        {
            let middle = (13.25 + 24.0) * PIXELS_PER_POINT;
            return [(first + last) as f32 / 2.0, middle];
        }
    }
    panic!("no text button in the top band's right half");
}

#[test]
fn a_failed_save_shows_a_retry_that_saves_once_the_store_works() {
    let fail = Arc::new(AtomicBool::new(true));
    let saved = Arc::new(Mutex::new(None));
    let store = Flaky {
        fail: Arc::clone(&fail),
        saved: Arc::clone(&saved),
    };
    let mut runtime = start(Box::new(store));
    gesture(&mut runtime, ToolKind::Stylus, &line(200.0));
    settle(&mut runtime);
    let failed = StorageNotice::SaveFailed { retrying: false };
    assert_eq!(runtime.projection().storage(), failed);
    let retry = retry_button(&mut runtime);
    gesture(&mut runtime, ToolKind::Finger, &[retry, retry]);
    settle(&mut runtime);
    assert_eq!(runtime.projection().storage(), failed, "still failing");
    fail.store(false, Ordering::Relaxed);
    gesture(&mut runtime, ToolKind::Finger, &[retry, retry]);
    settle(&mut runtime);
    assert_eq!(runtime.projection().storage(), StorageNotice::None);
    let stored = saved.lock().expect("saved").clone().expect("a save");
    assert!(stored.same_version(runtime.artwork().expect("artwork")));
}
