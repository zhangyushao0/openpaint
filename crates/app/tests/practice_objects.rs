//! Objects on a practice sheet (S94): an object's picture beside the box given by its front
//! corner, three edges and horizon; the box completed and its points found with Sketch, never
//! read; its visible edges drawn with Draw and compared against the exact answer (each edge read,
//! the error kept at the first Compare); an edge corrected in place of what it redraws; stage 4's
//! sessions from the log.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, OBJECT_SIDE, Overlay, Pen,
    PracticeIntent, PracticeLog, PracticePens, Session, SheetView, StrokeIntent, Title, Toolbox,
    Viewport, sheet_of,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Carve, Exercise, Family, Frame, Kind, Metric, Original, Plan, Point, SessionRecord, SolidCheck,
    resample, value,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> Session {
    let plan = Plan {
        exercises: vec![Exercise::generated(Kind::Object(Carve::Block), 0x5EED_0094)],
        warm_up: 0,
    };
    let record = SessionRecord::begun(1, (4, 1), (5, 0), plan);
    let [width, height] = sheet_of(&record, &[]).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, &[], ([width, height], &Title::untitled()));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let tally = openpaint_practice::Tally::default();
    Session::practising(
        sheet,
        (brush, pens),
        (record, Vec::new(), view, tally),
        &VIEWPORT,
    )
}

/// A pen stroke along `points` (frame units of `frame`), sampled every half pixel or so.
fn draw(session: &mut Session, frame: &Frame, points: &[Point]) {
    let path: Vec<[f32; 2]> = (resample(points, 0.002).into_iter())
        .map(|p| frame.to_canvas(p).map(|v| v as f32))
        .collect();
    let sample = |index: usize| InputSample {
        position: path[index],
        pressure: 0.7,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 4 * index as i64,
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
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_900_000).expect("practised");
}

/// The object's check over the copy.
fn checked(session: &Session) -> Arc<SolidCheck> {
    match (session.projection().practice()).and_then(|p| p.overlay.clone()) {
        Some(Overlay::Object(check)) => check,
        other => panic!("the object's check: {other:?}"),
    }
}

#[test]
fn an_object_is_constructed_drawn_compared_and_an_edge_corrected() {
    let mut session = opened();
    let practice = session.projection().practice().cloned().expect("a sheet");
    assert_eq!(practice.family, Some(Family::Object));
    assert_eq!(practice.instructions, Some(Family::Object));
    assert!(!practice.aligned_offered);
    let picture = practice.sheet.painted[0]
        .as_ref()
        .expect("the object's picture");
    assert_eq!(picture.size, [OBJECT_SIDE; 2]);
    let frame = practice.sheet.pairs[0].copy;
    let Original::Object(task) = &practice.sheet.originals[0] else {
        panic!("an object")
    };
    // The box completed and the construction drawn with Sketch: never read.
    practise(&mut session, PracticeIntent::Pen(Pen::Sketch));
    for (a, b, _) in task.seen.edges() {
        draw(&mut session, &frame, &[a, b]);
    }
    for line in task.construction().iter().take(6) {
        draw(&mut session, &frame, line);
    }
    // The visible edges with Draw, the first a little off.
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    let [a, b] = task.answer.sides[0];
    let off = |p: Point| [p[0] + 0.02, p[1] + 0.015];
    draw(&mut session, &frame, &[off(a), off(b)]);
    for side in &task.answer.sides[1..] {
        draw(&mut session, &frame, side);
    }
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    for stretch in seen {
        draw(&mut session, &frame, &stretch.points);
    }
    practise(&mut session, PracticeIntent::Compare(true));
    let check = checked(&session);
    assert!(
        check.sides.iter().all(|side| !side.is_empty()),
        "every edge read"
    );
    let pen = session.projection().practice().and_then(|p| p.pen);
    assert_eq!(pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let first = value(&check.metrics, Metric::ObjectOff).expect("measured");
    assert!(first > 0.0 && first < 0.02, "{first}");
    // The first edge corrected: read alone in place of the first try, the first error kept.
    draw(&mut session, &frame, &[a, b]);
    practise(&mut session, PracticeIntent::Compare(true));
    let now = value(&checked(&session).metrics, Metric::ObjectOff).expect("measured");
    assert!(now < first, "{now} against {first}");
    let record = session.practice_save_request().expect("the record changed");
    let result = record.result(0).expect("compared");
    assert_eq!(result.compares, 2);
    assert_eq!(value(&result.metrics, Metric::ObjectOff), Some(first));
}

#[test]
fn the_log_begins_stage_four_and_offers_it_after_stage_three() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let (record, title) = log.begin((4, None), 0x5EED_0094, 1_791_900_000);
    assert_eq!((record.stage, record.number), (4, 1));
    assert_eq!(title.as_str(), "Practice 1");
    let families: Vec<Family> = record.exercises.iter().map(|e| e.kind.family()).collect();
    assert_eq!(families[..3], [Family::Object; 3]);
    assert_eq!(families[3], Family::Cylinder);
    // No pack: no shape, no crop for a combination (S101); the solid last.
    assert_eq!(families.last(), Some(&Family::Cylinder));
    assert_eq!(log.today().ready, 4);
    let tiles = log.today().drills;
    assert!(
        tiles
            .iter()
            .any(|t| t.stage == 4 && t.family == Family::Object)
    );
}
