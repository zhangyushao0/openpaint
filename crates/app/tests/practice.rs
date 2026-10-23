//! A practice sheet's session: it opens on its pair, the pen draws only in the empty frame and
//! keeps its own samples, a line's own tools draw with the practice pens, Compare lays the
//! original over the copy in place or aligned and stores the result once, the pen turns blue and
//! corrections are marked, Compare comes again, Next keeps the zoom, an original is picked only
//! once compared, a colour's frame is filled from beside it, the full interface draws with the
//! toolbox's brush, and the last Next ends the session with its summary.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Overlay, Pen, PracticeIntent,
    PracticeNotice, PracticePens, PracticePick, Session, StreamLine, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{Metric, Metrics, Original, Pair, SessionRecord, Sheet, value};

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

fn pens() -> PracticePens {
    PracticePens::of(&BrushLibrary::built_in().expect("built-in brushes")).expect("pens")
}

fn practising() -> Session {
    let record = SessionRecord::stage_one(1, 1, 0x5EED_0045, 1_791_201_600);
    let [width, height] = Sheet::of(record.exercises.len()).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let tools = (brush(Color::BLACK), pens());
    Session::practising(sheet, tools, (record, Vec::new()), &VIEWPORT)
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

/// Every stroke's id and colour, oldest first.
fn strokes(session: &Session) -> Vec<(u64, Color)> {
    let artwork = session.artwork();
    let mut all: Vec<(u64, Color)> = (artwork.layers().iter())
        .flat_map(|l| l.strokes().iter())
        .map(|s| (s.id().get(), s.style().color()))
        .collect();
    all.sort_by_key(|(id, _)| *id);
    all
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

fn close(a: [f32; 2], b: [f32; 2], within: f32) -> bool {
    (a[0] - b[0]).abs() < within && (a[1] - b[1]).abs() < within
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_201_700).expect("practised");
}

/// The overlay's line, while it shows.
fn overlay(session: &Session) -> Option<Vec<[f32; 2]>> {
    match session.projection().practice()?.overlay.clone()? {
        Overlay::Line(points) => Some(points),
        Overlay::Colour { .. } => None,
    }
}

/// A sheet whose first line was copied, moved 30 px right and drawn smaller about its frame's
/// middle, with StreamLine asked for and not used: the session, the line's frames, the original
/// line (frame units) and the copy's points (canvas pixels).
fn copied() -> (Session, Pair, Vec<[f64; 2]>, Vec<[f32; 2]>) {
    let mut session = practising();
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    let (pair, original) = (practice.sheet.pairs[0], practice.sheet.originals[0].clone());
    assert!(close(on_screen(&session, pair.centre()), ROOM_MIDDLE, 0.5));
    assert_eq!(
        practice.instructions,
        Some(true),
        "lines' instructions first"
    );
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    // On the original: nothing drawn.
    draw(&mut session, &[[150.0, 150.0], [300.0, 300.0]]);
    assert!(strokes(&session).is_empty());
    assert_eq!(notice(&session), Some(PracticeNotice::DrawInFrame));
    assert!(session.practise(PracticeIntent::Compare(true), 1).is_err());
    // The copy: the original moved 30 px right and drawn smaller about its frame's middle;
    // StreamLine asked for and not used.
    session.set_streamline(StreamLine::new(1.0).expect("streamline"));
    let Original::Line(line) = original else {
        panic!("a line first")
    };
    let copy: Vec<[f32; 2]> = (line.points.iter().step_by(6))
        .chain(line.points.last())
        .map(|p| {
            pair.copy
                .to_canvas([0.5 + (p[0] - 0.5) * 0.8 + 0.075, 0.5 + (p[1] - 0.5) * 0.8])
        })
        .map(|p| p.map(|v| v as f32))
        .collect();
    draw(&mut session, &copy);
    let pens = pens();
    assert_eq!(
        strokes(&session)[0].1,
        pens.style(Pen::Draw).color(),
        "the drawing pen"
    );
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
    assert_eq!(drawn, copy, "the pen's own samples");
    (session, pair, line.points, copy)
}

#[test]
fn a_line_is_compared_in_place_and_aligned_and_its_pen_turns_blue() {
    let (mut session, pair, line, copy) = copied();
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.practice_save_request().expect("the record changed");
    let result = record.result(0).expect("compared");
    assert_eq!(
        (result.compares, result.metrics.len()),
        (1, 0),
        "no numbers for lines"
    );
    assert!(result.corrected_from.is_some(), "corrections marked");
    // In place: the original as it lies in its frame, moved into the copy's.
    let in_place = overlay(&session).expect("the overlay");
    let placed = pair.copy.to_canvas(line[0]).map(|v| v as f32);
    assert!(close(in_place[0], placed, 0.01));
    // Aligned: onto the copy.
    practise(&mut session, PracticeIntent::Aligned(true));
    let aligned = overlay(&session).expect("the overlay");
    assert!(
        close(aligned[0], copy[0], 1.0),
        "{:?} {:?}",
        aligned[0],
        copy[0]
    );
    assert!(close(aligned[480], copy[copy.len() - 1], 1.0));
    let projection = session.projection();
    assert_eq!(
        projection.practice().and_then(|p| p.pen).map(|p| p.0),
        Some(Pen::Correct),
        "the pen turned blue"
    );
}

#[test]
fn a_compared_line_is_corrected_in_blue_compared_again_and_left_by_next() {
    let (mut session, _, _, copy) = copied();
    practise(&mut session, PracticeIntent::Compare(true));
    practise(&mut session, PracticeIntent::Aligned(true));
    let record = session.practice_save_request().expect("the record changed");
    let first_correction = record.result(0).and_then(|r| r.corrected_from);
    // Drawing in the frame takes the comparison away and is a correction, in blue.
    draw(&mut session, &copy[..20]);
    assert!(overlay(&session).is_none(), "corrected by eye");
    let (id, colour) = strokes(&session)[1];
    assert_eq!(colour, pens().style(Pen::Correct).color());
    let record = session.practice_save_request();
    assert!(record.is_none(), "a stroke changes no record");
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.practice_save_request().expect("the record changed");
    let result = record.result(0).expect("compared");
    assert_eq!(result.compares, 2);
    assert_eq!(
        result.corrected_from, first_correction,
        "kept from the first"
    );
    assert!(result.corrects(id) && !result.corrects(strokes(&session)[0].0));
    assert!(overlay(&session).is_some(), "compared again, still aligned");
    let zoom = session.view().zoom();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    assert_eq!(session.view().zoom(), zoom, "the zoom kept");
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    assert_eq!((practice.current, practice.place), (Some(1), (2, 10)));
    assert_eq!(
        practice.pen.map(|p| p.0),
        Some(Pen::Draw),
        "back to the drawing pen"
    );
    assert!(close(
        on_screen(&session, practice.sheet.pairs[1].centre()),
        ROOM_MIDDLE,
        0.5
    ));
}

#[test]
fn the_sketch_pen_and_the_full_interface_draw_their_own_way() {
    let mut session = practising();
    let pair = session
        .projection()
        .practice()
        .expect("a sheet")
        .sheet
        .pairs[0];
    let at = pair.copy.to_canvas([0.5, 0.5]).map(|v| v as f32);
    assert!(
        session
            .practise(PracticeIntent::Pen(Pen::Correct), 1)
            .is_err(),
        "not before Compare"
    );
    practise(&mut session, PracticeIntent::Pen(Pen::Sketch));
    draw(&mut session, &[at, [at[0] + 40.0, at[1]]]);
    practise(&mut session, PracticeIntent::AllTools(true));
    assert_eq!(session.projection().practice().and_then(|p| p.pen), None);
    draw(&mut session, &[at, [at[0], at[1] + 40.0]]);
    practise(&mut session, PracticeIntent::AllTools(false));
    draw(&mut session, &[at, [at[0] - 40.0, at[1]]]);
    let colours: Vec<Color> = strokes(&session).into_iter().map(|(_, c)| c).collect();
    let sketch = pens().style(Pen::Sketch).color();
    assert_eq!(
        colours,
        vec![sketch, Color::BLACK, sketch],
        "the pen chosen is kept"
    );
    // Only construction lines: compared in place, the aligned fit has nothing to fit.
    practise(&mut session, PracticeIntent::Compare(true));
    practise(&mut session, PracticeIntent::Aligned(true));
    assert!(overlay(&session).is_some());
}

#[test]
fn a_colour_is_filled_from_beside_its_frame_picked_once_compared_and_summed_up() {
    let mut session = practising();
    for _ in 0..10 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    assert_eq!((practice.line, practice.place), (false, (1, 8)));
    assert_eq!(
        practice.instructions,
        Some(false),
        "colours' instructions first"
    );
    assert_eq!(practice.pen, None, "colours have no pens");
    let pair = practice.sheet.pairs[10];
    let Original::Colour(swatch) = practice.sheet.originals[10] else {
        panic!("a colour")
    };
    let middle = pair.original.to_canvas([0.5, 0.5]).map(|v| v as f32);
    assert_eq!(session.practice_pick(middle), PracticePick::Refused);
    assert_eq!(notice(&session), Some(PracticeNotice::PickAfterCompare));
    let colour = |[r, g, b]: [u8; 3]| {
        let c = |v: u8| f32::from(v) / 255.0;
        Color::rgb(c(r), c(g), c(b)).expect("colour")
    };
    for laid in [Color::BLACK, colour(swatch)] {
        practise(&mut session, PracticeIntent::Fill(laid));
    }
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.practice_save_request().expect("the record changed");
    let metrics = &record.result(10).expect("compared").metrics;
    assert_eq!(value(metrics, Metric::Corrections), Some(1.0));
    assert_eq!(value(metrics, Metric::FinalDelta), Some(0.0));
    assert!(value(metrics, Metric::FirstDelta).expect("first") > 0.1);
    assert_eq!(
        session.practice_pick(middle),
        PracticePick::Colour(colour(swatch))
    );
    filled_again(&mut session, metrics);
    finished(
        &mut session,
        pair.copy.to_canvas([0.5, 0.5]).map(|v| v as f32),
    );
}

/// The colour shown, compared with `metrics`, filled again: the comparison goes, and Compare
/// again shows the copy as it is now with the numbers as they were.
fn filled_again(session: &mut Session, metrics: &Metrics) {
    practise(session, PracticeIntent::Fill(Color::WHITE));
    assert_eq!(
        session
            .projection()
            .practice()
            .and_then(|p| p.overlay.clone()),
        None
    );
    practise(session, PracticeIntent::Compare(true));
    let record = session.practice_save_request().expect("the record changed");
    assert_eq!(&record.result(10).expect("compared").metrics, metrics);
    let Some(Overlay::Colour { last, .. }) = session
        .projection()
        .practice()
        .and_then(|p| p.overlay.clone())
    else {
        panic!("the colours compared");
    };
    assert_eq!(
        last,
        [255, 255, 255],
        "the overlay shows the copy as it is now"
    );
}

/// `session`, its eleventh exercise compared, taken past its last: the summary shows with the
/// colour judgement, One more is offered and the pen draws nowhere (`at`: a place it lands).
fn finished(session: &mut Session, at: [f32; 2]) {
    assert!(
        session.practise(PracticeIntent::OneMore, 30).is_err(),
        "not done yet"
    );
    for _ in 10..18 {
        practise(session, PracticeIntent::Next(VIEWPORT));
    }
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    assert_eq!(practice.current, None);
    let summary = practice.summary.as_ref().expect("the summary");
    assert_eq!((summary[1].checked, summary[1].held), (1, 8));
    let [this, before] = summary[1].judgement.expect("a judgement");
    assert!(!this.is_empty() && before.is_empty());
    assert!(
        session
            .practise(PracticeIntent::Fill(Color::BLACK), 40)
            .is_err(),
        "no colour shown"
    );
    assert!(
        session
            .practice_save_request()
            .and_then(|r| r.finished)
            .is_some()
    );
    practise(session, PracticeIntent::OneMore);
    draw(session, &[at, [at[0] + 40.0, at[1] + 40.0]]);
    assert_eq!(notice(session), Some(PracticeNotice::Done));
}
