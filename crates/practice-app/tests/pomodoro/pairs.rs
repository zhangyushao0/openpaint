//! A block's blind pair (S148): the lessons a block chains show no end, so none of them asks a pair
//! or marks one asked; End shows the block's end, which asks one pair of earlier copies among the
//! skills its lessons practised, marked asked only then; skipped it stores nothing, answered once
//! it is kept under the block's last lesson; another block on the same sheet asks its own, its
//! copies' cards read anew.

use openpaint_practice::{
    Answer, BlindPair, Exercise, First, Givens, Judge, Kind, Layout, LineTier, Outcome, Picked,
    Planned, Purpose, Sheet, Skill, SkillLevel,
};
use openpaint_practice_app::{
    PracticeIntent, PracticeLog, SheetSession, Start, Timed, TimerIntent,
};

use super::block::{chained, ended, finish, frame};
use super::{DAY, NOON, VIEWPORT, log_of, practising, shown, six_lines, tick};

/// The block's day: five weeks after the first copies.
const TODAY: u64 = NOON + 36 * DAY;

/// Sheet `id` of day `day`: a lesson of two straight lines, each compared and judged Almost by
/// eye.
fn past(id: u64, day: u64) -> Sheet {
    let at = NOON + day * DAY;
    let exercises = (0..2)
        .map(|i| {
            let kind = Kind::Line(LineTier::Straight);
            (Exercise::generated(kind, id * 8 + i), SkillLevel::First)
        })
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let mut sheet = Sheet::begun(id, (Purpose::Day, at), planned);
    sheet.canvas = 100 + id;
    for index in 0..2 {
        let laid = sheet.laid_mut(index).expect("an exercise");
        laid.compared(First::of(at + 60, 1, Vec::new()));
        laid.result = Some((Outcome::Near, Judge::Eye));
    }
    if let Some(lesson) = sheet.lesson_mut() {
        lesson.finished = Some(at + 900);
    }
    sheet
}

/// Lines copied on days 1 and 29, then today's sheet of six lines open with a block of 25
/// minutes under way, the sheet told so (as the runtime tells it each frame).
fn block_today() -> (PracticeLog, SheetSession) {
    let earlier = vec![past(1, 1), past(2, 29)];
    let day = six_lines(&log_of(earlier.clone(), TODAY), (TODAY, 40), Purpose::Day);
    let mut log = log_of([earlier, vec![day.clone()]].concat(), TODAY);
    tick(&mut log, TODAY, None);
    assert_eq!(log.time(TimerIntent::Start, None), Ok(Timed::Lesson));
    let mut session = practising(&day, Layout::laid_ahead(&day), &log);
    tick(&mut log, TODAY, Some(&day));
    told(&log, &mut session);
    (log, session)
}

/// The sheet told whether a block is in hand, as the runtime tells it each frame.
fn told(log: &PracticeLog, session: &mut SheetSession) {
    session
        .sheet_mut()
        .set_in_block(log.timer().block.is_some());
}

/// The lesson in hand finished in the block, its end asking nothing, and the next one chained in
/// place; the time then.
fn chain_on(log: &mut PracticeLog, session: &mut SheetSession, now: u64) -> u64 {
    let now = finish(log, session, now);
    let end = shown(session).end.expect("its end, behind the chain");
    assert!(
        end.pairs.is_empty(),
        "a lesson's end in a block asks nothing"
    );
    assert!(session.sheet().offered().is_empty(), "nothing marked asked");
    let record = chained(log, session, now).expect("in place");
    session.append_stored(record.id, true);
    now
}

/// End at `at`, the block's time up: its end in front.
fn end_at(log: &mut PracticeLog, session: &mut SheetSession, at: u64) {
    frame(log, session, at);
    told(log, session);
    let done = log.time(TimerIntent::End(VIEWPORT), Some(session));
    assert_eq!(done, Ok(Timed::Done));
    told(log, session);
}

#[test]
fn a_block_of_two_lessons_asks_one_pair_at_its_end_marked_asked_only_then() {
    let (mut log, mut session) = block_today();
    let now = chain_on(&mut log, &mut session, TODAY);
    chain_on(&mut log, &mut session, now);
    end_at(&mut log, &mut session, TODAY + 1500);
    assert_eq!(ended(&log).1.lessons, 2, "the lesson not begun gone");
    let end = shown(&session).end.expect("the end the block's card reads");
    assert_eq!(end.pairs.len(), 1, "one pair");
    let pair = end.pairs[0];
    assert_eq!(
        (pair.asked.older.sheet, pair.asked.newer.sheet, pair.answer),
        (1, 2, None),
        "two earlier copies weeks apart, never today's"
    );
    assert_eq!(session.sheet().offered(), [pair.asked], "asked once shown");
    assert!(
        session.sheet_mut().pairs_request().is_empty(),
        "skipped stores nothing"
    );
    let copies = session.copies_request().expect("its copies' cards");
    assert_eq!(copies, [pair.asked.older, pair.asked.newer]);
    assert!(session.copies_request().is_none(), "once");
    let answered = PracticeIntent::Pair(0, Answer::Left);
    session.practise(answered, TODAY + 1600).expect("answered");
    let end = shown(&session).end.expect("the end");
    assert_eq!(end.pairs[0].answer, Some(Answer::Left), "its dates show");
    let stored = session.sheet_mut().pairs_request();
    let lesson = stored.iter().map(|pair| pair.lesson).collect::<Vec<_>>();
    assert_eq!(lesson, [(3, 1)], "kept under the block's last lesson");
    assert!(session.practise(answered, TODAY + 1700).is_err(), "once");
}

#[test]
fn another_block_on_the_sheet_asks_its_own_pair_and_reads_its_cards() {
    let (mut log, mut session) = block_today();
    chain_on(&mut log, &mut session, TODAY);
    end_at(&mut log, &mut session, TODAY + 1500);
    let first = shown(&session).end.expect("the end").pairs[0].asked;
    session.copies_request().expect("its cards");
    // What the runtime keeps of it: asked today, answered.
    log.asked(session.sheet().offered());
    session
        .practise(PracticeIntent::Pair(0, Answer::Alike), TODAY + 1550)
        .expect("answered");
    let kept: Vec<BlindPair> = session.sheet_mut().pairs_request();
    kept.into_iter().for_each(|pair| log.keep_pair(pair));
    frame(&mut log, &session, TODAY + 1600);
    let another = log.time(TimerIntent::Another(VIEWPORT), Some(&mut session));
    assert_eq!(another, Ok(Timed::OneMore(VIEWPORT)));
    told(&log, &mut session);
    // Lines again, as a tile would start them: the next block practises the skill paired.
    log.keep(session.sheet().record().clone());
    let lines = Skill::of_key("line").expect("lines");
    let (sheet, _) = (log.start(Start::Skill(lines), 9, TODAY + 1600)).expect("lines");
    let lesson = sheet.lessons.into_iter().last().expect("its lesson");
    let givens = Givens::of_lesson(&lesson, log.packs());
    let laid = session.append_lesson((lesson, givens), &log, &VIEWPORT);
    let (record, _) = laid.expect("in place");
    session.append_stored(record.id, true);
    assert!(shown(&session).end.is_none(), "the next lesson in hand");
    let now = chain_on(&mut log, &mut session, TODAY + 1600);
    end_at(&mut log, &mut session, now + 1500);
    let end = shown(&session).end.expect("the second block's end");
    let second = end.pairs.first().expect("its own pair").asked;
    for copy in [second.older, second.newer] {
        assert!(
            ![first.older, first.newer].contains(&copy),
            "{copy:?} asked twice today"
        );
    }
    let copies = session.copies_request().expect("its cards read anew");
    assert_eq!(copies, [second.older, second.newer]);
}
