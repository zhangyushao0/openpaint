//! A block on the sheet: the chain of lessons with no end between (in place, or a new sheet when
//! the next does not fit; the append's rollback), the time's end and End's cut, the block's end,
//! the break and its one chime, Another pomodoro.
//!
//! What a block's end counts (`BlockTally::of`) and today's seals (`Blocks::on`) are the pure
//! crate's: these tests hold the timer's to them.

use openpaint_app::{Fitted, Session};
use openpaint_practice::{Block, BlockTally, Blocks, Givens, Layout, Length, Purpose, Sheet};
use openpaint_practice_app::{
    BlockShown, PracticeLog, PracticeNotice, PracticeSheet, Start, Timed, TimerIntent,
    append_lesson, append_stored,
};

use super::{
    NOON, VIEWPORT, day, log_of, next, practising, record_of, running, shown, six_lines, tick,
};

/// A block of 25 minutes begun on the home at noon, a day's sheet of six lines opened at once
/// and in front.
fn begun() -> (PracticeLog, Session) {
    let empty = log_of(Vec::new(), NOON);
    let drill = six_lines(&empty, (NOON, 40), Purpose::Day);
    let mut log = log_of(vec![drill.clone()], NOON);
    tick(&mut log, NOON, None);
    assert_eq!(log.time(TimerIntent::Start, None), Ok(Timed::Lesson));
    let session = practising(&drill, Layout::laid_ahead(&drill), &log);
    tick(&mut log, NOON, Some(&drill));
    (log, session)
}

/// A frame at `now` with `session`'s sheet in front.
fn frame(log: &mut PracticeLog, session: &Session, now: u64) {
    tick(log, now, Some(&record_of(session)));
}

/// Next on each exercise left of the lesson in hand, a frame a minute from `now`; the time then.
fn finish(log: &mut PracticeLog, session: &mut Session, mut now: u64) -> u64 {
    while shown(session).current.is_some() {
        now += 60;
        frame(log, session, now);
        next(session, now);
    }
    now
}

/// The chain as the runtime lays it after a Next: the lesson the end offers, in place.
fn chained(log: &mut PracticeLog, session: &mut Session, now: u64) -> Option<Sheet> {
    assert!(log.chains(session), "a lesson ended with time left");
    let offer = shown(session)
        .end
        .and_then(|end| end.next)
        .expect("an offer");
    log.keep(record_of(session));
    let (sheet, _) = log.start(Start::Skill(offer.skill), 7, now).expect("begun");
    let lesson = sheet.lessons.into_iter().last().expect("its lesson");
    assert_eq!(
        lesson.skill(),
        Some(offer.skill),
        "One more's offer honoured"
    );
    let givens = Givens::of_lesson(&lesson, log.packs());
    let (record, _) = append_lesson(session, (lesson, givens), log, &VIEWPORT)?;
    log.keep(record.clone());
    Some(record)
}

/// The record End cut, for the store, once.
fn trimmed(session: &mut Session) -> Option<Sheet> {
    let sheet = PracticeSheet::of_mut(session).expect("a sheet");
    let trimmed = sheet.trim_request();
    assert_eq!(sheet.trim_request(), None, "once");
    trimmed
}

/// The block's end the frame shows: its length, its tally and the break's minutes left.
fn ended(log: &PracticeLog) -> (Length, BlockTally, Option<u32>) {
    match log.timer().block {
        Some(BlockShown::Ended { end, rest }) => (end.length, end.tally.clone(), rest.map(|r| r.0)),
        other => panic!("the block's end: {other:?}"),
    }
}

#[test]
fn the_chain_lays_lesson_after_lesson_with_no_end_between() {
    let (mut log, mut session) = begun();
    let now = finish(&mut log, &mut session, NOON);
    assert!(shown(&session).end.is_some(), "kept for the offer");
    assert_eq!(running(&log), Some((19, false)), "the screens show no end");
    let record = chained(&mut log, &mut session, now).expect("in place");
    assert!(!log.chains(&session), "the next lesson in hand");
    append_stored(&mut session, record.id, true);
    let practice = shown(&session);
    assert_eq!(practice.current, Some(6), "from the third row");
    let [x, y, width, height] = Layout::row_rect(6);
    let row = Fitted {
        origin: [x, y].map(|v| v as u32),
        size: [width, height].map(|v| v as u32),
    };
    assert_eq!(session.view().fits(), Some(row), "glided to its first pair");
    let now = finish(&mut log, &mut session, now);
    assert!(log.chains(&session), "and again");
    assert!(chained(&mut log, &mut session, now).is_some());
}

#[test]
fn a_chain_that_does_not_fit_goes_on_a_new_sheet_the_block_held_while_it_opens() {
    let (mut log, _) = begun();
    let mut full = log.session(1).expect("the drill").clone();
    for _ in 1..6 {
        if let Some(last) = full.lessons.last_mut() {
            last.finished = Some(NOON);
        }
        let lesson = six_lines(&log, (NOON, 0), Purpose::Day).lessons.remove(0);
        full.append(lesson, Givens::default());
    }
    log.keep(full.clone());
    let mut session = practising(&full, Layout::laid_ahead(&full), &log);
    let now = finish(&mut log, &mut session, NOON);
    assert!(
        chained(&mut log, &mut session, now).is_none(),
        "twelve rows used"
    );
    // The runtime leaves for a new sheet: held while it opens.
    tick(&mut log, now + 1, None);
    tick(&mut log, now + 600, None);
    assert_eq!(running(&log), Some((19, false)));
    let new = six_lines(&log, (now + 600, 41), Purpose::Day);
    tick(&mut log, now + 600, Some(&new));
    tick(&mut log, now + 660, Some(&new));
    assert_eq!(running(&log), Some((18, false)));
}

#[test]
fn time_up_mid_exercise_then_end_cuts_the_lesson_and_keeps_the_block() {
    let (mut log, mut session) = begun();
    for now in [NOON + 60, NOON + 120] {
        frame(&mut log, &session, now);
        next(&mut session, now);
    }
    frame(&mut log, &session, NOON + 1600);
    assert_eq!(
        log.timer().block,
        Some(BlockShown::Up),
        "nothing interrupts"
    );
    assert_eq!(
        shown(&session).current,
        Some(2),
        "the exercise in hand stays"
    );
    log.timer_request();
    let done = log.time(TimerIntent::End(VIEWPORT), Some(&mut session));
    assert_eq!(done, Ok(Timed::Done));
    let record = record_of(&session);
    let lesson = &record.lessons[0];
    assert_eq!(
        (lesson.exercises.len(), lesson.at),
        (3, 2),
        "those after it gone"
    );
    assert_eq!(lesson.finished, Some(NOON + 1600));
    assert_eq!(trimmed(&mut session), Some(record.clone()));
    let sheet = PracticeSheet::of_mut(&mut session).expect("a sheet");
    assert_eq!(sheet.save_request(), None, "the cut went with the trim");
    let practice = shown(&session);
    assert_eq!((practice.sheet.pairs.len(), practice.current), (3, None));
    assert_eq!(practice.end.map(|end| end.results.len()), Some(3));
    let used = Layout::used(&record);
    assert_eq!(session.view().fits().map(|f| f.size), Some(used));
    assert_eq!(log.session(1), Some(&record), "the log took the cut");
    // The block completed in one write, its end over the rows drawn in it.
    let block = Block {
        started: NOON,
        day: day(NOON),
        length: Length::TwentyFive,
    };
    let write = log.timer_request().expect("written");
    assert_eq!((write.under_way, write.completed), (None, Some(block)));
    let tally = BlockTally::of(std::slice::from_ref(&record), NOON);
    assert_eq!(ended(&log), (Length::TwentyFive, tally, Some(5)));
    let blocks = Blocks {
        done: vec![block],
        ..Blocks::default()
    };
    let seals = u8::try_from(blocks.on(day(NOON))).expect("a few");
    assert_eq!(log.timer().sealed, seals);
}

#[test]
fn a_next_as_the_time_runs_out_chains_nothing_and_end_keeps_the_lesson_whole() {
    let (mut log, mut session) = begun();
    for now in (1..6).map(|minute| NOON + minute * 60) {
        frame(&mut log, &session, now);
        next(&mut session, now);
    }
    frame(&mut log, &session, NOON + 1500);
    assert_eq!(
        log.timer().block,
        Some(BlockShown::Up),
        "exactly at its end"
    );
    next(&mut session, NOON + 1500);
    assert!(!log.chains(&session), "the block ends there");
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    assert_eq!(trimmed(&mut session), None, "nothing to cut");
    let sheet = PracticeSheet::of_mut(&mut session).expect("a sheet");
    let saved = sheet.save_request().expect("saved as ever");
    assert_eq!(saved.lessons[0].exercises.len(), 6);
    assert_eq!(saved.lessons[0].finished, Some(NOON + 1500));
}

#[test]
fn a_lesson_not_begun_when_the_time_ran_out_leaves_nothing() {
    let (mut log, mut session) = begun();
    let now = finish(&mut log, &mut session, NOON);
    let record = chained(&mut log, &mut session, now).expect("in place");
    append_stored(&mut session, record.id, true);
    frame(&mut log, &session, NOON + 1500);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    let cut = trimmed(&mut session).expect("cut");
    assert_eq!(
        cut.lessons.len(),
        1,
        "the lesson laid as the time ran out gone"
    );
    let practice = shown(&session);
    assert_eq!(
        (practice.sheet.pairs.len(), practice.sheet.rows.len()),
        (6, 1)
    );
    assert_eq!(practice.end.map(|end| end.results.len()), Some(6));
    let tally = BlockTally::of(std::slice::from_ref(&cut), NOON);
    assert_eq!(ended(&log).1, tally);
    // A sheet's only lesson keeps its first exercise, the How over it closed.
    let (mut log, mut session) = begun();
    assert!(shown(&session).how.is_some(), "the lines' How by itself");
    frame(&mut log, &session, NOON + 1500);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    let practice = shown(&session);
    assert_eq!((practice.how, practice.current), (None, None));
    let cut = trimmed(&mut session).expect("cut");
    assert_eq!(cut.lessons[0].exercises.len(), 1);
}

#[test]
fn the_break_counts_down_and_chimes_once() {
    for chime in [true, false] {
        let (mut log, mut session) = begun();
        log.time(TimerIntent::Chime(chime), None).expect("switch");
        frame(&mut log, &session, NOON + 1500);
        log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
            .expect("end");
        assert_eq!((log.wake(), log.screen_on()), (Some(60), true));
        frame(&mut log, &session, NOON + 1500 + 299);
        assert_eq!((ended(&log).2, log.wake()), (Some(1), Some(1)));
        assert!(!log.take_chime());
        frame(&mut log, &session, NOON + 1500 + 300);
        assert_eq!(ended(&log).2, None, "the ring complete");
        assert_eq!(log.take_chime(), chime, "the switch");
        assert!(!log.take_chime(), "once");
        assert_eq!((log.wake(), log.screen_on()), (None, false));
        frame(&mut log, &session, NOON + 3000);
        assert!(!log.take_chime());
        // That's all for today: the home, no block.
        tick(&mut log, NOON + 3001, None);
        assert_eq!(log.timer().block, None);
    }
    // Left during the break: no chime.
    let (mut log, mut session) = begun();
    frame(&mut log, &session, NOON + 1500);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    frame(&mut log, &session, NOON + 1600);
    tick(&mut log, NOON + 2000, None);
    assert!(!log.take_chime());
}

#[test]
fn another_pomodoro_starts_at_once_during_the_break() {
    let (mut log, mut session) = begun();
    frame(&mut log, &session, NOON + 1500);
    log.time(TimerIntent::End(VIEWPORT), Some(&mut session))
        .expect("end");
    log.timer_request();
    frame(&mut log, &session, NOON + 1600);
    let another = log.time(TimerIntent::Another(VIEWPORT), Some(&mut session));
    assert_eq!(another, Ok(Timed::OneMore(VIEWPORT)));
    assert_eq!(running(&log), Some((25, false)));
    let begun = log.timer_request().and_then(|w| w.under_way).expect("kept");
    assert_eq!(begun.started, NOON + 1600);
    assert!(log.chains(&session), "the runtime lays the end's next");
    assert!(shown(&session).end.and_then(|end| end.next).is_some());
    let refused = log.time(TimerIntent::Another(VIEWPORT), Some(&mut session));
    assert!(refused.is_err(), "only on a block's end");
    frame(&mut log, &session, NOON + 1900);
    assert!(!log.take_chime(), "the break is over");
}

#[test]
fn a_chain_whose_write_failed_rolls_back_with_the_block_still_counting() {
    let (mut log, mut session) = begun();
    let now = finish(&mut log, &mut session, NOON);
    let record = chained(&mut log, &mut session, now).expect("in place");
    append_stored(&mut session, record.id, false);
    let practice = shown(&session);
    assert_eq!(practice.notice, Some(PracticeNotice::NotSaved));
    assert_eq!((practice.current, practice.sheet.rows.len()), (None, 1));
    assert!(practice.rolled_back, "its end shows while the block runs");
    assert_eq!(running(&log), Some((19, false)));
    assert!(log.chains(&session), "its lesson ended, time left");
    // One more tries again: laid, the end gone with the roll-back.
    let record = chained(&mut log, &mut session, now).expect("in place");
    assert!(!shown(&session).rolled_back);
    append_stored(&mut session, record.id, true);
    assert_eq!(shown(&session).current, Some(6));
    // End is refused while a lesson appended waits for the store.
    let (mut log, mut session) = begun();
    let now = finish(&mut log, &mut session, NOON);
    chained(&mut log, &mut session, now).expect("in place");
    frame(&mut log, &session, NOON + 1500);
    let waiting = log.time(TimerIntent::End(VIEWPORT), Some(&mut session));
    assert!(waiting.is_err());
    assert_eq!(log.timer().block, Some(BlockShown::Up));
}
