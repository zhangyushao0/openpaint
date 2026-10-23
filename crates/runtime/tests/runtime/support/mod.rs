//! Shared by the runtime's GPU tests: the device, a hand's gestures and the cold draw, a library
//! that keeps nothing, starting on it, where the canvas shows, a test's own directory, and the
//! few steps many tests take. The binaries of their own (`../budget.rs` and the others) take the
//! files they need alone.

pub mod gpu;
pub mod hand;
pub mod launch;
pub mod screen;
pub mod store;
pub mod test_dir;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use openpaint_app::{GalleryProjection, ScreenProjection};
use openpaint_render::RgbaImage;
use openpaint_runtime::{ImageSink, PortError, Runtime};

pub use gpu::core;
pub use hand::{Hand, assert_cold, cold, on};
pub use launch::start;
pub use screen::{mapping, pixel_at};
pub use store::Nowhere;
pub use test_dir::TestDir;

/// The strokes of every layer.
pub fn strokes(runtime: &Runtime) -> usize {
    let artwork = runtime.artwork().expect("artwork");
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

/// A level pen path of 21 points from x 200 to 1000 at `y`: right of the side bar.
pub fn line(y: f32) -> Vec<[f32; 2]> {
    (0..=20u8)
        .map(|i| [200.0 + 40.0 * f32::from(i), y])
        .collect()
}

/// The canvas as the screen shows it, read back.
pub fn canvas(runtime: &mut Runtime) -> RgbaImage {
    runtime.read_canvas().expect("readback").expect("canvas")
}

/// Frames of `size` until the runtime asks for none and no worker is busy; panics after 30
/// seconds.
pub fn settle(runtime: &mut Runtime, size: [u32; 2]) {
    let start = Instant::now();
    while runtime.frame_offscreen(size).expect("frame") || runtime.storing() {
        assert!(start.elapsed().as_secs() < 30, "never settled");
    }
}

/// Frames of `size` until `ready` holds and the runtime asks for none; panics after 20 seconds.
pub fn until(runtime: &mut Runtime, size: [u32; 2], ready: impl Fn(&Runtime) -> bool) {
    let start = Instant::now();
    while runtime.frame_offscreen(size).expect("frame") || !ready(runtime) {
        assert!(start.elapsed().as_secs() < 20, "never got there");
    }
}

/// A file the system's save dialog gave, in memory: what was written into it last.
#[derive(Clone, Default)]
pub struct Written(Arc<Mutex<Option<Vec<u8>>>>);

impl Written {
    /// The bytes written; panics when nothing was.
    pub fn bytes(&self) -> Vec<u8> {
        let written = self.0.lock().expect("written");
        written.clone().expect("a file written")
    }
}

impl ImageSink for Written {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError> {
        *self.0.lock().expect("written") = Some(bytes.to_vec());
        Ok(())
    }
}

/// The gallery, once it lists the library.
pub fn gallery(runtime: &Runtime) -> Option<GalleryProjection> {
    match runtime.screen() {
        ScreenProjection::Gallery(gallery) if gallery.listed() => Some(gallery),
        _ => None,
    }
}
