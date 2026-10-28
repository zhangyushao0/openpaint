//! The practice book's side of a sheet (S89): its time runs on the uses while it shows, a gap over
//! five minutes or the app hidden starting a new span, and goes to the store at least once a
//! minute while it grows; a finished session shows its dot on its stage's road and the seals its
//! work earned beside every other record; and what the book reads of a sheet: each single line's
//! first copy (corrections left out), each colour's first pick, its frames and marks.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, Finished, InputSample, Intent, Mark,
    PracticeIntent, PracticePens, Session, SheetRead, SheetView, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Original, PAUSE, STAGE_SESSIONS, Seal, SessionRecord, Sheet, Span, Tally,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

fn practising(record: SessionRecord, (before, others): (Vec<SessionRecord>, Tally)) -> Session {
    let [width, height] = Sheet::of(record.exercises.len()).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::of(&library).expect("pens");
    let view = SheetView::of(&record, &[], ([width, height], &Title::untitled()));
    Session::practising(
        sheet,
        (brush, pens),
        (record, before, view, others),
        &VIEWPORT,
    )
}

fn first_session() -> SessionRecord {
    SessionRecord::stage_one(1, 1, 0x5EED_0045, NOON, &[])
}

/// A pen stroke through `points` (canvas pixels).
fn draw(session: &mut Session, points: &[[f32; 2]]) {
    let sample = |index: usize| InputSample {
        position: points[index],
        pressure: 0.6,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    let last = points.len() - 1;
    let steps = [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ];
    for (phase, samples) in steps {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, NOON + 600).expect("practised");
}

#[test]
fn a_sheet_s_time_runs_on_its_uses_and_reaches_the_store_at_least_once_a_minute() {
    let mut session = practising(first_session(), (Vec::new(), Tally::default()));
    session.practised(NOON);
    let saved = session.practice_save_request().expect("a span began");
    assert_eq!(
        saved.time,
        [Span {
            from: NOON,
            to: NOON
        }]
    );
    for second in 1..60 {
        session.practised(NOON + second);
        assert_eq!(session.practice_save_request(), None, "{second} s");
    }
    session.practised(NOON + 60);
    let saved = session.practice_save_request().expect("a minute");
    assert_eq!(
        saved.time,
        [Span {
            from: NOON,
            to: NOON + 60
        }]
    );
    // Left open and untouched two hours: they count nothing.
    let back = NOON + 60 + 2 * 3600;
    session.practised(back);
    let saved = session.practice_save_request().expect("a span began");
    assert_eq!(saved.time.len(), 2);
    session.practised(back + 30);
    assert_eq!(session.practice_save_request(), None, "half a minute");
    // Hidden: the time so far goes to the store, and the next use starts again.
    session.practice_hidden();
    let saved = session.practice_save_request().expect("hidden");
    assert_eq!(openpaint_practice::seconds(&saved.time), 60 + 30);
    session.practised(back + 30 + PAUSE / 2);
    let saved = session.practice_save_request().expect("a span began");
    assert_eq!(saved.time.len(), 3);
    assert_eq!(openpaint_practice::seconds(&saved.time), 60 + 30);
    session.practice_flush();
    assert_eq!(session.practice_save_request(), None, "nothing new");
}

#[test]
fn a_finished_session_shows_its_dot_and_stamps_the_seal_it_earned() {
    let before: Vec<SessionRecord> = (1..=9)
        .map(|id| SessionRecord {
            finished: Some(NOON + id * 86_400),
            ..SessionRecord::stage_one(id, u32::try_from(id).expect("number"), id, NOON, &[])
        })
        .collect();
    let others = Tally::of(&before);
    let record = SessionRecord::stage_one(10, 10, 99, NOON + 10 * 86_400, &[]);
    let count = record.exercises.len();
    let mut session = practising(record, (before, others));
    for _ in 0..count {
        let finished = session
            .projection()
            .practice()
            .and_then(|p| p.finished.clone());
        assert_eq!(finished, None, "not done yet");
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    let finished = session
        .projection()
        .practice()
        .and_then(|p| p.finished.clone());
    assert_eq!(
        finished,
        Some(Finished {
            done: 10,
            of: STAGE_SESSIONS,
            sealed: vec![Seal::TenSessions],
        })
    );
}

#[test]
fn the_book_reads_a_sheet_s_first_line_its_first_colour_its_frames_and_marks() {
    let mut session = practising(first_session(), (Vec::new(), Tally::default()));
    let pairs = (session.projection().practice().expect("sheet").sheet.pairs).clone();
    let at = |frame: usize, [x, y]: [f64; 2]| {
        let copy = pairs[frame].copy;
        copy.to_canvas([x, y]).map(|v| v as f32)
    };
    draw(
        &mut session,
        &[at(0, [0.3, 0.2]), at(0, [0.4, 0.5]), at(0, [0.5, 0.8])],
    );
    practise(&mut session, PracticeIntent::Compare(true));
    // A correction after Compare: not the first copy.
    draw(&mut session, &[at(0, [0.6, 0.2]), at(0, [0.7, 0.8])]);
    for _ in 0..10 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    let picked = Color::rgb(0.8, 0.2, 0.1).expect("colour");
    practise(&mut session, PracticeIntent::Fill(picked));
    practise(&mut session, PracticeIntent::Compare(true));
    let later = Color::rgb(0.1, 0.2, 0.8).expect("colour");
    practise(&mut session, PracticeIntent::Fill(later));
    let record = session.practice_save_request().expect("the record");
    let read = SheetRead::of(session.artwork(), &record);
    assert_eq!(read.size, Sheet::of(record.exercises.len()).size);
    assert_eq!(read.frames.len(), 2 * record.exercises.len());
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].index, 0);
    let Original::Lines(original) = record.exercises[0].original() else {
        panic!("a line");
    };
    assert_eq!(read.lines[0].original, original);
    assert_eq!(read.lines[0].copy.len(), 1, "the correction left out");
    assert!((read.lines[0].copy[0][0][0] - 0.3).abs() < 1e-3);
    let Original::Colour(swatch) = record.exercises[10].original() else {
        panic!("a colour");
    };
    assert_eq!(read.colours, [(swatch, [204, 51, 26])], "the first pick");
    let lines = (read.marks.iter())
        .filter(|mark| matches!(mark, Mark::Line { .. }))
        .count();
    let fills = (read.marks.iter())
        .filter(|mark| matches!(mark, Mark::Fill { .. }))
        .count();
    assert_eq!((lines, fills), (2, 2));
}
