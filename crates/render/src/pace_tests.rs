//! The frame budget's steps: while nothing touches the screen a step gives what keeps the
//! GPU's queue of background work within its most, a first piece whatever it costs only when the
//! queue is empty; while it is touched a frame gives its share, and work another thread submits
//! (the raster worker's tiles) is promised from that share but marked by that thread.

use super::{BACKGROUND_NS, IDLE_BACKGROUND_NS, Pace};
use crate::core::test_core;

const MS: f64 = 1_000_000.0;

#[test]
fn an_idle_step_fills_the_queue_to_its_most_and_no_further() {
    let core = test_core();
    let mut pace = Pace::new(&core.gpu);
    pace.begin(&core.gpu, Some(6.0 * MS));
    assert!(pace.idle());
    assert_eq!(pace.most_queued(), IDLE_BACKGROUND_NS);
    assert!(pace.take_serial(3.0 * MS), "within the 4 ms left");
    assert!(!pace.take_serial(2.0 * MS), "past the queue's most");
    assert_eq!(
        pace.unmarked(),
        3.0 * MS,
        "what was given, to be marked queued"
    );
    assert_eq!(pace.unmarked(), 0.0, "marked once");
    pace.begin(&core.gpu, Some(12.0 * MS));
    assert!(pace.spent(), "a queue over its most takes nothing");
    assert!(!pace.take_serial(0.1 * MS), "not even a first piece");
    pace.begin(&core.gpu, Some(0.0));
    assert!(
        pace.take_serial(30.0 * MS),
        "an empty queue takes a first piece whatever it costs"
    );
    assert!(!pace.take_serial(0.1 * MS));
}

#[test]
fn a_touched_frame_gives_its_share_and_promises_the_workers_tiles_from_it() {
    let core = test_core();
    let mut pace = Pace::new(&core.gpu);
    pace.begin(&core.gpu, Some(0.0));
    assert!(pace.take_serial(30.0 * MS), "an idle step's first piece");
    pace.begin(&core.gpu, None);
    assert!(!pace.idle());
    assert_eq!(pace.share(), BACKGROUND_NS);
    assert_eq!(
        pace.left, BACKGROUND_NS,
        "an idle step's overrun is owed by no frame"
    );
    assert!(
        pace.promise(6.0 * MS),
        "a frame's first tile, whatever it costs"
    );
    assert!(!pace.promise(0.5 * MS), "the share is gone");
    assert_eq!(
        pace.unmarked(),
        0.0,
        "a promised tile is the worker's to mark"
    );
    pace.begin(&core.gpu, None);
    assert_eq!(
        pace.left,
        2.0 * MS,
        "a touched frame's overrun is owed by the next"
    );
}
