//! The break with the user's tablet in hand: the app hidden a moment (a system dialog, the screen
//! off, a launch of the app on top of itself) during the break.

use openpaint_practice_app::{PracticeLog, TimerIntent};

use super::block::{begun, ended, frame};
use super::{NOON, VIEWPORT, tick};

/// The app hidden at `now`, as the runtime's `leave` hides it.
fn hidden(log: &mut PracticeLog, now: u64) {
    log.set_clock(now, 0);
    log.hidden(now);
}

#[test]
fn the_app_hidden_during_the_break_keeps_the_blocks_end_and_holds_the_break_to_its_chime() {
    let (mut log, mut session) = begun();
    frame(&mut log, &session, NOON + 1500);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    let end = ended(&log);
    frame(&mut log, &session, NOON + 1505);
    // Away for ten minutes and back on the sheet: the end as it was, the break held.
    hidden(&mut log, NOON + 1505);
    assert!(!log.screen_on(), "nothing counts while hidden");
    frame(&mut log, &session, NOON + 2105);
    assert_eq!(ended(&log), end, "the block's end kept");
    assert!(log.screen_on(), "the break counts again");
    frame(&mut log, &session, NOON + 2105 + 294);
    assert_eq!((ended(&log).2, log.take_chime()), (Some(1), false));
    frame(&mut log, &session, NOON + 2105 + 295);
    assert_eq!(ended(&log).2, None, "the ring complete");
    assert!(log.take_chime(), "its chime");
    assert_eq!(ended(&log).1, end.1, "nothing else changes");
    // The home leaves the end.
    tick(&mut log, NOON + 3000, None);
    assert_eq!(log.timer().block, None);
}
