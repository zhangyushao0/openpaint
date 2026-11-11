//! Lessons counted, not sheets (S121b): a lesson appended to a sheet is laid from the row after
//! the last one used, its givens moved with it, and every count walks lessons in the order of
//! sheet and lesson: numbers, the tally and the seals a finish stamps, the book's road and lines,
//! the summary's earlier lessons.

use openpaint_practice::{
    Book, Exercise, Family, First, Given, Givens, Kind, Layout, Lesson, LineShown, LineTier, Plan,
    Purpose, Seal, Sheet, Tally, lessons, number, sealed, stage_one,
};

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A drill of four single lines, the `id`th sheet, finished.
fn line_drill(id: u64) -> Sheet {
    let exercises = (0..4)
        .map(|seed| Exercise::generated(Kind::Line(LineTier::Arc), seed))
        .collect();
    let mut drill = Sheet::drill(
        id,
        NOON,
        Plan {
            exercises,
            warm_up: 0,
        },
    );
    drill.lessons[0].finished = Some(NOON + 300);
    drill
}

/// A first session of stage 1 as a lesson to append, seeded `seed`.
fn session_lesson(seed: u64) -> Lesson {
    Sheet::session(0, 1, NOON + 600, stage_one(seed, &[]))
        .lessons
        .remove(0)
}

/// `sheet`'s exercise `index` compared for the first time.
fn compared(sheet: &mut Sheet, index: usize) {
    let laid = sheet.laid_mut(index).expect("an exercise");
    laid.compared(First::of(NOON + 400, 1, Vec::new()));
}

#[test]
fn a_lesson_appended_is_laid_from_a_new_row_with_its_givens() {
    let mut sheet = line_drill(1);
    let first = Givens::of_lesson(&sheet.lessons[0], &[]);
    assert_eq!(Layout::next_slot(&sheet), 6, "two rows used");
    let lesson = session_lesson(5);
    let count = lesson.exercises.len();
    let givens = Givens::of_lesson(&lesson, &[]);
    let moved = sheet.append(lesson, givens);
    let slots: Vec<u16> = moved.slots.iter().map(|(slot, _)| *slot).collect();
    let laid: Vec<u16> = sheet.lessons[1].exercises.iter().map(|l| l.slot).collect();
    assert_eq!(slots, laid, "the givens moved with their exercises");
    assert_eq!(laid[0], 6);
    assert_eq!((sheet.in_hand(), sheet.at()), (1, 4));
    assert_eq!((sheet.stage(), sheet.drilled()), (Some(1), None));
    // One pair an exercise, at its slot: the second lesson starts the third row's first column.
    let layout = Layout::of_sheet(&sheet);
    assert_eq!(layout.pairs.len(), 4 + count);
    assert_eq!(layout.pairs[4], Layout::pair_at(6, None));
    assert_eq!(
        layout.pairs[4].original.origin[0],
        layout.pairs[0].original.origin[0]
    );
    assert!(layout.pairs[4].original.origin[1] > layout.pairs[3].original.origin[1]);
    assert_eq!(layout.size, Layout::used(&sheet));
    assert_eq!(Layout::rows_on(layout.size), 2 + count.div_ceil(3));
    assert_eq!(Layout::next_slot(&sheet), 3 * (2 + count.div_ceil(3)));
    assert_eq!(Layout::on(&sheet, [2920, 6280]).pairs, layout.pairs);
    assert_eq!(Layout::row_rect(4), [0.0, 520.0, 2920.0, 560.0]);
    assert_eq!(
        Layout::of(&[None]).size,
        [2920, 560],
        "a row's rectangle is a one-row sheet"
    );
    // Its givens resolve for every exercise.
    let printed = Givens::laid(&[first, moved], &sheet).expect("every exercise's");
    assert_eq!(printed, Given::of_sheet(&sheet, &[]));
    // A sheet of the rows used holds no more; twelve rows hold a session after a drill.
    let drill = line_drill(2);
    assert!(!Layout::fits(&drill, count, Layout::used(&drill)));
    assert!(Layout::fits(&drill, count, [2920, 6280]));
    assert_eq!(Layout::rows_on([2920, 6280]), 12);
    // A new day's sheet is laid ahead to twelve rows (S121c); a lesson longer takes its own.
    assert_eq!(Layout::laid_ahead(&drill), [2920, 6280]);
    assert_eq!(
        Layout::rows_on(Layout::laid_ahead(&drill)),
        openpaint_practice::ROWS
    );
    let copy = Sheet::copy(
        3,
        (1, NOON),
        openpaint_practice::ItemRef {
            pack: "test".to_owned(),
            item: "teapot".to_owned(),
        },
    );
    assert!(
        !Layout::fits(&copy, 1, [6000, 6000]),
        "never on a colour copy's"
    );
    assert_eq!(Layout::laid_ahead(&copy), Layout::of_sheet(&copy).size);
}

/// Sheet 1: a line drill, then a session of stage 1 appended and finished with a line compared;
/// sheet 2: a session of stage 1 left open; sheet 3: a line drill, a test's.
fn three_sheets() -> Vec<Sheet> {
    let mut day = line_drill(1);
    compared(&mut day, 0);
    let lesson = session_lesson(5);
    day.append(lesson, Givens::default());
    compared(&mut day, 4);
    day.lessons[1].finished = Some(NOON + 900);
    let open = Sheet::session(2, 1, NOON + 1200, stage_one(6, &[]));
    let test = Sheet {
        purpose: Purpose::Test,
        ..line_drill(3)
    };
    vec![test, open, day]
}

#[test]
fn every_count_sees_each_lesson_in_the_order_of_sheet_and_lesson() {
    let sheets = three_sheets();
    let order: Vec<(u64, usize)> = lessons(&sheets).iter().map(|l| l.order()).collect();
    assert_eq!(order, [(1, 0), (1, 1), (2, 0), (3, 0)]);
    let [test, open, day] = [&sheets[0], &sheets[1], &sheets[2]];
    assert_eq!(number(&sheets, day, 0), 1, "the first line drill");
    assert_eq!(number(&sheets, day, 1), 1, "the first session");
    assert_eq!(number(&sheets, open, 0), 2, "the second session");
    assert_eq!(
        number(&sheets, test, 0),
        2,
        "a test's drill among the drills"
    );
    assert_eq!(test.drilled(), Some(Family::Line));
    // The tally: one session finished, the lines compared in both lessons.
    let tally = Tally::of(&sheets);
    assert_eq!((tally.sessions, tally.stages[0], tally.lines), (1, 1, 2));
    // The session's finish stamps the first session beside every other lesson, its drill's
    // included.
    let others = Tally::beside(&sheets, day);
    assert_eq!((others.sessions, others.lines), (0, 1));
    assert_eq!(sealed(&others, day), [Seal::FirstSession]);
    // The book: the road and the lines are the session lessons', the line's index its sheet's.
    let book = Book::of(&sheets, 1, 0);
    assert_eq!((book.sessions, book.road.done_in(1)), (1, 1));
    assert_eq!(
        book.lines,
        [LineShown {
            record: 1,
            number: 1,
            index: 4
        }]
    );
}
