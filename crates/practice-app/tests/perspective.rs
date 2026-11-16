//! Week 2's perspective on a practice sheet (S84, S145): a box completed from its front corner,
//! drawn with the pens; Compare lays the exact box over it (its vanishing points ringed, never
//! aligned; its construction on request, S97) and waits for the learner's verdict; a page of free
//! boxes compared by its horizon run on; an ellipse compared by the exact one laid. Nothing read
//! from the strokes, no numbers kept, the eye's choice kept once (D123).

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    BoxCompared, BoxView, Compared, Ellipse, EllipseCompared, EllipseGuide, Exercise, Figure,
    Frame, FreeCompared, FreePage, Guide, Horizon, Judge, Kind, LinePen, Outcome, PerspectiveBox,
    Point, SheetMark, Verdict,
};
use openpaint_practice_app::{Judged, Pen, PracticeIntent, PracticePens, SheetSession};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> SheetSession {
    let exercises = vec![
        Exercise::generated(Kind::Box(BoxView::TwoPoint), 0x5EED_0084),
        Exercise::generated(Kind::FreeBoxes(Horizon::Middle), 0x5EED_0085),
        Exercise::generated(Kind::Ellipse(Guide::Frame), 0x5EED_0086),
    ];
    let record = support::laid(exercises, 0);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let opening = support::opening(&record, (&[], Vec::new()), [width, height]);
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT)
}

/// A pen stroke through `points` (frame units of `frame`), sampled finely.
fn draw(session: &mut SheetSession, frame: &Frame, points: &[Point]) {
    let mut path: Vec<[f32; 2]> = Vec::new();
    for pair in points.windows(2) {
        for k in 0..20 {
            let t = f64::from(k) / 20.0;
            let p = [0, 1].map(|i| pair[0][i] + (pair[1][i] - pair[0][i]) * t);
            path.push(frame.to_canvas(p).map(|v| v as f32));
        }
    }
    path.push(frame.to_canvas(points[points.len() - 1]).map(|v| v as f32));
    let sample = |index: usize| InputSample {
        position: path[index],
        pressure: 0.7,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    let last = path.len() - 1;
    for (phase, samples) in [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ] {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        let intent = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(intent).expect("dispatch");
    }
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, 1_791_500_000).expect("practised");
}

fn compared<T: Compared + Clone>(session: &SheetSession) -> Option<T> {
    let practice = session.projection();
    practice.compared.as_ref()?.downcast::<T>().cloned()
}

/// The overlay's marks: its lines in `pen` (whole, dashed) and its rings.
fn laid(session: &SheetSession, pen: LinePen) -> [usize; 3] {
    let overlay = session.projection().overlay.unwrap_or_default();
    let lines = |dashed: bool| {
        let line = |mark: &&SheetMark| matches!(&mark.drawn, Figure::Line { pen: p, dashed: d, .. } if *p == pen && *d == dashed);
        overlay.iter().filter(line).count()
    };
    let rings = (overlay.iter()).filter(|mark| matches!(mark.drawn, Figure::Ring(_)));
    [lines(false), lines(true), rings.count()]
}

/// The eye's three asked, nothing chosen yet; the learner's `outcome` then kept with the record,
/// once.
fn judged_once(session: &mut SheetSession, index: usize, outcome: Outcome) {
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(
        session.projection().judged,
        Some(waiting),
        "the eye's to judge"
    );
    practise(session, PracticeIntent::Judge(outcome));
    let kept = Some((outcome, Judge::Eye));
    assert_eq!(session.projection().judged.and_then(|j| j.result), kept);
    let again = session.practise(PracticeIntent::Judge(Outcome::Clean), 1_791_500_100);
    assert!(again.is_err(), "once");
    let record = session
        .sheet_mut()
        .save_request()
        .expect("the record changed");
    assert_eq!(record.laid(index).and_then(|laid| laid.result), kept);
}

/// The opened sheet's box completed (the nine edges but the front corner's, one of them a little
/// off) and compared; its copy's frame.
fn completed(session: &mut SheetSession) -> Frame {
    let practice = session.projection();
    let frame = practice.sheet.pairs[0].copy;
    let Some(&exact) = practice
        .original
        .as_deref()
        .expect("in hand")
        .downcast::<PerspectiveBox>()
    else {
        panic!("a box")
    };
    let mut drawn = 0;
    for (a, b, _) in exact.edges() {
        if a == exact.corners[exact.front] || b == exact.corners[exact.front] {
            continue;
        }
        let end = if drawn == 0 { [b[0], b[1] + 0.03] } else { b };
        draw(session, &frame, &[a, end]);
        drawn += 1;
    }
    practise(session, PracticeIntent::Compare(true));
    frame
}

#[test]
fn a_box_is_completed_and_compare_lays_the_exact_box_for_the_eye() {
    let mut session = opened();
    let practice = session.projection();
    let two_point = Kind::Box(BoxView::TwoPoint);
    assert_eq!(practice.kind, Some(two_point));
    assert_eq!(practice.how, Some(two_point), "told the first time");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    assert!(!practice.aligned_offered, "geometry is never aligned");
    let frame = completed(&mut session);
    let Some(BoxCompared { exact }) = compared(&session) else {
        panic!("the box laid")
    };
    assert_eq!(
        exact,
        PerspectiveBox::generate(BoxView::TwoPoint, 0x5EED_0084)
    );
    // The exact box over the copy: every edge red (the hidden dashed), its two points ringed,
    // nothing of the copy run on.
    let hidden = exact.hidden().iter().filter(|h| **h).count();
    assert_eq!(laid(&session, LinePen::Answer), [12 - hidden, hidden, 2]);
    let [run_on, dashed, _] = laid(&session, LinePen::Off);
    assert_eq!((run_on, dashed), (0, 0), "nothing of the copy run on");
    let practice = session.projection();
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    assert!(session.practise(PracticeIntent::Aligned(true), 1).is_err());
    // A correction takes the comparison away; Compare again counts, no numbers kept.
    draw(&mut session, &frame, &[[0.4, 0.4], [0.45, 0.47]]);
    assert!(compared::<BoxCompared>(&session).is_none());
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session
        .sheet_mut()
        .save_request()
        .expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(
        (
            record.laid(0).map_or(0, |l| l.compares),
            result.metrics.len()
        ),
        (2, 0)
    );
    judged_once(&mut session, 0, Outcome::Near);
}

/// Construction (S97): off at each Compare, on when asked, off again when tapped again; none
/// to lay with nothing compared.
#[test]
fn a_compared_box_s_construction_shows_on_request() {
    let mut session = opened();
    let construction = |session: &SheetSession| Some(session.projection().construction);
    assert!(
        session
            .practise(PracticeIntent::Construction(true), 1)
            .is_err()
    );
    let frame = completed(&mut session);
    assert_eq!(construction(&session), Some(false));
    practise(&mut session, PracticeIntent::Construction(true));
    assert_eq!(construction(&session), Some(true));
    practise(&mut session, PracticeIntent::Construction(false));
    assert_eq!(construction(&session), Some(false));
    practise(&mut session, PracticeIntent::Construction(true));
    draw(&mut session, &frame, &[[0.4, 0.4], [0.45, 0.47]]);
    assert_eq!(
        construction(&session),
        Some(false),
        "nothing to lay it over"
    );
    assert!(
        session
            .practise(PracticeIntent::Construction(true), 1)
            .is_err()
    );
    practise(&mut session, PracticeIntent::Compare(true));
    assert_eq!(
        construction(&session),
        Some(false),
        "each Compare starts without it"
    );
}

#[test]
fn free_boxes_are_compared_by_their_horizon_run_on_for_the_eye() {
    let mut session = opened();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::FreeBoxes(Horizon::Middle)));
    let frame = practice.sheet.pairs[1].copy;
    let Some(FreePage(example, _)) = practice
        .original
        .as_deref()
        .expect("in hand")
        .downcast::<FreePage>()
    else {
        panic!("a page")
    };
    // The example box drawn twice, side by side, each made smaller about the horizon.
    let h = example.horizon;
    for x in [0.27, 0.73] {
        let map = |p: Point| [x + 0.6 * (p[0] - 0.5), h + 0.6 * (p[1] - h)];
        for (a, b, _) in example.edges() {
            draw(&mut session, &frame, &[map(a), map(b)]);
        }
    }
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(FreeCompared { horizon }) = compared(&session) else {
        panic!("the page compared")
    };
    assert_eq!(horizon, h);
    // The horizon run on, faint, and nothing else: no ring, nothing of the copy.
    assert_eq!(laid(&session, LinePen::Faint), [1, 0, 0]);
    assert_eq!(laid(&session, LinePen::Answer), [0, 0, 0]);
    judged_once(&mut session, 1, Outcome::Clean);
}

#[test]
fn an_ellipse_is_compared_by_the_exact_one_laid_for_the_eye() {
    let mut session = opened();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::Ellipse(Guide::Frame)));
    let frame = practice.sheet.pairs[2].copy;
    let Some(&guide) = practice
        .original
        .as_deref()
        .expect("in hand")
        .downcast::<EllipseGuide>()
    else {
        panic!("an ellipse")
    };
    // Drawn flatter than the frame asks.
    let flatter = Ellipse {
        axes: [guide.exact.axes[0], guide.exact.axes[1] * 0.7],
        ..guide.exact
    };
    draw(&mut session, &frame, &flatter.outline(48));
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(EllipseCompared { guide: laid_over }) = compared(&session) else {
        panic!("the ellipse laid")
    };
    assert_eq!(laid_over, guide);
    // The exact ellipse red, nothing ringed; the middles dotted only under Construction.
    assert_eq!(laid(&session, LinePen::Answer), [1, 0, 0]);
    let dots = |session: &SheetSession| {
        let overlay = session.projection().overlay.unwrap_or_default();
        (overlay.iter())
            .filter(|mark| matches!(mark.drawn, Figure::Dot { .. }))
            .count()
    };
    assert_eq!(dots(&session), 0);
    practise(&mut session, PracticeIntent::Construction(true));
    assert_eq!(dots(&session), 4);
    judged_once(&mut session, 2, Outcome::Redo);
}
