//! The practice book's side of a sheet (S89): its time runs on the uses while it shows, a gap over
//! five minutes or the app hidden starting a new span, and goes to the store at least once a
//! minute while it grows; a finished lesson's end stamps the seals its work earned beside every
//! other record; and what the book reads of a sheet: each single line's first copy (corrections
//! left out), each colour's first pick, its frames and marks.

mod mixed;
mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Estimate, Given, Kind, Layout, PAUSE, ScaleTone, Seal, Sheet, Solid, Span, Trick,
};
use openpaint_practice_app::{Mark, PracticeIntent, PracticePens, SheetRead, SheetSession};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// `record` open as a log of `others` opens it.
fn practising(record: &Sheet, others: Vec<Sheet>) -> SheetSession {
    let size = Layout::of(&vec![None; record.exercises().count()]).size;
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let opening = support::opening(record, (&[], others), size);
    SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT)
}

/// A lesson of six lines, four combined lines and six colours, sheet `id`, seeded `seed`.
fn mixed_lesson(id: u64, seed: u64) -> Sheet {
    Sheet {
        id,
        ..support::laid(mixed::mixed(seed), NOON)
    }
}

fn first_session() -> SheetSession {
    practising(&mixed_lesson(1, 0x5EED_0045), Vec::new())
}

/// A pen stroke through `points` (canvas pixels).
fn draw(session: &mut SheetSession, points: &[[f32; 2]]) {
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

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOON + 600).expect("practised");
}

#[test]
fn a_sheet_s_time_runs_on_its_uses_and_reaches_the_store_at_least_once_a_minute() {
    let mut session = first_session();
    session.sheet_mut().practised(NOON);
    let saved = session.sheet_mut().save_request().expect("a span began");
    assert_eq!(
        saved.spans,
        [Span {
            from: NOON,
            to: NOON
        }]
    );
    for second in 1..60 {
        session.sheet_mut().practised(NOON + second);
        assert_eq!(session.sheet_mut().save_request(), None, "{second} s");
    }
    session.sheet_mut().practised(NOON + 60);
    let saved = session.sheet_mut().save_request().expect("a minute");
    assert_eq!(
        saved.spans,
        [Span {
            from: NOON,
            to: NOON + 60
        }]
    );
    // Left open and untouched two hours: they count nothing.
    let back = NOON + 60 + 2 * 3600;
    session.sheet_mut().practised(back);
    let saved = session.sheet_mut().save_request().expect("a span began");
    assert_eq!(saved.spans.len(), 2);
    session.sheet_mut().practised(back + 30);
    assert_eq!(session.sheet_mut().save_request(), None, "half a minute");
    // Hidden: the time so far goes to the store, and the next use starts again.
    session.sheet_mut().hidden();
    let saved = session.sheet_mut().save_request().expect("hidden");
    assert_eq!(openpaint_practice::seconds(&saved.spans), 60 + 30);
    let seconds = saved.laid(0).map(|laid| laid.seconds);
    assert_eq!(seconds, Some(60 + 30), "the exercise in hand's");
    session.sheet_mut().practised(back + 30 + PAUSE / 2);
    let saved = session.sheet_mut().save_request().expect("a span began");
    assert_eq!(saved.spans.len(), 3);
    assert_eq!(openpaint_practice::seconds(&saved.spans), 60 + 30);
    session.sheet_mut().flush();
    assert_eq!(session.sheet_mut().save_request(), None, "nothing new");
}

#[test]
fn a_finished_lesson_s_end_stamps_the_seal_it_earned() {
    let before: Vec<Sheet> = (1..=9)
        .map(|id| {
            let mut sheet = mixed_lesson(id, id);
            sheet.lessons[0].finished = Some(NOON + id * 86_400);
            sheet
        })
        .collect();
    let record = mixed_lesson(10, 99);
    let count = record.exercises().count();
    let mut session = practising(&record, before);
    for _ in 0..count {
        let end = session.projection().end;
        assert_eq!(end, None, "not done yet");
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    let end = session.projection().end;
    let end = end.expect("the lesson's end");
    assert_eq!(end.sealed, [Seal::TenLessons]);
    assert!(end.next.is_some(), "the lesson One more starts");
}

#[test]
fn the_book_reads_a_sheet_s_first_line_its_first_colour_its_frames_and_marks() {
    let mut session = first_session();
    let pairs = (session.projection().sheet.pairs).clone();
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
    let record = session.sheet_mut().save_request().expect("the record");
    let printed = Given::of_sheet(&record, &[]);
    let read = SheetRead::of(session.session().artwork(), &record, &printed);
    assert_eq!(
        read.size,
        Layout::of(&vec![None; record.exercises().count()]).size
    );
    assert_eq!(read.frames.len(), 2 * record.exercises().count());
    // A kind built in one frame shows that one (S97).
    let mut scaled = record.clone();
    scaled.laid_mut(3).expect("an exercise").exercise.kind = Kind::Scale(ScaleTone::Grey, 9);
    let one_less = SheetRead::of(session.session().artwork(), &scaled, &printed)
        .frames
        .len();
    assert_eq!(one_less, 2 * record.exercises().count() - 1);
    // A trick and a solid alike (S98); an estimate keeps its original beside its copy.
    scaled.laid_mut(4).expect("an exercise").exercise.kind = Kind::Space(Trick::Centre);
    scaled.laid_mut(5).expect("an exercise").exercise.kind = Kind::Solid(Solid::Cone);
    scaled.laid_mut(6).expect("an exercise").exercise.kind = Kind::Flat(Estimate::Point);
    let alike = SheetRead::of(session.session().artwork(), &scaled, &printed)
        .frames
        .len();
    assert_eq!(alike, 2 * record.exercises().count() - 3);
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].index, 0);
    let original = |index| record.laid(index).expect("an exercise").exercise.original();
    assert_eq!(read.lines[0].original, original(0).lines().expect("a line"));
    assert_eq!(read.lines[0].copy.len(), 1, "the correction left out");
    assert!((read.lines[0].copy[0][0][0] - 0.3).abs() < 1e-3);
    let swatch = original(10).swatch().expect("a colour");
    assert_eq!(read.colours, [(swatch, [204, 51, 26])], "the first pick");
    let lines = (read.marks.iter())
        .filter(|mark| matches!(mark, Mark::Line { .. }))
        .count();
    let fills = (read.marks.iter())
        .filter(|mark| matches!(mark, Mark::Fill { .. }))
        .count();
    assert_eq!((lines, fills), (2, 2));
}
