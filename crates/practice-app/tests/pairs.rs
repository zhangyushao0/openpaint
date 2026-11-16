//! A lesson's end and the learner's own eye (S147): a lesson of a skill done in this sitting asks
//! one blind pair of two earlier copies, never today's; skipped it stores nothing and its copies are
//! not asked again that day; answered once it is kept for the store and its dates show; a finished lesson
//! opened again shows its pair answered, else none. The extra drill shows only where the curve is
//! sure the skill fell. A unit's re-test is offered and laid as the path says. And the fit takes
//! well under a frame on a long history.

use std::sync::Arc;
use std::time::Instant;

use openpaint_app::{BrushLibrary, Title, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Answer, BlindPair, CopyRef, Exercise, First, Given, Judge, Kind, Layout, LineTier, Outcome,
    Picked, Planned, Progress, Purpose, Sheet, Skill, SkillFit, SkillLevel, older_left,
};
use openpaint_practice_app::{PracticeIntent, PracticeLog, PracticePens, SheetSession, Start};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC, and a day.
const NOON: u64 = 1_791_201_600;
const DAY: u64 = 86_400;

fn lines() -> Skill {
    Skill::of_key("line").expect("lines")
}

/// Sheet `id` of day `day`: a lesson of lines at the first level, each exercise compared and
/// judged by eye as `outcomes` say.
fn past(id: u64, day: u64, outcomes: &[Outcome]) -> Sheet {
    past_of(Kind::Line(LineTier::Straight), (id, day), outcomes)
}

/// Sheet `id` of day `day`: a lesson of `kind` at the first level, ended as `outcomes` say.
fn past_of(kind: Kind, (id, day): (u64, u64), outcomes: &[Outcome]) -> Sheet {
    let at = NOON + day * DAY;
    let exercises = (0..outcomes.len())
        .map(|i| {
            (
                Exercise::generated(kind, id * 8 + i as u64),
                SkillLevel::First,
            )
        })
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let mut sheet = Sheet::begun(id, (Purpose::Day, at), planned);
    sheet.canvas = 100 + id;
    for (index, outcome) in outcomes.iter().enumerate() {
        let laid = sheet.laid_mut(index).expect("an exercise");
        laid.compared(First::of(at + 60, 1, Vec::new()));
        laid.result = Some((*outcome, Judge::Eye));
    }
    if let Some(lesson) = sheet.lesson_mut() {
        lesson.finished = Some(at + 900);
    }
    sheet
}

/// `record` open as `log` opens it.
fn opened(record: &Sheet, log: &PracticeLog) -> SheetSession {
    let size = Layout::laid_ahead(record);
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(record, log.packs());
    let opening = log.practising(record, &printed, (size, &Title::untitled()));
    SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT)
}

/// Today's lesson of lines, on day `day`, as `log` starts it, done by Next alone.
fn done_today(log: &PracticeLog, day: u64, id: u64) -> SheetSession {
    let now = NOON + day * DAY;
    let (sheet, _) = log.start(Start::Skill(lines()), 3, now).expect("a lesson");
    let record = Sheet {
        id,
        canvas: 900 + id,
        ..sheet
    };
    let mut session = opened(&record, log);
    while session.projection().current.is_some() {
        (session.practise(PracticeIntent::Next(VIEWPORT), now + 300)).expect("next");
    }
    session
}

fn log_of(sheets: Vec<Sheet>, pairs: Vec<BlindPair>, day: u64) -> PracticeLog {
    let mut log = PracticeLog::new(sheets, Vec::new(), Vec::new());
    log.set_pairs(pairs);
    log.set_clock(NOON + day * DAY, 0);
    log
}

#[test]
fn a_lesson_s_end_asks_one_pair_of_earlier_copies_kept_once_when_answered() {
    let near = [Outcome::Near; 4];
    let log = log_of(vec![past(1, 0, &near), past(2, 28, &near)], Vec::new(), 35);
    let mut session = done_today(&log, 35, 3);
    let end = session.projection().end.expect("the lesson's end");
    assert_eq!(end.pairs.len(), 1, "one pair");
    let pair = end.pairs[0];
    assert_eq!(
        (pair.asked.older.sheet, pair.asked.newer.sheet),
        (1, 2),
        "never today's"
    );
    assert_eq!(pair.answer, None);
    assert_eq!(
        pair.older_left,
        older_left(pair.asked.older, pair.asked.newer)
    );
    assert!(end.drill.is_none(), "no fall");
    assert!(
        session.sheet_mut().pairs_request().is_empty(),
        "skipped stores nothing"
    );
    let cards = session
        .copies_request()
        .expect("its copies asked of the store");
    assert_eq!(cards, [pair.asked.older, pair.asked.newer]);
    assert!(session.copies_request().is_none(), "once");
    let answered = PracticeIntent::Pair(0, Answer::Left);
    session
        .practise(answered, NOON + 35 * DAY + 400)
        .expect("answered");
    let end = session.projection().end.expect("the end");
    assert_eq!(end.pairs[0].answer, Some(Answer::Left), "its dates show");
    let stored = session.sheet_mut().pairs_request();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].lesson, (3, 0));
    assert!(
        session.sheet_mut().pairs_request().is_empty(),
        "stored once"
    );
    assert!(
        session.practise(answered, NOON + 35 * DAY + 500).is_err(),
        "answered once"
    );
    reopened(session.sheet().record(), stored);
}

/// `record`, its lesson done and its pair answered as `stored`, opened again: the pair as
/// answered; without the answer stored, none.
fn reopened(record: &Sheet, stored: Vec<BlindPair>) {
    let near = [Outcome::Near; 4];
    let sheets = vec![past(1, 0, &near), past(2, 28, &near), record.clone()];
    let kept = log_of(sheets.clone(), stored, 35);
    let shown = opened(record, &kept).projection().end.expect("the end");
    assert_eq!(shown.pairs.len(), 1);
    assert_eq!(shown.pairs[0].answer, Some(Answer::Left));
    let unanswered = log_of(sheets, Vec::new(), 35);
    let shown = (opened(record, &unanswered).projection().end).expect("the end");
    assert!(shown.pairs.is_empty(), "nothing asked anew");
}

#[test]
fn a_pair_skipped_is_not_asked_again_that_day() {
    let near = [Outcome::Near; 4];
    let mut log = log_of(vec![past(1, 0, &near), past(2, 28, &near)], Vec::new(), 35);
    let first = done_today(&log, 35, 3);
    let skipped = first.sheet().offered();
    log.asked(skipped.clone());
    let again = done_today(&log, 35, 4);
    let end = again.projection().end.expect("the end");
    for pair in &end.pairs {
        for copy in [pair.asked.older, pair.asked.newer] {
            let seen = skipped.iter().any(|s| s.older == copy || s.newer == copy);
            assert!(!seen, "{copy:?} asked again the same day");
        }
    }
}

#[test]
fn the_drill_shows_only_where_the_curve_is_sure_the_skill_fell() {
    let clean = [Outcome::Clean; 6];
    let redo = [Outcome::Redo; 6];
    let mut sheets: Vec<Sheet> = (0..7).map(|i| past(i + 1, i * 2, &clean)).collect();
    sheets.extend((0..5).map(|i| past(i + 20, 26 + i * 2, &redo)));
    let fit = SkillFit::of(lines(), &sheets, &[], 0).expect("a fit");
    assert!(matches!(
        fit.change(),
        Some(openpaint_practice::Change::Fell { .. })
    ));
    let log = log_of(sheets, Vec::new(), 36);
    let end = done_today(&log, 36, 40).projection().end.expect("the end");
    let drill = end.drill.expect("an extra drill");
    assert_eq!(drill.skill, lines());
    assert!(drill.exercises > 0);
}

#[test]
fn a_unit_s_end_re_test_is_offered_once_its_seal_is_earned_and_laid_as_its_set() {
    let log = log_of(Vec::new(), Vec::new(), 0);
    assert!(log.path().retest_end.is_none(), "nothing sealed");
    assert!(log.path().retest.is_none());
    // Unit 1 sealed (each skill with its first dot): its end re-test offered beside the next
    // lesson, unit 2's start in the next lesson's place, which Start lays.
    let clean = [Outcome::Clean; 6];
    let colour = Kind::Colour(openpaint_practice::Tone::of_code(1).expect("a tone"));
    let combined = Kind::Combined(openpaint_practice::Combination::ALL[0]);
    let sealed = vec![
        Sheet::begun(
            1,
            (Purpose::Day, NOON),
            openpaint_practice::schedule::first(1),
        ),
        past(2, 1, &clean),
        past_of(combined, (3, 1), &clean),
        past_of(colour, (4, 2), &clean),
    ];
    let log = log_of(sealed, Vec::new(), 3);
    let path = log.path();
    let end = path.retest_end.expect("unit 1's end re-test");
    assert_eq!((end.unit, end.end, end.skills.len()), (1, true, 3));
    let start = path.retest.expect("unit 2's start re-test");
    assert_eq!((start.unit, start.end), (2, false));
    let (next, _) = log
        .start(Start::Next, 9, NOON + 3 * DAY)
        .expect("Start's lesson");
    let picked = next.lesson().map(|lesson| lesson.picked);
    assert_eq!(
        picked,
        Some(Picked::Retest {
            unit: 2,
            end: false
        })
    );
    let start = Start::Retest(3, false);
    let (sheet, _) = log.start(start, 5, NOON).expect("unit 3's re-test");
    assert_eq!(
        sheet.lesson().map(|l| l.picked),
        Some(Picked::Retest {
            unit: 3,
            end: false
        })
    );
    let set =
        openpaint_practice::retest::retest(3, false, &[], &openpaint_practice::Pictures::default())
            .expect("its set");
    let laid: Vec<&Exercise> = sheet.exercises().map(|laid| &laid.exercise).collect();
    let planned: Vec<&Exercise> = set.exercises.iter().map(|(exercise, _)| exercise).collect();
    assert_eq!(laid, planned);
}

/// A learner of 120 practice days, four results a day and a pair every second day reaching
/// three to five weeks back: a few hundred results.
fn long_history() -> (Vec<Sheet>, Vec<BlindPair>) {
    let outcomes = [Outcome::Clean, Outcome::Near, Outcome::Near, Outcome::Redo];
    let sheets: Vec<Sheet> = (0..120).map(|day| past(day + 1, day, &outcomes)).collect();
    let pairs = (40..120)
        .step_by(2)
        .map(|day| BlindPair {
            asked: NOON + day * DAY + 1000,
            lesson: (day + 1, 0),
            older: CopyRef {
                sheet: day + 1 - 21 - day % 15,
                exercise: 1,
            },
            newer: CopyRef {
                sheet: day,
                exercise: 2,
            },
            older_left: true,
            answer: [Answer::Left, Answer::Alike, Answer::Right][(day % 3) as usize],
        })
        .collect();
    (sheets, pairs)
}

#[test]
fn the_fit_of_a_long_history_takes_well_under_a_frame() {
    let (sheets, pairs) = long_history();
    let mut times: Vec<f64> = (0..15)
        .map(|_| {
            let begun = Instant::now();
            let fit = SkillFit::of(lines(), &sheets, &pairs, 0).expect("a fit");
            assert!(fit.steps() < 30, "{} steps", fit.steps());
            begun.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    println!(
        "a skill of 120 days, 480 results and 40 pairs fitted in {median:.2} ms (median of 15)"
    );
    // A frame at 120 Hz is 8.3 ms; the gate runs every test binary at once beside it.
    assert!(median < 8.3, "{median:.2} ms");
    let begun = Instant::now();
    let progress = Progress::of(&sheets, &pairs, 0);
    println!(
        "the book's curves in {:.2} ms",
        begun.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(progress.curves.len(), 1);
}
