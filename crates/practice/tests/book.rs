//! The practice book on known records (S89): practice time from the uses only (a session left
//! untouched two hours counts its minutes, time hidden counts nothing, a copy painted over two
//! days shows on both), seals earned from what was done, the colour's first pick per session and
//! the lines shown.

use openpaint_practice::{
    Book, Day, Family, First, ItemRef, LineShown, Metric, PAUSE, STAGE_SESSIONS, Seal, Sheet, Span,
    Tally, earned, practised, sealed, seconds, stage_one,
};

/// 2026-10-05 12:00 UTC, a Monday.
const NOON: u64 = 1_791_201_600;
const DAY: u64 = 86_400;
/// The device's offset in Beijing, minutes.
const BEIJING: i32 = 8 * 60;

fn session(id: u64, started: u64) -> Sheet {
    Sheet::session(id, 1, started, stage_one(0x5EED_0045 + id, &[]))
}

/// The index of `session`'s `nth` exercise of `family`.
fn of(session: &Sheet, family: Family, nth: usize) -> usize {
    (session.exercises().enumerate())
        .filter(|(_, laid)| laid.exercise.kind.family() == family)
        .nth(nth)
        .map(|(index, _)| index)
        .expect("an exercise of the family")
}

/// `session`'s exercise `index` compared first at `at` with `metrics`.
fn compared(session: &mut Sheet, index: usize, at: u64, metrics: Vec<(Metric, f64)>) {
    let laid = session.laid_mut(index).expect("an exercise");
    laid.compared(First::of(at, 1, metrics));
}

/// `index` as the book names an exercise.
fn named(index: usize) -> u32 {
    u32::try_from(index).expect("index")
}

/// Uses every `step` seconds from `from` for `minutes`.
fn uses(spans: &mut Vec<Span>, from: u64, minutes: u64, step: u64) {
    let mut at = from;
    while at <= from + minutes * 60 {
        practised(spans, at, false);
        at += step;
    }
}

#[test]
fn a_session_left_untouched_two_hours_counts_only_its_minutes() {
    let mut record = session(1, NOON);
    uses(&mut record.spans, NOON, 12, 20);
    // Put down for two hours with the app in front, then eight more minutes.
    let back = NOON + 12 * 60 + 2 * 3600;
    uses(&mut record.spans, back, 8, 45);
    assert_eq!(seconds(&record.spans), 720 + 450);
    let book = Book::of(std::slice::from_ref(&record), 1, BEIJING);
    assert_eq!(book.seconds, seconds(&record.spans));
    assert_eq!(book.days.len(), 1);
    assert_eq!(book.days[0].records, [1]);
    assert!(
        (19 * 60..=20 * 60).contains(&book.days[0].seconds),
        "{} s: the two hours count nothing",
        book.days[0].seconds
    );
}

#[test]
fn time_hidden_counts_nothing_and_a_gap_over_the_pause_counts_none_of_it() {
    let mut spans = Vec::new();
    uses(&mut spans, NOON, 5, 30);
    // Hidden for three minutes (under the pause): none of it counts.
    practised(&mut spans, NOON + 8 * 60, true);
    uses(&mut spans, NOON + 8 * 60, 2, 30);
    assert_eq!(seconds(&spans), 7 * 60);
    // Six minutes untouched: not five of them, none.
    practised(&mut spans, NOON + 10 * 60 + 6 * 60, false);
    assert_eq!(seconds(&spans), 7 * 60);
    assert_eq!(PAUSE, 300);
}

#[test]
fn a_copy_painted_over_two_days_shows_on_both() {
    let item = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    };
    let mut copy = Sheet::copy(2, (9, NOON), item);
    copy.canvas = 40;
    // 21:00 to 21:40 in Beijing (13:00 UTC), and the next evening 20:00 to 20:30.
    uses(&mut copy.spans, NOON + 3600, 40, 60);
    uses(&mut copy.spans, NOON + DAY, 30, 60);
    let book = Book::of(std::slice::from_ref(&copy), 1, BEIJING);
    let days: Vec<(i64, u64, Vec<u64>)> = (book.days.iter())
        .map(|day| (day.day, day.seconds, day.records.clone()))
        .collect();
    assert_eq!(
        days,
        [(20_731, 2400, vec![2]), (20_732, 1800, vec![2])],
        "the evening of 5 October and of the 6th"
    );
    assert_eq!(book.sessions, 0, "a copy is no session");
    // Past midnight in Beijing: one sitting from 23:50 to 00:10 counts on both days.
    let picture = (copy.laid(0)).and_then(|laid| laid.exercise.picture.clone());
    let mut late = Sheet::copy(3, (9, NOON), picture.expect("picture"));
    uses(&mut late.spans, NOON + 3 * 3600 + 50 * 60, 20, 60);
    let book = Book::of(std::slice::from_ref(&late), 1, BEIJING);
    assert_eq!(
        book.days,
        [
            Day {
                day: 20_731,
                seconds: 600,
                records: vec![3]
            },
            Day {
                day: 20_732,
                seconds: 600,
                records: vec![3]
            }
        ]
    );
}

/// A finished session of stage 1 holding `lines` single lines and `colours` colours compared,
/// each colour's first pick `first` off, practised `minutes`.
fn done(id: u64, (lines, colours): (usize, usize), first: f64, minutes: u64) -> Sheet {
    let at = NOON + id * DAY;
    let mut record = session(id, at);
    for nth in 0..lines {
        let index = of(&record, Family::Line, nth);
        compared(&mut record, index, at + 60, Vec::new());
    }
    for nth in 0..colours {
        let index = of(&record, Family::Colour, nth);
        let metrics = vec![
            (Metric::FirstDelta, first),
            (Metric::FinalDelta, first / 2.0),
        ];
        compared(&mut record, index, at + 90, metrics);
    }
    record.spans = vec![Span {
        from: at,
        to: at + minutes * 60,
    }];
    record.lessons[0].finished = Some(at + minutes * 60);
    record
}

#[test]
fn seals_are_earned_from_what_was_done_and_stamp_on_the_session_that_did_it() {
    let records: Vec<Sheet> = (1..=10).map(|id| done(id, (6, 6), 0.05, 40)).collect();
    let seals = earned(&records);
    let by = |seal: Seal| seals.iter().find(|e| e.seal == seal).map(|e| e.by);
    assert_eq!(by(Seal::FirstSession), Some(1));
    assert_eq!(by(Seal::TenSessions), Some(10));
    assert_eq!(by(Seal::HundredLines), None, "60 lines");
    assert_eq!(by(Seal::FirstCopy), None);
    let others = Tally::of(&records[..9]);
    assert_eq!(sealed(&others, &records[9]), [Seal::TenSessions]);
    assert!(
        sealed(&Tally::of(&records), &records[9]).is_empty(),
        "earned once"
    );
    // 400 minutes so far; four more sessions of an hour pass ten hours, and seventeen
    // sessions of six lines make a hundred lines.
    let mut more = records.clone();
    more.extend((11..=STAGE_SESSIONS + 2).map(|id| done(u64::from(id), (6, 1), 0.04, 60)));
    let seals = earned(&more);
    let by = |seal: Seal| seals.iter().find(|e| e.seal == seal).map(|e| e.by);
    assert_eq!(by(Seal::Stage(1)), Some(u64::from(STAGE_SESSIONS)));
    assert_eq!(by(Seal::HundredLines), Some(17));
    assert_eq!(
        by(Seal::TenHours),
        Some(14),
        "400 minutes, then an hour a session"
    );
    assert_eq!(by(Seal::HundredColours), None);
    let mut unfinished = done(30, (0, 0), 0.0, 1);
    unfinished.lessons[0].finished = None;
    assert!(sealed(&Tally::default(), &unfinished).is_empty());
}

#[test]
fn the_book_shows_the_road_the_first_picks_and_the_first_and_latest_lines() {
    let mut records: Vec<Sheet> = vec![
        done(1, (2, 6), 0.08, 30),
        done(2, (0, 6), 0.06, 30),
        done(3, (1, 0), 0.0, 30),
    ];
    // An open session with a line compared: the latest.
    let mut open = session(4, NOON + 4 * DAY);
    let line = of(&open, Family::Line, 1);
    compared(&mut open, line, NOON + 4 * DAY + 60, Vec::new());
    records.push(open);
    let book = Book::of(&records, 1, 0);
    assert_eq!(book.sessions, 3);
    assert_eq!(book.road.stage, 1);
    assert_eq!(book.road.done_in(1), 3);
    assert_eq!(book.road.done_in(2), 0);
    assert_eq!(
        book.colour,
        [0.08, 0.06],
        "sessions without colours left out"
    );
    let first_line = of(&records[0], Family::Line, 0);
    assert_eq!(
        book.lines,
        [
            LineShown {
                record: 1,
                number: 1,
                index: named(first_line)
            },
            LineShown {
                record: 4,
                number: 4,
                index: named(line)
            }
        ]
    );
    assert_eq!(book.days.len(), 3, "the open session has no time yet");
    assert_eq!(book.seconds, 90 * 60);
    assert_eq!(book.day(20_732).map(|day| day.seconds), Some(1800));
}
