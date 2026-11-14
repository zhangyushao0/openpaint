//! The Pomodoro's blocks (S124): a day's seals counted on the local day each block started,
//! across midnight and the clock's offset; the book's days holding each day's blocks, a day with
//! a block and no time among them; a block's end counting the lessons finished in it, one
//! continued into it whole and one cut at its end to the exercises done.

use openpaint_practice::{
    Block, BlockTally, Blocks, Book, Day, Exercise, Givens, ItemRef, Judge, Kind, Length, LineTier,
    Outcome, Picked, Planned, Purpose, Sheet, Skill, SkillLevel, Span, local_day, schedule,
};

/// 2026-10-05 12:00 UTC, 20:00 in Beijing.
const NOON: u64 = 1_791_201_600;
const MINUTE: u64 = 60;
/// The device's offset in Beijing, minutes.
const BEIJING: i32 = 8 * 60;

/// A block of 25 minutes started at `started`, on its local day in Beijing.
fn block(started: u64) -> Block {
    Block {
        started,
        day: local_day(started, BEIJING),
        length: Length::TwentyFive,
    }
}

/// A lesson of `n` arcs, the `id`th sheet's, begun at `started`.
fn arcs(id: u64, n: u64, started: u64) -> Sheet {
    let exercises = (0..n)
        .map(|seed| {
            (
                Exercise::generated(Kind::Line(LineTier::Arc), seed),
                SkillLevel::First,
            )
        })
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    Sheet::begun(id, (Purpose::Day, started), planned)
}

/// The lesson of `sheet` (its only one) judged `results` in turn and finished at `finished`.
fn ended(mut sheet: Sheet, results: &[Option<Outcome>], finished: u64) -> Sheet {
    let lesson = &mut sheet.lessons[0];
    for (laid, result) in lesson.exercises.iter_mut().zip(results) {
        laid.result = result.map(|outcome| (outcome, Judge::App));
    }
    lesson.finished = Some(finished);
    sheet
}

#[test]
fn a_block_s_seal_counts_on_the_local_day_it_started_even_past_midnight() {
    // 23:50 in Beijing, running past midnight; then two blocks the next morning.
    let late = NOON + 3 * 60 * MINUTE + 50 * MINUTE;
    let morning = NOON + 4 * 60 * MINUTE + 10 * MINUTE;
    let blocks = Blocks {
        done: vec![block(late), block(morning), block(morning + 40 * MINUTE)],
        ..Blocks::default()
    };
    let today = local_day(late, BEIJING);
    assert_eq!(
        local_day(late + 25 * MINUTE, BEIJING),
        today + 1,
        "it ended the next day"
    );
    assert_eq!((blocks.on(today), blocks.on(today + 1)), (1, 2));
    assert_eq!(blocks.on(today - 1), 0);
    // In UTC all three started on one day (15:50 to 16:50): the day is the one kept with it.
    assert_eq!(local_day(late, 0), local_day(morning, 0));
    assert_eq!(blocks.on(local_day(late, 0)), 1);
}

#[test]
fn the_book_holds_each_day_s_blocks_and_a_day_with_a_block_and_no_time() {
    let mut sheet = arcs(1, 2, NOON);
    sheet.spans.push(Span {
        from: NOON,
        to: NOON + 20 * MINUTE,
    });
    let day = local_day(NOON, BEIJING);
    let next = NOON + 24 * 60 * MINUTE;
    let earlier = NOON - 3 * 24 * 60 * MINUTE;
    let blocks = [
        block(NOON),
        block(next),
        block(next + 30 * MINUTE),
        block(earlier),
    ];
    let book = Book::of(std::slice::from_ref(&sheet), &blocks, BEIJING);
    let shown = |day, seconds, records: &[u64], blocks| Day {
        day,
        seconds,
        records: records.to_vec(),
        blocks,
    };
    assert_eq!(
        book.days,
        [
            shown(day - 3, 0, &[], 1),
            shown(day, 1200, &[1], 1),
            shown(day + 1, 0, &[], 2),
        ]
    );
    assert_eq!(book.day(day + 1).map(|shown| shown.blocks), Some(2));
    assert_eq!(book.seconds, 1200, "a block adds no time of its own");
    let without = Book::of(std::slice::from_ref(&sheet), &[], BEIJING);
    assert_eq!(without.days, [shown(day, 1200, &[1], 0)]);
}

#[test]
fn a_block_counts_the_lessons_finished_in_it_one_continued_into_it_whole() {
    use Outcome::{Clean, Near};
    let since = NOON + 20 * MINUTE;
    let end = since + 25 * MINUTE;
    // Before the block: four clean arcs. Continued into it: two more clean, the sixth at the
    // level earns the line's first dot; then a near and one not judged.
    let before = ended(arcs(1, 4, NOON), &[Some(Clean); 4], NOON + 10 * MINUTE);
    let continued = [Some(Clean), Some(Clean), Some(Near), None];
    let into = ended(
        arcs(2, 4, NOON + 15 * MINUTE),
        &continued,
        since + 5 * MINUTE,
    );
    // Cut at the block's end to the two done, finished at the end.
    let mut cut = arcs(3, 4, since + 10 * MINUTE);
    cut.lessons[0].exercises.truncate(2);
    let cut = ended(cut, &[Some(Clean), None], end);
    // The first lesson, finished in the block: never scored, whatever it holds.
    let mut first = Sheet::begun(4, (Purpose::Test, since + 20 * MINUTE), schedule::first(5));
    first.lessons[0].exercises[1].result = Some((Clean, Judge::Eye));
    first.lessons[0].finished = Some(since + 22 * MINUTE);
    // A lesson in hand on it, not finished, and a long colour copy: neither counts.
    let next = arcs(0, 2, end).lessons.remove(0);
    first.append(next, Givens::default());
    let picture = ItemRef {
        pack: "pack".into(),
        item: "item".into(),
    };
    let mut copy = Sheet::copy(5, (9, since), picture);
    copy.lessons[0].finished = Some(end);
    let sheets = [before, into, cut, first, copy];

    let tally = BlockTally::of(&sheets, since);
    assert_eq!((tally.lessons, tally.exercises), (3, 4 + 2 + 3));
    let expected = [continued.to_vec(), vec![Some(Clean), None], vec![None; 3]].concat();
    assert_eq!(tally.results, expected);
    let line = Skill::of(Kind::Line(LineTier::Arc)).expect("lines");
    assert_eq!(tally.skills, [(line, 0, 1)]);
    // A lesson finished at the block's start counts; after the block's end, none is.
    assert_eq!(BlockTally::of(&sheets, since + 5 * MINUTE).lessons, 3);
    assert_eq!(BlockTally::of(&sheets, end + 1), BlockTally::default());
}
