//! Lessons counted, not sheets (S121b, S122): a lesson appended to a sheet is laid from the row
//! after the last one used, its givens moved with it, and every count walks lessons in the order
//! of sheet and lesson: numbers over the learner's lessons, the tally and the seals a finish
//! stamps, the book's lessons and lines.

use openpaint_practice::{
    Book, Exercise, First, Given, Givens, Kind, Layout, Learner, Lesson, LineShown, LineTier,
    Picked, Planned, Purpose, Seal, Sheet, SkillLevel, Tally, lessons, number, sealed,
};

/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A lesson of four single lines, the `id`th sheet, not finished.
fn line_lesson(id: u64) -> Sheet {
    let exercises = (0..4)
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
    Sheet::begun(id, (Purpose::Day, NOON), planned)
}

/// Unit 1's full lesson as an old record holds it (none is laid since S124), as a lesson to
/// append, seeded `seed`: a warm-up of three, then lines.
fn full_lesson(seed: u64) -> Lesson {
    let exercises = (0..7)
        .map(|at| {
            let kind = Kind::Line(LineTier::Arc);
            (Exercise::generated(kind, seed + at), SkillLevel::First)
        })
        .collect();
    let planned = Planned {
        picked: Picked::Full {
            stage: 1,
            warm_up: 3,
        },
        exercises,
    };
    Sheet::begun(0, (Purpose::Day, NOON + 600), planned)
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
    let mut sheet = line_lesson(1);
    let first = Givens::of_lesson(&sheet.lessons[0], &[]);
    assert_eq!(Layout::next_slot(&sheet), 6, "two rows used");
    let lesson = full_lesson(5);
    let count = lesson.exercises.len();
    let givens = Givens::of_lesson(&lesson, &[]);
    let moved = sheet.append(lesson, givens);
    let slots: Vec<u16> = moved.slots.iter().map(|(slot, _)| *slot).collect();
    let laid: Vec<u16> = sheet.lessons[1].exercises.iter().map(|l| l.slot).collect();
    assert_eq!(slots, laid, "the givens moved with their exercises");
    assert_eq!(laid[0], 6);
    assert_eq!((sheet.in_hand(), sheet.at()), (1, 4));
    assert_eq!(sheet.lesson().and_then(Lesson::unit), Some(1));
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
    // A sheet of the rows used holds no more; twelve rows hold a full lesson after a lesson.
    let lines = line_lesson(2);
    assert!(!Layout::fits(&lines, count, Layout::used(&lines)));
    assert!(Layout::fits(&lines, count, [2920, 6280]));
    assert_eq!(Layout::rows_on([2920, 6280]), 12);
    // A new day's sheet is laid ahead to twelve rows (S121c); a lesson longer takes its own.
    assert_eq!(Layout::laid_ahead(&lines), [2920, 6280]);
    assert_eq!(
        Layout::rows_on(Layout::laid_ahead(&lines)),
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

/// Sheet 1: a lesson of lines, then unit 1's full lesson appended and finished, a line compared in
/// each; sheet 2: a full lesson left open; sheet 3: a lesson of lines, a test's; sheet 4: a copy.
fn four_sheets() -> Vec<Sheet> {
    let mut day = line_lesson(1);
    compared(&mut day, 0);
    day.lessons[0].finished = Some(NOON + 300);
    let lesson = full_lesson(5);
    day.append(lesson, Givens::default());
    compared(&mut day, 4);
    day.lessons[1].finished = Some(NOON + 900);
    let mut open = line_lesson(2);
    open.lessons = vec![full_lesson(6)];
    let test = Sheet {
        purpose: Purpose::Test,
        ..line_lesson(3)
    };
    let item = openpaint_practice::ItemRef::default();
    let copy = Sheet::copy(4, (1, NOON), item);
    vec![test, copy, open, day]
}

#[test]
fn every_count_sees_each_lesson_in_the_order_of_sheet_and_lesson() {
    let sheets = four_sheets();
    let order: Vec<(u64, usize)> = lessons(&sheets).iter().map(|l| l.order()).collect();
    assert_eq!(order, [(1, 0), (1, 1), (2, 0), (3, 0), (4, 0)]);
    let [test, copy, open, day] = [&sheets[0], &sheets[1], &sheets[2], &sheets[3]];
    // A lesson's number is its place among the learner's lessons; a copy's among the copies.
    assert_eq!(number(&sheets, day, 0), 1);
    assert_eq!(number(&sheets, day, 1), 2);
    assert_eq!(number(&sheets, open, 0), 3);
    assert_eq!(number(&sheets, test, 0), 4, "a test's among them");
    assert_eq!(number(&sheets, copy, 0), 1);
    assert_eq!(Learner::of(&sheets).lessons(), 4);
    // The tally: the day's two lessons finished, the lines compared in both.
    let tally = Tally::of(&sheets);
    assert_eq!((tally.lessons, tally.lines), (2, 2));
    // The full lesson in hand is set beside every other lesson, its sheet's first included: the
    // first lesson finished was that one, so its finish stamps nothing; the first alone did.
    let others = Tally::beside(&sheets, day);
    assert_eq!((others.lessons, others.lines), (1, 1));
    assert!(sealed(&others, day).is_empty());
    let mut alone = day.clone();
    alone.lessons.truncate(1);
    assert_eq!(sealed(&Tally::default(), &alone), [Seal::FirstLesson]);
    // The book: lessons finished, and the first and the latest lesson's first line compared.
    let book = Book::of(&sheets, &[], 0);
    assert_eq!(book.sessions, 2);
    let shown = |number, index| LineShown {
        record: 1,
        number,
        index,
    };
    assert_eq!(book.lines, [shown(1, 0), shown(2, 4)]);
}
