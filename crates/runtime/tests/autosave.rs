//! GPU test of autosave through the whole frame loop: strokes are stored on the worker and a new
//! runtime on the same library opens them with identical pixels; a failing store shows the
//! notice, Retry tapped on the bar tries again and clears it once the store works.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use openpaint_app::{PortError, Project, ProjectId, ProjectStore, StorageNotice};
use openpaint_core::Artwork;
use openpaint_runtime::{
    Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, Runtime, ToolKind,
};
use openpaint_store::Library;

const SIZE: [u32; 2] = [640, 400];
const PIXELS_PER_POINT: f32 = 2.0;
/// The face of a text button (`design::RAISED`).
const RAISED: [u8; 4] = [52, 52, 52, 255];

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

fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [100.0 + 20.0 * f32::from(i), y])
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

    fn create(&mut self, artwork: &Artwork) -> Result<ProjectId, PortError> {
        self.call(artwork).map(|()| ProjectId::new(1))
    }

    fn save(&mut self, _id: ProjectId, artwork: &Artwork) -> Result<(), PortError> {
        self.call(artwork)
    }
}

/// The centre of the text button in the top bar, found by its face colour.
fn retry_button(runtime: &mut Runtime) -> [f32; 2] {
    let screen = runtime.screenshot(SIZE).expect("screenshot");
    let y = 48;
    let row = &screen.pixels()[y * SIZE[0] as usize * 4..(y + 1) * SIZE[0] as usize * 4];
    let (pixels, _) = row.as_chunks::<4>();
    let face: Vec<usize> = (0..pixels.len()).filter(|&x| pixels[x] == RAISED).collect();
    let (first, last) = (
        face.first().expect("a button"),
        face.last().expect("a button"),
    );
    [(first + last) as f32 / 2.0, y as f32]
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
