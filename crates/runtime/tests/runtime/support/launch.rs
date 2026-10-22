//! Starting the runtime on the test binary's device (`gpu::core`) until its canvas is open (the
//! library opens on a worker).

use openpaint_app::ProjectStore;
use openpaint_runtime::{Runtime, StartOptions};

/// Runs frames of `size` until the canvas is open and the runtime asks for no more (the
/// canvas's bar is laid out, so the next tap lands on it); panics after 10 seconds.
fn open(runtime: &mut Runtime, size: [u32; 2]) {
    let start = std::time::Instant::now();
    while runtime.frame_offscreen(size).expect("frame") || runtime.artwork().is_none() {
        assert!(
            start.elapsed().as_secs() < 10,
            "the library did not open in time"
        );
    }
}

/// A runtime on `store` and the binary's device, its canvas open on a screen of `size`.
pub fn start(pixels_per_point: f32, size: [u32; 2], store: Box<dyn ProjectStore>) -> Runtime {
    let options = StartOptions {
        core: Some(super::gpu::core()),
        ..StartOptions::default()
    };
    let mut runtime = Runtime::start(pixels_per_point, store, options).expect("the runtime starts");
    open(&mut runtime, size);
    runtime
}
