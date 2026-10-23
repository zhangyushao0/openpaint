//! A practice sheet's session: it opens on its pair, the pen draws only in the empty frame and
//! keeps its own samples, Compare scores the copy and stores the result once, a checked frame
//! takes nothing more, Next keeps the zoom, an original is picked only once checked, and the last
//! Next ends the session with its summary.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, PracticeIntent,
    PracticeNotice, PracticePick, Session, StreamLine, StrokeIntent, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{Metric, Original, SessionRecord, Sheet, value};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// The middle of the room left of the practice card in `VIEWPORT`, in screen pixels.
const ROOM_MIDDLE: [f32; 2] = [(2880.0 - 416.0 * 1.75) / 2.0, 84.0 + 1716.0 / 2.0];

fn brush(color: Color) -> StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, color).expect("style")
}

fn practising() -> Session {
    let record = SessionRecord::stage_one(1, 1, 0x5EED_0045, 1_791_201_600);
    let [width, height] = Sheet::of(record.exercises.len()).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    Session::practising(sheet, brush(Color::BLACK), (record, Vec::new()), &VIEWPORT)
}

/// A pen stroke through `points` (canvas pixels), 8 ms apart.
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

fn strokes(session: &Session) -> usize {
    let artwork = session.artwork();
    artwork.layers().iter().map(|l| l.strokes().len()).sum()
}

fn notice(session: &Session) -> Option<PracticeNotice> {
    session.projection().practice().and_then(|p| p.notice)
}

/// Where `point` (canvas pixels) shows now.
fn on_screen(session: &Session, point: [f64; 2]) -> [f32; 2] {
    let canvas = session.artwork().canvas();
    let mapping = session.view().mapping(canvas, &VIEWPORT);
    mapping.to_screen(point.map(|v| v as f32))
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 0.5 && (a[1] - b[1]).abs() < 0.5
}

#[test]
fn lines_are_drawn_raw_in_the_empty_frame_compared_once_and_left_by_next() {
    let mut session = practising();
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    let (pair, original) = (practice.sheet.pairs[0], practice.sheet.originals[0].clone());
    assert!(close(on_screen(&session, pair.centre()), ROOM_MIDDLE));
    assert_eq!(
        practice.instructions,
        Some(true),
        "lines' instructions the first time"
    );
    // On the original: nothing drawn.
    draw(&mut session, &[[150.0, 150.0], [300.0, 300.0]]);
    assert_eq!(strokes(&session), 0);
    assert_eq!(notice(&session), Some(PracticeNotice::DrawInFrame));
    assert!(session.practise(PracticeIntent::Compare(true), 1).is_err());
    // A copy along the original, StreamLine asked for and not used: the pen's own samples.
    session.set_streamline(StreamLine::new(1.0).expect("streamline"));
    let Original::Line(line) = original else {
        panic!("a line first")
    };
    let copy: Vec<[f32; 2]> = (line.points.iter().step_by(6))
        .chain(line.points.last())
        .map(|p| pair.copy.to_canvas(*p).map(|v| v as f32))
        .collect();
    draw(&mut session, &copy);
    assert_eq!(strokes(&session), 1);
    assert_eq!(
        notice(&session),
        None,
        "a stroke that begins lets the refusal go"
    );
    let artwork = session.artwork();
    let layer = artwork.layer(artwork.active()).expect("layer");
    let stroke = layer.strokes().iter().next().expect("stroke");
    let drawn: Vec<[f32; 2]> = (stroke.samples().iter())
        .map(|s| [s.position().x(), s.position().y()])
        .collect();
    assert_eq!(drawn, copy);
    session
        .practise(PracticeIntent::Compare(true), 1_791_201_700)
        .expect("compared");
    let record = session.practice_save_request().expect("the record changed");
    assert!(session.practice_save_request().is_none(), "once");
    let result = record.result(0).expect("checked");
    assert_eq!(result.checked_at, 1_791_201_700);
    assert!(value(&result.metrics, Metric::KeyPoints).expect("key points") < 0.005);
    assert_eq!(value(&result.metrics, Metric::Strokes), Some(1.0));
    // Checked: the frame takes no more.
    draw(&mut session, &copy);
    assert_eq!(strokes(&session), 1);
    assert_eq!(notice(&session), Some(PracticeNotice::Checked));
    let zoom = session.view().zoom();
    session
        .practise(PracticeIntent::Next(VIEWPORT), 1_791_201_800)
        .expect("next");
    assert_eq!(session.view().zoom(), zoom, "the zoom kept");
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    assert_eq!((practice.current, practice.place), (Some(1), (2, 10)));
    let next = practice.sheet.pairs[1];
    assert!(close(on_screen(&session, next.centre()), ROOM_MIDDLE));
    assert_eq!(session.practice_save_request().map(|r| r.current), Some(1));
}

#[test]
fn a_swatch_is_picked_only_once_checked_and_the_last_next_ends_the_session() {
    let mut session = practising();
    for _ in 0..10 {
        session
            .practise(PracticeIntent::Next(VIEWPORT), 10)
            .expect("next");
    }
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    assert_eq!((practice.line, practice.place), (false, (1, 8)));
    assert_eq!(
        practice.instructions,
        Some(false),
        "colours' instructions the first time"
    );
    let pair = practice.sheet.pairs[10];
    let Original::Colour(swatch) = practice.sheet.originals[10] else {
        panic!("a colour")
    };
    let middle = pair.original.to_canvas([0.5, 0.5]).map(|v| v as f32);
    assert_eq!(session.practice_pick(middle), PracticePick::Refused);
    assert_eq!(notice(&session), Some(PracticeNotice::PickAfterCompare));
    let copy = pair.copy.to_canvas([0.5, 0.5]).map(|v| v as f32);
    let colour = |[r, g, b]: [u8; 3]| {
        let c = |v: u8| f32::from(v) / 255.0;
        Color::rgb(c(r), c(g), c(b)).expect("colour")
    };
    for laid in [Color::BLACK, colour(swatch)] {
        session.set_brush(brush(laid));
        draw(&mut session, &[copy, [copy[0] + 40.0, copy[1] + 40.0]]);
    }
    session
        .practise(PracticeIntent::Compare(true), 20)
        .expect("compared");
    let record = session.practice_save_request().expect("the record changed");
    let metrics = &record.result(10).expect("checked").metrics;
    assert_eq!(value(metrics, Metric::Corrections), Some(1.0));
    assert_eq!(value(metrics, Metric::FinalDelta), Some(0.0));
    assert!(value(metrics, Metric::FirstDelta).expect("first") > 0.1);
    assert_eq!(
        session.practice_pick(middle),
        PracticePick::Colour(colour(swatch))
    );
    finished(&mut session, copy);
}

/// `session`, its eleventh exercise checked, taken past its last: the summary shows, One more is
/// offered and the pen draws nowhere (`at`: a place it lands).
fn finished(session: &mut Session, at: [f32; 2]) {
    assert!(
        session.practise(PracticeIntent::OneMore, 30).is_err(),
        "not done yet"
    );
    for _ in 10..18 {
        session
            .practise(PracticeIntent::Next(VIEWPORT), 40)
            .expect("next");
    }
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    assert_eq!(practice.current, None);
    let summary = practice.summary.as_ref().expect("the summary");
    assert_eq!((summary[1].checked, summary[1].held), (1, 8));
    assert_eq!(
        session.practice_save_request().and_then(|r| r.finished),
        Some(40)
    );
    session
        .practise(PracticeIntent::OneMore, 50)
        .expect("one more");
    draw(session, &[at, [at[0] + 40.0, at[1] + 40.0]]);
    assert_eq!(notice(session), Some(PracticeNotice::Done));
}
