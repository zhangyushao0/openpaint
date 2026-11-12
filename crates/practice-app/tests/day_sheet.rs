//! The day's sheet (S121c): a sheet is today's when it was started on today's local day; a lesson
//! from the home goes onto today's newest day's sheet once its last lesson is finished (a test's
//! sheet never, nor yesterday's); One more appends the next lesson in place from a new row where
//! the rows left hold it, gliding at the zoom the view had before Finish, its row labelled, the
//! How told once per kind, undo reaching only the exercise shown, the record held back until the
//! store answers and rolled back with the end as it was when the write failed; a sheet from
//! before (its canvas only the rows it uses) never takes a lesson. Every view of a pair fits its
//! row, so a twelve-row sheet shows a pair as a one-row sheet did.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, CanvasView, Fitted, Intent, MAX_ZOOM, Session, Title, Toolbox, ViewGesture,
    ViewIntent, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Exercise, Given, Givens, Kind, Layout, Lesson, LineTier, Picked, Planned, Purpose, Sheet,
    Skill, SkillLevel,
};
use openpaint_practice_app::{
    PracticeIntent, PracticeLog, PracticeNotice, PracticePens, PracticeProjection, PracticeSheet,
    append_lesson, append_stored, open_sheet, practise,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC, and a day.
const NOON: u64 = 1_791_201_600;
const DAY: u64 = 86_400;

/// The log of `sheets` on the clock at `now`, UTC.
fn log_of(sheets: Vec<Sheet>, now: u64) -> PracticeLog {
    let mut log = PracticeLog::new(sheets, Vec::new(), Vec::new());
    log.set_clock(now, 0);
    log
}

/// A lesson of six lines (two rows), as `chosen` lays a lesson of a skill, the next sheet of
/// `log` begun at `started` on project `canvas`.
fn six_lines(log: &PracticeLog, started: u64, canvas: u64) -> Sheet {
    let exercises = (0..6)
        .map(|seed| Exercise::generated(Kind::Line(LineTier::Arc), seed))
        .map(|exercise| (exercise, SkillLevel::First))
        .collect();
    let planned = Planned {
        picked: Picked::Chosen,
        exercises,
    };
    let lesson = Sheet::begun(log.next_id(), (Purpose::Day, started), planned);
    Sheet { canvas, ..lesson }
}

/// `record` open on a canvas of `size` as `log` opens it.
fn practising(record: &Sheet, size: [u32; 2], log: &PracticeLog) -> Session {
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(record, &[]);
    let opening = log.practising(record, &printed, (size, &Title::untitled()));
    open_sheet(sheet, (brush, pens), opening, &VIEWPORT)
}

fn shown(session: &Session) -> PracticeProjection {
    PracticeProjection::of(session).expect("a sheet")
}

fn record_of(session: &Session) -> Sheet {
    PracticeSheet::of(session)
        .expect("a sheet")
        .record()
        .clone()
}

/// Next until the lesson in hand is done.
fn finish(session: &mut Session) {
    while shown(session).current.is_some() {
        practise(session, PracticeIntent::Next(VIEWPORT), NOON + 300).expect("next");
    }
}

/// The next lesson of six lines, and what it prints.
fn next_lesson(log: &PracticeLog) -> (Lesson, Givens) {
    let lesson = six_lines(log, NOON + 600, 0).lessons.remove(0);
    let givens = Givens::of_lesson(&lesson, &[]);
    (lesson, givens)
}

/// A pinch about the room's middle zooming the view by `zoom`.
fn pinch(zoom: f32) -> Intent {
    Intent::View(ViewIntent::Gesture {
        gesture: ViewGesture {
            pivot: [1076.0, 942.0],
            translation: [0.0; 2],
            zoom,
            rotation: 0.0,
        },
        viewport: VIEWPORT,
    })
}

/// Where `session`'s view shows canvas point `point`, in screen pixels.
fn on_screen(session: &Session, point: [f64; 2]) -> [f32; 2] {
    let canvas = session.artwork().canvas();
    let mapping = session.view().mapping(canvas, &VIEWPORT);
    mapping.to_screen(point.map(|v| v as f32))
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
}

#[test]
fn today_s_sheet_takes_a_lesson_from_the_gallery_once_its_last_is_finished() {
    let empty = log_of(Vec::new(), NOON + 600);
    let mut drill = six_lines(&empty, NOON, 40);
    let unfinished = log_of(vec![drill.clone()], NOON + 600);
    assert!(unfinished.day_sheet().is_none(), "its lesson unfinished");
    drill.lessons[0].finished = Some(NOON + 300);
    let mut test = Sheet {
        id: 2,
        canvas: 41,
        purpose: Purpose::Test,
        ..drill.clone()
    };
    let log = log_of(vec![drill.clone(), test.clone()], NOON + 600);
    assert!(log.is_today(&drill));
    assert_eq!(log.day_sheet().map(|s| s.id), Some(1), "a test's never");
    test.purpose = Purpose::Copy;
    let log = log_of(vec![test], NOON + 600);
    assert!(log.day_sheet().is_none(), "a colour copy's never");
    // After midnight the morning's sheet is yesterday's.
    let tomorrow = log_of(vec![drill.clone()], NOON + DAY / 2 + 60);
    assert!(!tomorrow.is_today(&drill));
    assert!(tomorrow.day_sheet().is_none());
    // A day is local: at 14:00 UTC, 11 hours east, the sheet begun at 23:00 is yesterday's.
    let mut east = log_of(vec![drill], NOON + 2 * 3600);
    assert!(east.day_sheet().is_some(), "the same day in UTC");
    east.set_clock(NOON + 2 * 3600, 11 * 60);
    assert!(east.day_sheet().is_none());
}

#[test]
fn one_more_appends_in_place_from_a_new_row_at_the_zoom_before_finish() {
    let empty = log_of(Vec::new(), NOON + 600);
    let drill = six_lines(&empty, NOON, 40);
    let mut log = log_of(vec![drill.clone()], NOON + 600);
    let mut session = practising(&drill, Layout::laid_ahead(&drill), &log);
    assert_eq!(
        session.artwork().canvas().height(),
        6280,
        "twelve rows laid ahead"
    );
    practise(&mut session, PracticeIntent::Instructions(false), NOON).expect("closed");
    session.dispatch(pinch(1.5)).expect("pinch");
    let zoom = session.view().zoom();
    finish(&mut session);
    log.keep(record_of(&session));
    let finished_view = session.view();
    assert_eq!(
        finished_view.fits().map(|f| f.size),
        Some([2920, 1080]),
        "the finish fits the two rows used"
    );
    let (lesson, givens) = next_lesson(&log);
    let (record, moved) =
        append_lesson(&mut session, (lesson, givens), &log, &VIEWPORT).expect("in place");
    assert_eq!(record.lessons.len(), 2);
    assert_eq!(record.lessons[1].exercises[0].slot, 6, "from the third row");
    assert_eq!(moved.slots[0].0, 6);
    let practice = shown(&session);
    let lesson = practice.lesson.expect("the lesson in hand");
    assert_eq!((practice.current, lesson.place), (Some(6), (1, 6)));
    assert_eq!((practice.sheet.pairs.len(), practice.revealed), (12, 7));
    assert_eq!(practice.how, None, "the lines' How told once");
    // Its row labelled after the first: the second lesson of lines.
    let lines = Skill::of_key("line").expect("the lines' skill");
    assert_eq!(practice.sheet.rows.len(), 2);
    let labelled = practice.sheet.rows[1];
    assert_eq!(
        (labelled.slot, labelled.number, labelled.skill),
        (6, 2, Some(lines))
    );
    assert_eq!(lesson.row, labelled);
    let view = session.view();
    assert!((view.zoom() - zoom).abs() < 1e-6, "the zoom before Finish");
    assert_eq!(view.fits(), Some(row(6)));
    let pair = practice.sheet.pairs[6];
    assert!(close(on_screen(&session, pair.centre()), [1076.0, 942.0]));
    // Undo reaches only the exercise shown; the record waits for the store.
    assert_eq!(session.history().undo_steps(), 0);
    let sheet = PracticeSheet::of_mut(&mut session).expect("a sheet");
    assert_eq!(sheet.save_request(), None, "held until the store answers");
    assert!(practise(&mut session, PracticeIntent::OneMore(VIEWPORT), NOON).is_err());
    append_stored(&mut session, record.id, true);
    let saved = PracticeSheet::of_mut(&mut session).and_then(PracticeSheet::save_request);
    assert_eq!(saved.map(|s| s.lessons.len()), Some(2));
}

#[test]
fn a_failed_write_rolls_the_lesson_back_with_the_failed_save_s_notice() {
    let empty = log_of(Vec::new(), NOON + 600);
    let drill = six_lines(&empty, NOON, 40);
    let mut log = log_of(vec![drill.clone()], NOON + 600);
    let mut session = practising(&drill, Layout::laid_ahead(&drill), &log);
    finish(&mut session);
    log.keep(record_of(&session));
    let finished_view = session.view();
    let ended = shown(&session).end.expect("the first lesson's end");
    let (record, _) =
        append_lesson(&mut session, next_lesson(&log), &log, &VIEWPORT).expect("in place");
    assert!(shown(&session).end.is_none(), "the next lesson in hand");
    append_stored(&mut session, record.id, false);
    let practice = shown(&session);
    assert_eq!(practice.current, None);
    assert_eq!(practice.end, Some(ended), "its end as it was");
    assert_eq!(practice.sheet.rows.len(), 1);
    assert_eq!(practice.notice, Some(PracticeNotice::NotSaved));
    assert_eq!(practice.sheet.pairs.len(), 6);
    assert_eq!(session.view(), finished_view, "the view as before One more");
    let saved = PracticeSheet::of_mut(&mut session).and_then(PracticeSheet::save_request);
    assert_eq!(saved.map(|s| s.lessons.len()), Some(1));
}

#[test]
fn a_lesson_goes_on_a_new_sheet_when_it_does_not_fit_or_the_sheet_is_old_or_yesterday_s() {
    let empty = log_of(Vec::new(), NOON + 600);
    let drill = six_lines(&empty, NOON, 40);
    let log = log_of(vec![drill.clone()], NOON + 600);
    // A sheet from before: its canvas holds only the rows it uses.
    let mut old = practising(&drill, Layout::used(&drill), &log);
    finish(&mut old);
    assert!(append_lesson(&mut old, next_lesson(&log), &log, &VIEWPORT).is_none());
    // Yesterday's: One more after midnight starts a new sheet.
    let tomorrow = log_of(vec![drill.clone()], NOON + DAY);
    let mut late = practising(&drill, Layout::laid_ahead(&drill), &tomorrow);
    finish(&mut late);
    assert!(append_lesson(&mut late, next_lesson(&tomorrow), &tomorrow, &VIEWPORT).is_none());
    // Six drills of two rows fill the twelve: a seventh does not fit.
    let mut full = drill.clone();
    for _ in 1..6 {
        if let Some(last) = full.lessons.last_mut() {
            last.finished = Some(NOON + 60);
        }
        full.append(next_lesson(&log).0, Givens::default());
    }
    assert_eq!(Layout::next_slot(&full), 36);
    let mut filled = practising(&full, Layout::laid_ahead(&drill), &log);
    finish(&mut filled);
    assert!(append_lesson(&mut filled, next_lesson(&log), &log, &VIEWPORT).is_none());
    // A test's sheet appends a test lesson in place.
    let test = Sheet {
        purpose: Purpose::Test,
        ..drill
    };
    let mut tested = practising(&test, Layout::laid_ahead(&test), &log);
    finish(&mut tested);
    let (record, _) = (append_lesson(&mut tested, next_lesson(&log), &log, &VIEWPORT))
        .expect("a test lesson in place");
    assert_eq!((record.purpose, record.lessons.len()), (Purpose::Test, 2));
}

/// The rectangle of slot `slot`'s row.
fn row(slot: usize) -> Fitted {
    let [x, y, width, height] = Layout::row_rect(slot);
    Fitted {
        origin: [x, y].map(|v| v as u32),
        size: [width, height].map(|v| v as u32),
    }
}

#[test]
fn every_view_of_a_pair_fits_its_row_as_a_one_row_sheet_did() {
    let empty = log_of(Vec::new(), NOON + 600);
    let drill = six_lines(&empty, NOON, 40);
    let log = log_of(vec![drill.clone()], NOON + 600);
    let mut tall = practising(&drill, Layout::laid_ahead(&drill), &log);
    // A sheet from before of one row: the first row exactly.
    let mut one = practising(&drill, Layout::of(&[None]).size, &log);
    let pairs = shown(&tall).sheet.pairs.clone();
    let same =
        |a: &Session, b: &Session, point: [f64; 2]| close(on_screen(a, point), on_screen(b, point));
    assert_eq!(
        tall.view().fits(),
        Some(row(0)),
        "opening on the row in hand"
    );
    assert!(same(&tall, &one, pairs[0].copy.origin));
    assert!(same(&tall, &one, pairs[2].copy.middle()));
    // The zoom's limits are a row's: as far in as on the one-row sheet.
    for session in [&mut tall, &mut one] {
        session.dispatch(pinch(100.0)).expect("pinch");
        assert_eq!(session.view().zoom(), MAX_ZOOM);
    }
    let canvas = |s: &Session| s.artwork().canvas();
    let scale = |s: &Session| s.view().mapping(canvas(s), &VIEWPORT).scale();
    assert!((scale(&tall) - scale(&one)).abs() < 1e-4);
    // A quick pinch fits the row in hand.
    tall.dispatch(Intent::View(ViewIntent::Fit)).expect("fit");
    assert_eq!(tall.view(), CanvasView::fitting(Some(row(0))));
    // Next on to the second row: the row fitted moves, the zoom kept.
    for _ in 0..3 {
        practise(&mut tall, PracticeIntent::Next(VIEWPORT), NOON + 60).expect("next");
    }
    assert_eq!(tall.view().fits(), Some(row(3)));
    assert!((tall.view().zoom() - 1.0).abs() < 1e-6);
    assert!(close(on_screen(&tall, pairs[3].centre()), [1076.0, 942.0]));
}
