//! The time's end and the break with the user's tablet in hand: the screen kept on at Time until
//! End, the app hidden a moment (a system dialog, the screen off, a launch of the app on top of
//! itself) during the break, and what the timer tells the app's log at each transition.

use openpaint_practice_app::{BlockShown, PracticeLog, TimerIntent, TimerTold};

use super::block::{begun, ended, frame, untouched};
use super::{NOON, VIEWPORT, next, tick};

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

#[test]
fn the_screen_stays_on_at_time_on_the_sheet_until_end() {
    let (mut log, mut session) = begun();
    frame(&mut log, &session, NOON + 1500);
    assert_eq!(log.timer().block, Some(BlockShown::Up));
    assert_eq!(
        (log.screen_on(), log.wake()),
        (true, None),
        "the exercise being finished"
    );
    tick(&mut log, NOON + 1510, None);
    assert!(!log.screen_on(), "the home holds it");
    frame(&mut log, &session, NOON + 1520);
    assert!(log.screen_on());
    hidden(&mut log, NOON + 1530);
    assert!(!log.screen_on(), "hidden");
    frame(&mut log, &session, NOON + 1540);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    assert!(log.screen_on(), "the break counts");
}

#[test]
fn each_transition_is_told_once_as_it_happens() {
    use TimerTold::{BreakEnded, BreakStarted, Counts, Held, Up};
    let (mut log, mut session) = begun();
    assert_eq!(log.timer_told(), [TimerTold::Started(1500), Counts]);
    log.time(TimerIntent::Pause, None).expect("pause");
    log.time(TimerIntent::Resume, None).expect("continue");
    hidden(&mut log, NOON + 60);
    frame(&mut log, &session, NOON + 120);
    frame(&mut log, &session, NOON + 1600);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    frame(&mut log, &session, NOON + 1900);
    let completed = TimerTold::Completed {
        lessons: 1,
        exercises: 1,
    };
    let told = [Held, Counts, Held, Counts, Up, completed, BreakStarted(300)];
    assert_eq!(
        log.timer_told(),
        [&told[..], &[BreakEnded { chime: true }]].concat()
    );
    assert_eq!(log.timer_told(), [], "once");
    // End with nothing begun; a break left before its end.
    let (mut log, mut session) = untouched();
    frame(&mut log, &session, NOON + 60);
    next(&mut session, NOON + 60);
    frame(&mut log, &session, NOON + 1500);
    log.timer_told();
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    assert_eq!(log.timer_told(), [TimerTold::Stopped]);
    let (mut log, mut session) = begun();
    frame(&mut log, &session, NOON + 1500);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    log.timer_told();
    tick(&mut log, NOON + 1600, None);
    assert_eq!(log.timer_told(), [TimerTold::BreakLeft]);
}
