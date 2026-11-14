//! Week 2's perspective on a practice sheet (S84): a box completed from its front corner, drawn
//! with the pens and compared as geometry (an edge that misses the exact box's vanishing point
//! drawn on past it, never aligned; its construction on request, S97), a page of free boxes read
//! box by box, and an ellipse fitted to the lines drawn, then to the corrections alone; no numbers
//! kept.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    BoxCompared, BoxView, Compared, Ellipse, EllipseCompared, EllipseGuide, Exercise, Frame,
    FreeCompared, FreePage, Guide, Horizon, Kind, PerspectiveBox, Point, Roundness,
};
use openpaint_practice_app::{Pen, PracticeIntent, PracticePens, SheetSession};

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
fn a_box_is_completed_and_only_its_edge_that_misses_is_drawn_on() {
    let mut session = opened();
    let practice = session.projection();
    let two_point = Kind::Box(BoxView::TwoPoint);
    assert_eq!(practice.kind, Some(two_point));
    assert_eq!(practice.how, Some(two_point), "told the first time");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    assert!(!practice.aligned_offered, "geometry is never aligned");
    let frame = completed(&mut session);
    let Some(BoxCompared { check, .. }) = compared(&session) else {
        panic!("the box's check")
    };
    assert_eq!(check.edges.len(), 9);
    assert_eq!(
        check.edges.iter().filter(|e| !e.on).count(),
        1,
        "the one off"
    );
    // Only the edge that misses its point is drawn on (S97).
    let extended: Vec<bool> = check.edges.iter().map(|e| e.off.is_some()).collect();
    let missed: Vec<bool> = check.edges.iter().map(|e| !e.on).collect();
    assert_eq!(extended, missed);
    let practice = session.projection();
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    assert!(session.practise(PracticeIntent::Aligned(true), 1).is_err());
    // A correction takes the check away; Compare again counts, no numbers kept.
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
fn free_boxes_are_read_box_by_box_against_the_page_s_horizon() {
    let mut session = opened();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::FreeBoxes(Horizon::Middle)));
    let frame = practice.sheet.pairs[1].copy;
    let Some(FreePage(example)) = practice
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
    let Some(FreeCompared { check, .. }) = compared(&session) else {
        panic!("the page's check")
    };
    assert_eq!(check.boxes.len(), 2);
    for free in &check.boxes {
        assert_eq!(free.meets.len(), 2, "two directions");
        assert!(free.apart.is_empty() && free.narrow.is_empty());
    }
}

#[test]
fn an_ellipse_is_fitted_then_its_correction_alone() {
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
    let round = |e: &Ellipse| e.outline(48);
    draw(&mut session, &frame, &round(&guide.exact));
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(EllipseCompared { check, .. }) = compared(&session) else {
        panic!("the ellipse's check")
    };
    assert!(check.fitted.is_some() && check.roundness.is_none() && check.sides.is_empty());
    // Corrected flatter in blue: the fit is the correction's.
    let flatter = Ellipse {
        axes: [guide.exact.axes[0], guide.exact.axes[1] * 0.7],
        ..guide.exact
    };
    draw(&mut session, &frame, &round(&flatter));
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(EllipseCompared { check, .. }) = compared(&session) else {
        panic!("the ellipse's check")
    };
    assert_eq!(check.roundness, Some(Roundness::Flatter));
}
