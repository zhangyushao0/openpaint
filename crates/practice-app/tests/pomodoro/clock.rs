//! The timer's clock: what counts, what holds, what shows, what is written, what a day and a
//! restart do to a block under way.

use openpaint_practice::{Blocks, Length, Purpose, Timing, UnderWay};
use openpaint_practice_app::{BlockShown, Timed, TimerIntent};

use super::{DAY, NOON, VIEWPORT, day, log_of, running, six_lines, tick};

#[test]
fn the_block_counts_only_while_a_day_s_or_a_test_s_sheet_is_in_front() {
    let mut log = log_of(Vec::new(), NOON);
    let sheet = six_lines(&log, (NOON, 40), Purpose::Day);
    let test = six_lines(&log, (NOON, 41), Purpose::Test);
    let copy = openpaint_practice::Sheet {
        purpose: Purpose::Copy,
        ..sheet.clone()
    };
    tick(&mut log, NOON, None);
    assert_eq!(log.timer().block, None, "Start timer offered");
    assert_eq!(log.time(TimerIntent::Start, None), Ok(Timed::Lesson));
    assert_eq!(running(&log), Some((25, false)));
    // Held on the home, and while the sheet opens: the seconds since the home's tick are its.
    tick(&mut log, NOON + 600, None);
    tick(&mut log, NOON + 610, Some(&sheet));
    assert_eq!(running(&log), Some((25, false)));
    // A tick that comes late counts every second the sheet was in front, exactly.
    tick(&mut log, NOON + 610 + 61, Some(&sheet));
    assert_eq!(running(&log), Some((24, false)), "1439 s left");
    // Time that does not move, or is set back, counts nothing.
    tick(&mut log, NOON + 671, Some(&sheet));
    tick(&mut log, NOON + 500, Some(&sheet));
    tick(&mut log, NOON + 600, Some(&sheet));
    tick(&mut log, NOON + 671, Some(&sheet));
    assert_eq!(running(&log), Some((24, false)));
    // A colour copy in front holds it, from the tick that saw it.
    tick(&mut log, NOON + 671 + 300, Some(&copy));
    assert_eq!(running(&log), Some((19, false)), "the sheet was in front");
    tick(&mut log, NOON + 671 + 900, Some(&copy));
    assert_eq!(running(&log), Some((19, false)));
    // A test's sheet counts as a day's does; a jump of minutes counts down to Time, never below.
    tick(&mut log, NOON + 2000, Some(&test));
    tick(&mut log, NOON + 2000 + 3 * 3600, Some(&test));
    assert_eq!(log.timer().block, Some(BlockShown::Up));
    tick(&mut log, NOON + 2000 + 4 * 3600, None);
    assert_eq!(log.timer().block, Some(BlockShown::Up), "held at Time");
    // Start on the home with a block under way is Continue: the same block, its lesson opened.
    assert_eq!(log.time(TimerIntent::Start, None), Ok(Timed::Lesson));
    assert_eq!(log.timer().block, Some(BlockShown::Up));
}

#[test]
fn pause_continue_and_stop() {
    let mut log = log_of(Vec::new(), NOON);
    let sheet = six_lines(&log, (NOON, 40), Purpose::Day);
    tick(&mut log, NOON, Some(&sheet));
    let refused =
        |log: &mut openpaint_practice_app::PracticeLog, intent| log.time(intent, None).is_err();
    for intent in [TimerIntent::Pause, TimerIntent::Resume, TimerIntent::Stop] {
        assert!(refused(&mut log, intent), "no block: {intent:?}");
    }
    assert!(refused(&mut log, TimerIntent::Another(VIEWPORT)));
    log.time(TimerIntent::Start, None).expect("start");
    assert!(refused(&mut log, TimerIntent::End(VIEWPORT)), "time left");
    log.time(TimerIntent::Pause, None).expect("pause");
    assert_eq!(running(&log), Some((25, true)));
    tick(&mut log, NOON + 600, Some(&sheet));
    assert_eq!(running(&log), Some((25, true)), "paused: nothing counts");
    assert_eq!((log.wake(), log.screen_on()), (None, false));
    log.time(TimerIntent::Resume, None).expect("continue");
    tick(&mut log, NOON + 660, Some(&sheet));
    assert_eq!(running(&log), Some((24, false)));
    assert!(log.screen_on());
    // Up: no Pause, End only on a sheet, Stop still.
    tick(&mut log, NOON + 3600, Some(&sheet));
    assert!(refused(&mut log, TimerIntent::Pause));
    assert!(refused(&mut log, TimerIntent::End(VIEWPORT)), "off a sheet");
    log.timer_request();
    assert_eq!(log.time(TimerIntent::Stop, None), Ok(Timed::Done));
    assert_eq!(log.timer().block, None);
    let write = log.timer_request().expect("written");
    assert_eq!((write.under_way, write.completed), (None, None), "no seal");
    assert_eq!(log.timer().sealed, 0);
}

#[test]
fn the_minutes_shown_round_up_and_the_timer_wakes_at_each_turn() {
    let mut log = log_of(Vec::new(), NOON);
    let sheet = six_lines(&log, (NOON, 40), Purpose::Day);
    tick(&mut log, NOON, None);
    log.time(TimerIntent::Length(Length::Fifteen), None)
        .expect("15");
    log.time(TimerIntent::Start, None).expect("start");
    assert_eq!((log.wake(), log.screen_on()), (None, false), "held");
    tick(&mut log, NOON, Some(&sheet));
    assert_eq!((running(&log), log.wake()), (Some((15, false)), Some(60)));
    assert!(log.screen_on());
    let at = |log: &mut openpaint_practice_app::PracticeLog, gone: u64| {
        tick(log, NOON + gone, Some(&sheet));
        (running(log).map(|(left, _)| left), log.wake())
    };
    assert_eq!(at(&mut log, 1), (Some(15), Some(59)));
    assert_eq!(at(&mut log, 60), (Some(14), Some(60)));
    assert_eq!(at(&mut log, 839), (Some(2), Some(1)));
    assert_eq!(at(&mut log, 840), (Some(1), Some(60)));
    assert_eq!(at(&mut log, 899), (Some(1), Some(1)));
    let BlockShown::Running { share, .. } = log.timer().block.expect("a block") else {
        panic!("running");
    };
    assert!((share - 1.0 / 900.0).abs() < 1e-6, "the ring's share left");
    assert_eq!(at(&mut log, 900), (None, None), "Time: nothing more counts");
    assert_eq!(log.timer().block, Some(BlockShown::Up));
    assert!(!log.screen_on());
}

#[test]
fn the_store_gets_each_transition_and_a_minute_while_it_counts() {
    let mut log = log_of(Vec::new(), NOON);
    let sheet = six_lines(&log, (NOON, 40), Purpose::Day);
    tick(&mut log, NOON, Some(&sheet));
    assert_eq!(log.timer_request(), None, "nothing changed");
    log.time(TimerIntent::Chime(false), None).expect("switch");
    let write = log.timer_request().expect("the settings");
    assert_eq!(
        write.timing,
        Timing {
            length: Length::TwentyFive,
            chime: false
        }
    );
    log.time(TimerIntent::Start, None).expect("start");
    let begun = UnderWay {
        started: NOON,
        day: day(NOON),
        length: Length::TwentyFive,
        left: 1500,
        paused: false,
    };
    assert_eq!(log.timer_request().and_then(|w| w.under_way), Some(begun));
    tick(&mut log, NOON + 59, Some(&sheet));
    assert_eq!(log.timer_request(), None);
    tick(&mut log, NOON + 60, Some(&sheet));
    let left = |log: &mut openpaint_practice_app::PracticeLog| {
        log.timer_request()
            .and_then(|w| w.under_way)
            .map(|b| b.left)
    };
    assert_eq!(left(&mut log), Some(1440), "a minute on");
    tick(&mut log, NOON + 1499, Some(&sheet));
    assert_eq!(left(&mut log), Some(1));
    tick(&mut log, NOON + 1500, Some(&sheet));
    assert_eq!(left(&mut log), Some(0), "Time");
    assert_eq!(log.timer_request(), None);
}

#[test]
fn a_block_held_when_the_day_changes_is_dropped_without_a_seal() {
    let midnight = NOON + DAY / 2;
    let mut log = log_of(Vec::new(), NOON);
    let sheet = six_lines(&log, (NOON, 40), Purpose::Day);
    tick(&mut log, NOON, Some(&sheet));
    log.time(TimerIntent::Start, None).expect("start");
    tick(&mut log, NOON + 60, None);
    log.timer_request();
    tick(&mut log, midnight + 60, None);
    assert_eq!(log.timer().block, None, "held over midnight");
    let write = log.timer_request().expect("written");
    assert_eq!((write.under_way, write.completed), (None, None));
    // One running over midnight goes on, and is held on its new day as on any.
    let mut log = log_of(Vec::new(), midnight - 600);
    tick(&mut log, midnight - 600, Some(&sheet));
    log.time(TimerIntent::Start, None).expect("start");
    tick(&mut log, midnight + 300, Some(&sheet));
    tick(&mut log, midnight + 400, None);
    assert_eq!(running(&log), Some((9, false)), "500 s left");
    let kept = log.timer_request().and_then(|w| w.under_way).expect("kept");
    assert_eq!(
        kept.day,
        day(midnight - 600),
        "its seal counts on its own day"
    );
    tick(&mut log, midnight + DAY, None);
    assert_eq!(log.timer().block, None);
}

#[test]
fn a_restart_brings_a_block_under_way_back_held() {
    let block = UnderWay {
        started: NOON,
        day: day(NOON),
        length: Length::Forty,
        left: 900,
        paused: false,
    };
    let kept = Blocks {
        timing: Timing {
            length: Length::Forty,
            chime: true,
        },
        under_way: Some(block),
        done: Vec::new(),
    };
    let mut log = log_of(Vec::new(), NOON + 3600);
    let sheet = six_lines(&log, (NOON, 40), Purpose::Day);
    log.set_blocks(kept.clone());
    assert_eq!(log.timer().length, Length::Forty);
    // The seconds the app was gone are not the block's, even with the sheet in front at once.
    tick(&mut log, NOON + 3600, Some(&sheet));
    assert_eq!(running(&log), Some((15, false)));
    tick(&mut log, NOON + 3660, Some(&sheet));
    assert_eq!(running(&log), Some((14, false)));
    // Paused it stays paused.
    let paused = Blocks {
        under_way: Some(UnderWay {
            paused: true,
            ..block
        }),
        ..kept.clone()
    };
    log.set_blocks(paused);
    tick(&mut log, NOON + 3700, Some(&sheet));
    tick(&mut log, NOON + 3800, Some(&sheet));
    assert_eq!(running(&log), Some((15, true)));
    // A restart on a later day finds it held: dropped.
    let mut later = log_of(Vec::new(), NOON + DAY);
    later.set_blocks(kept);
    tick(&mut later, NOON + DAY, Some(&sheet));
    assert_eq!(later.timer().block, None);
    let write = later.timer_request().expect("written");
    assert_eq!((write.under_way, write.completed), (None, None));
}
