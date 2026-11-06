//! Starting the runtime on the test binary's device (`gpu::core`) with a hand on it, until its
//! canvas is open (the library opens on a worker).

use openpaint_app::ProjectStore;
use openpaint_runtime::{Runtime, StartOptions};

use super::frames::until;
use super::hand::Hand;

/// A runtime on `store` and the binary's device, its canvas open on a screen of `size` and the
/// runtime asking for no more frames (the canvas's bar is laid out, so the next tap lands on
/// it), and the hand whose clock it has run on from its first frame.
pub fn start(
    pixels_per_point: f32,
    size: [u32; 2],
    store: Box<dyn ProjectStore>,
) -> (Runtime, Hand) {
    let options = StartOptions {
        core: Some(super::gpu::core()),
        ..StartOptions::default()
    };
    let mut runtime = Runtime::start(pixels_per_point, store, options).expect("the runtime starts");
    let mut hand = Hand::new(size);
    until(&mut runtime, &mut hand, |runtime| {
        runtime.artwork().is_some()
    });
    (runtime, hand)
}
