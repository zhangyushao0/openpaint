//! A practice sheet's session: it opens on its pair, the pen draws only in the empty frame and
//! keeps its own samples, a line's own tools draw with the practice pens, Compare lays the
//! original over the copy in place or aligned and stores the result once, the pen turns blue and
//! corrections are marked, Compare comes again, Next keeps the zoom, an original is picked only
//! once compared, a colour's frame is filled from beside it, the full interface draws with the
//! toolbox's brush, and the last Next ends the lesson with each exercise's result.

mod mixed;
mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StreamLine,
    StrokeIntent, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    ColourCompared, Figure, Judge, Kind, Layout, Lightness, LineTier, Metric, Metrics, Outcome,
    Pair, Purity, Tone, value,
};
use openpaint_practice_app::{
    Pen, PracticeIntent, PracticeNotice, PracticePens, PracticePick, PracticeProjection,
};
use support::OnSheet;

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
    PracticePens::built_in().expect("pens")
}

fn practising() -> Session {
    let record = support::laid(mixed::mixed(0x5EED_0045), 1_791_201_600);
    let [width, height] = Layout::of(&vec![None; record.exercises().count()]).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let tools = (brush(Color::BLACK), pens());
    let opening = support::opening(&record, (&[], Vec::new()), [width, height]);
    openpaint_practice_app::open_sheet(sheet, tools, opening, &VIEWPORT)
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
    PracticeProjection::of(session).and_then(|p| p.notice)
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
    fit(session);
}

/// The aligned fit asked for, found as the runtime's worker finds it.
fn fit(session: &mut Session) {
    if let Some(job) = session.sheet().fit_request() {
        session.sheet().fitted(job.sequence, job.run());
    }
}

/// The overlay's line laid over the copy (canvas pixels), while it shows.
fn overlay(session: &Session) -> Option<Vec<[f32; 2]>> {
    let practice = PracticeProjection::of(session)?;
    let frame = practice.sheet.pairs[practice.current?].copy;
    let canvas = |p: &[f64; 2]| frame.to_canvas(*p).map(|v| v as f32);
    practice
        .overlay
        .iter()
        .flatten()
        .find_map(|mark| match &mark.drawn {
            Figure::Line { points, .. } => Some(points.iter().map(canvas).collect()),
            _ => None,
        })
}

/// A sheet whose first line was copied, moved 30 px right and drawn smaller about its frame's
/// middle, with StreamLine asked for and not used: the session, the line's frames, the original
/// line (frame units) and the copy's points (canvas pixels).
fn copied() -> (Session, Pair, Vec<[f64; 2]>, Vec<[f32; 2]>) {
    let mut session = practising();
    let practice = PracticeProjection::of(&session).expect("a practice sheet");
    let (pair, original) = (
        practice.sheet.pairs[0],
        (practice.original.as_deref()).and_then(|o| o.lines().map(<[_]>::to_vec)),
    );
    assert!(close(on_screen(&session, pair.centre()), ROOM_MIDDLE, 0.5));
    let straight = Some(Kind::Line(LineTier::Straight));
    assert_eq!(practice.how, straight, "lines' How first");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    // On the original: nothing drawn.
    draw(&mut session, &[[150.0, 150.0], [300.0, 300.0]]);
    assert!(strokes(&session).is_empty());
    assert_eq!(notice(&session), Some(PracticeNotice::DrawInFrame));
    assert!(session.practise(PracticeIntent::Compare(true), 1).is_err());
    // The copy: the original moved 30 px right and drawn smaller about its frame's middle;
    // StreamLine asked for and not used.
    session.set_streamline(StreamLine::new(1.0).expect("streamline"));
    let Some(mut lines) = original else {
        panic!("a line first")
    };
    let line = lines.remove(0);
    let copy: Vec<[f32; 2]> = (line.iter().step_by(6))
        .chain(line.last())
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
    (session, pair, line, copy)
}

#[test]
fn a_line_is_compared_in_place_and_aligned_and_its_pen_turns_blue() {
    let (mut session, pair, line, copy) = copied();
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.sheet().save_request().expect("the record changed");
    let result = record.first(0).expect("compared");
    let (compares, judged) = (record.laid(0)).map_or((0, None), |l| (l.compares, l.result));
    let kept = (compares, result.metrics.len(), judged);
    assert_eq!(kept, (1, 0, None), "no numbers, judged by the eye");
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
    let projection = PracticeProjection::of(&session);
    assert_eq!(
        projection.as_ref().and_then(|p| p.pen).map(|p| p.0),
        Some(Pen::Correct),
        "the pen turned blue"
    );
}

#[test]
fn a_compared_line_is_corrected_in_blue_compared_again_and_left_by_next() {
    let (mut session, _, _, copy) = copied();
    practise(&mut session, PracticeIntent::Compare(true));
    practise(&mut session, PracticeIntent::Aligned(true));
    let record = session.sheet().save_request().expect("the record changed");
    let first_correction = record.first(0).and_then(|r| r.corrected_from);
    // Drawing in the frame takes the comparison away and is a correction, in blue.
    draw(&mut session, &copy[..20]);
    assert!(overlay(&session).is_none(), "corrected by eye");
    let (id, colour) = strokes(&session)[1];
    assert_eq!(colour, pens().style(Pen::Correct).color());
    let record = session.sheet().save_request();
    assert!(record.is_none(), "a stroke changes no record");
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.sheet().save_request().expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    assert_eq!(
        result.corrected_from, first_correction,
        "kept from the first"
    );
    assert!(result.corrects(id) && !result.corrects(strokes(&session)[0].0));
    assert!(overlay(&session).is_some(), "compared again, still aligned");
    let zoom = session.view().zoom();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    assert_eq!(session.view().zoom(), zoom, "the zoom kept");
    let practice = PracticeProjection::of(&session).expect("a practice sheet");
    let place = practice.lesson.map(|lesson| lesson.place);
    assert_eq!((practice.current, place), (Some(1), Some((2, 6))));
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
    let pair = PracticeProjection::of(&session)
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
    assert_eq!(PracticeProjection::of(&session).and_then(|p| p.pen), None);
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
    let practice = PracticeProjection::of(&session).expect("a practice sheet");
    let muted = Kind::Colour(Tone {
        purity: Purity::Muted,
        lightness: Lightness::Middle,
    });
    let place = practice.lesson.map(|lesson| lesson.place);
    assert_eq!((practice.kind, place), (Some(muted), Some((1, 6))));
    assert_eq!(practice.how, Some(muted), "colours' How first");
    assert_eq!(practice.pen, None, "colours have no pens");
    let pair = practice.sheet.pairs[10];
    let Some(swatch) = practice.original.as_deref().expect("in hand").swatch() else {
        panic!("a colour")
    };
    let middle = pair.original.to_canvas([0.5, 0.5]).map(|v| v as f32);
    assert_eq!(session.sheet().pick(middle), PracticePick::Refused);
    assert_eq!(notice(&session), Some(PracticeNotice::PickAfterCompare));
    let colour = |[r, g, b]: [u8; 3]| {
        let c = |v: u8| f32::from(v) / 255.0;
        Color::rgb(c(r), c(g), c(b)).expect("colour")
    };
    for laid in [Color::BLACK, colour(swatch)] {
        practise(&mut session, PracticeIntent::Fill(laid));
    }
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.sheet().save_request().expect("the record changed");
    let metrics = &record.first(10).expect("compared").metrics;
    assert_eq!(value(metrics, Metric::Corrections), Some(1.0));
    assert_eq!(value(metrics, Metric::FinalDelta), Some(0.0));
    let judged = record.laid(10).and_then(|laid| laid.result);
    assert_eq!(judged, Some((Outcome::Clean, Judge::App)), "clean");
    assert!(value(metrics, Metric::FirstDelta).expect("first") > 0.1);
    assert_eq!(
        session.sheet().pick(middle),
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
    let taken = PracticeProjection::of(session).map(|p| p.overlay.is_none());
    assert_eq!(taken, Some(true));
    practise(session, PracticeIntent::Compare(true));
    let record = session.sheet().save_request().expect("the record changed");
    assert_eq!(&record.first(10).expect("compared").metrics, metrics);
    let projection = PracticeProjection::of(session);
    let compared = (projection.as_ref()).and_then(|p| p.compared.as_ref());
    let Some(ColourCompared { last, .. }) = compared.and_then(|c| c.downcast()) else {
        panic!("the colours compared");
    };
    assert_eq!(*last, [255; 3], "the overlay shows the copy as it is now");
}

/// `session`, its eleventh exercise compared, taken past its last: the end shows each exercise's
/// result (the colour's the app's, the rest none), One more is offered and the pen draws nowhere
/// (`at`: a place it lands).
fn finished(session: &mut Session, at: [f32; 2]) {
    let more = PracticeIntent::OneMore(VIEWPORT);
    assert!(session.practise(more, 30).is_err(), "not done yet");
    for _ in 10..16 {
        practise(session, PracticeIntent::Next(VIEWPORT));
    }
    let practice = PracticeProjection::of(session).expect("a practice sheet");
    assert_eq!(practice.current, None);
    let end = practice.end.as_ref().expect("the end");
    let judged: Vec<(usize, Option<Outcome>)> = (0..16)
        .map(|index| (index, (index == 10).then_some(Outcome::Clean)))
        .collect();
    assert_eq!(end.results, judged);
    assert_eq!(end.skill, None, "a full lesson's");
    assert!(
        session
            .practise(PracticeIntent::Fill(Color::BLACK), 40)
            .is_err(),
        "no colour shown"
    );
    assert!(
        session
            .sheet()
            .save_request()
            .and_then(|r| r.finished())
            .is_some()
    );
    practise(session, PracticeIntent::OneMore(VIEWPORT));
    draw(session, &[at, [at[0] + 40.0, at[1] + 40.0]]);
    assert_eq!(notice(session), Some(PracticeNotice::Done));
}
