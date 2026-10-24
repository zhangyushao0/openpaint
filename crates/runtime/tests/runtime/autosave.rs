//! GPU test of autosave through the whole frame loop: strokes are stored on the worker and a new
//! runtime on the same library opens them with identical pixels; a failing store shows the
//! notice, Retry tapped in its pill tries again and clears it once the store works.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use openpaint_app::StorageNotice;
use openpaint_runtime::ToolKind;
use openpaint_store::Library;

use crate::support::{Hand, Nowhere, TestDir, line, settle, start};

/// Wide enough for the notice pill beside the right island.
const SIZE: [u32; 2] = [1600, 1000];
const PIXELS_PER_POINT: f32 = 2.0;

#[test]
fn strokes_survive_a_restart_with_identical_pixels() {
    let dir = TestDir::new("restart");
    let mut runtime = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(160.0));
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(260.0));
    settle(&mut runtime, SIZE);
    let before = runtime.read_canvas().expect("readback").expect("canvas");
    let parts = runtime.artwork().expect("artwork").to_parts();
    drop(runtime);
    let mut reopened = start(PIXELS_PER_POINT, SIZE, Box::new(Library::at(&dir)));
    // The canvas fills in over frames (S53).
    settle(&mut reopened, SIZE);
    let after = reopened.read_canvas().expect("readback").expect("canvas");
    assert_eq!(reopened.artwork().expect("artwork").to_parts(), parts);
    assert_eq!(after.content_hash(), before.content_hash());
}

#[test]
fn a_failed_save_shows_a_retry_that_saves_once_the_store_works() {
    let fail = Arc::new(AtomicBool::new(true));
    let store = Nowhere {
        fail: Arc::clone(&fail),
        ..Nowhere::default()
    };
    let saved = Arc::clone(&store.saved);
    let mut runtime = start(PIXELS_PER_POINT, SIZE, Box::new(store));
    let mut hand = Hand::new(SIZE);
    hand.gesture(&mut runtime, ToolKind::Stylus, &line(200.0));
    settle(&mut runtime, SIZE);
    let failed = StorageNotice::SaveFailed { retrying: false };
    assert_eq!(runtime.projection().storage(), failed);
    hand.tap(&mut runtime, "Retry");
    settle(&mut runtime, SIZE);
    assert_eq!(runtime.projection().storage(), failed, "still failing");
    fail.store(false, Ordering::Relaxed);
    hand.tap(&mut runtime, "Retry");
    settle(&mut runtime, SIZE);
    assert_eq!(runtime.projection().storage(), StorageNotice::None);
    let stored = saved.lock().expect("saved").clone().expect("a save");
    assert!(stored.same_version(runtime.artwork().expect("artwork")));
}
