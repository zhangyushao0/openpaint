//! Frames without input on the hand's clock, one 60 Hz frame apart, until the runtime shows what
//! a test waits for: never a count of frames, only `HANG` of wall time as a guard.

use std::time::Instant;

use openpaint_runtime::Runtime;

use super::hand::{HANG, Hand};

/// Milliseconds of one 60 Hz frame.
const FRAME_MS: i64 = 16;

/// One frame a 60 Hz frame on the hand's clock (`Hand::frame_now`); whether the runtime wants
/// another.
pub fn frame(runtime: &mut Runtime, hand: &mut Hand) -> bool {
    hand.tick(runtime, FRAME_MS);
    hand.frame_now(runtime)
}

/// Frames until `ready` holds and the runtime wants no more.
pub fn until(runtime: &mut Runtime, hand: &mut Hand, ready: impl Fn(&Runtime) -> bool) {
    let start = Instant::now();
    while frame(runtime, hand) || !ready(runtime) {
        assert!(start.elapsed() < HANG, "never got there in {HANG:?}");
    }
}

/// Frames until the runtime wants none: nothing moves, fills in or waits for a worker.
pub fn settle(runtime: &mut Runtime, hand: &mut Hand) {
    until(runtime, hand, |runtime| !runtime.moving());
}
