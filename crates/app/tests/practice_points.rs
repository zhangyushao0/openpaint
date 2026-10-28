//! Stage 3 on a practice sheet (S90): an estimate marked with the pens and compared against its
//! exact answer (the last mark read, its error kept at the first Compare, a correction read in its
//! place), an angle's line, a point in space marked on its box's face; never aligned; and the log
//! beginning stage 3's sessions.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Overlay, Pen, PracticeIntent,
    PracticeLog, PracticePens, Session, SheetView, StrokeIntent, Title, Toolbox, Viewport,
    sheet_of,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Estimate, Exercise, Family, Flat, FlatAnswer, Frame, Kind, Metric, Original, Plan, Point,
    SessionRecord, SpaceAnswer, Trick, value,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> Session {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Flat(Estimate::Point), 0x5EED_0090),
            Exercise::generated(Kind::Flat(Estimate::Angle), 0x5EED_0091),
            Exercise::generated(Kind::Space(Trick::Centre), 0x5EED_0092),
        ],
        warm_up: 0,
    };
    let record = SessionRecord::begun(1, (3, 1), (5, 0), plan);
    let [width, height] = sheet_of(&record, &[]).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, &[], ([width, height], &Title::untitled()));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::of(&library).expect("pens");
    let tally = openpaint_practice::Tally::default();
    Session::practising(
        sheet,
        (brush, pens),
        (record, Vec::new(), view, tally),
        &VIEWPORT,
    )
}

/// A pen stroke through `points` (frame units of `frame`), sampled finely.
fn draw(session: &mut Session, frame: &Frame, points: &[Point]) {
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
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

/// A small cross at `p`, two strokes.
fn cross(session: &mut Session, frame: &Frame, p: Point) {
    draw(session, frame, &[[p[0] - 0.02, p[1]], [p[0] + 0.02, p[1]]]);
    draw(session, frame, &[[p[0], p[1] - 0.02], [p[0], p[1] + 0.02]]);
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_900_000).expect("practised");
}

fn overlay(session: &Session) -> Option<Overlay> {
    session.projection().practice()?.overlay.clone()
}

#[test]
fn a_point_is_marked_compared_and_corrected_its_first_error_kept() {
    let mut session = opened();
    let practice = session.projection().practice().cloned().expect("a sheet");
    assert_eq!(practice.family, Some(Family::Point));
    assert_eq!(
        practice.instructions,
        Some(Family::Point),
        "told the first time"
    );
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    assert!(!practice.aligned_offered);
    let frame = practice.sheet.pairs[0].copy;
    let Original::Flat(flat) = practice.sheet.originals[0] else {
        panic!("an estimate")
    };
    let FlatAnswer::Point(at) = flat.answer() else {
        panic!("a point")
    };
    // A construction line with Sketch is never read; the cross off by 0.05 across is.
    practise(&mut session, PracticeIntent::Pen(Pen::Sketch));
    draw(&mut session, &frame, &[[0.0, at[1]], [1.0, at[1]]]);
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    cross(&mut session, &frame, [at[0] + 0.05, at[1]]);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(Overlay::Flat(check)) = overlay(&session) else {
        panic!("the estimate's check")
    };
    let mark = check.mark.expect("the cross read");
    assert!((mark[0] - at[0] - 0.05).abs() < 1e-3 && (mark[1] - at[1]).abs() < 1e-3);
    let practice = session.projection().practice().cloned().expect("a sheet");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    assert!(session.practise(PracticeIntent::Aligned(true), 1).is_err());
    // Corrected onto the answer: read in place of the first, the first error kept.
    cross(&mut session, &frame, at);
    assert!(overlay(&session).is_none(), "the pen landing takes it away");
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(Overlay::Flat(check)) = overlay(&session) else {
        panic!("the estimate's check")
    };
    let mark = check.mark.expect("the correction read");
    assert!((mark[0] - at[0]).abs() < 1e-3);
    let record = session.practice_save_request().expect("the record changed");
    let result = record.result(0).expect("compared");
    assert_eq!(result.compares, 2);
    let across = value(&result.metrics, Metric::PointAcross).expect("kept");
    assert!((across - 0.05).abs() < 1e-3, "{across}");
}

#[test]
fn an_angle_s_last_line_is_read_and_a_point_in_space_marked_on_its_face() {
    let mut session = opened();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection().practice().cloned().expect("a sheet");
    assert_eq!(practice.family, Some(Family::Angle));
    let frame = practice.sheet.pairs[1].copy;
    let Original::Flat(flat @ Flat::Angle { .. }) = practice.sheet.originals[1] else {
        panic!("an angle")
    };
    let FlatAnswer::Line([a, b]) = flat.answer() else {
        panic!("a line")
    };
    draw(&mut session, &frame, &[a, b]);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(Overlay::Flat(check)) = overlay(&session) else {
        panic!("the angle's check")
    };
    let off = value(&check.metrics, Metric::AngleOff).expect("measured");
    assert!(off.abs() < 0.5, "drawn along the answer: {off}");
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection().practice().cloned().expect("a sheet");
    assert_eq!(practice.family, Some(Family::Centre));
    let frame = practice.sheet.pairs[2].copy;
    let Original::Space(task) = &practice.sheet.originals[2] else {
        panic!("a point in space")
    };
    let SpaceAnswer::Point(centre) = task.answer() else {
        panic!("a point")
    };
    // The middle of the drawing, not the centre in space.
    let [p, q, r, s] = task.corners;
    let middle = [0, 1].map(|k| (p[k] + q[k] + r[k] + s[k]) / 4.0);
    draw(
        &mut session,
        &frame,
        &[middle, [middle[0] + 0.004, middle[1]]],
    );
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(Overlay::Space(check)) = overlay(&session) else {
        panic!("the point's check")
    };
    let off = value(&check.metrics, Metric::SpaceOff).expect("measured");
    assert!(off > 0.0 && check.mark.is_some(), "{off}");
    draw(
        &mut session,
        &frame,
        &[centre, [centre[0] + 0.004, centre[1]]],
    );
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(Overlay::Space(check)) = overlay(&session) else {
        panic!("the point's check")
    };
    assert!(value(&check.metrics, Metric::SpaceOff).is_some_and(|v| v < 0.02));
}

#[test]
fn the_log_begins_stage_three_and_offers_it_after_stage_two() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let (record, title) = log.begin((3, None), 0x5EED_0090, 1_791_900_000);
    assert_eq!((record.stage, record.number), (3, 1));
    assert_eq!(title.as_str(), "Practice 1");
    let families: Vec<Family> = record.exercises.iter().map(|e| e.kind.family()).collect();
    assert_eq!(
        families[..3],
        [Family::Point, Family::Proportion, Family::Angle]
    );
    assert_eq!(families.last(), Some(&Family::Areas));
    assert_eq!(log.today().ready, 3);
}
