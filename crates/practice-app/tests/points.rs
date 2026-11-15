//! Unit 3 on a practice sheet (S90): an estimate marked with the pens and compared against its
//! exact answer (the last mark read, its error kept at the first Compare, a correction read in its
//! place), an angle's line, a point in space marked on its box's face; never aligned; and a
//! lesson of estimates as the path starts it.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Compared, Estimate, Exercise, Flat, FlatAnswer, FlatCompared, Frame, Kind, Metric, Off,
    Original, Point, Skill, SkillLevel, SpaceAnswer, SpaceCompared, SpaceTask, Trick, value,
};
use openpaint_practice_app::{
    Pen, PracticeIntent, PracticeLog, PracticePens, PracticeProjection, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> SheetSession {
    let exercises = vec![
        Exercise::generated(Kind::Flat(Estimate::Point), 0x5EED_0090),
        Exercise::generated(Kind::Flat(Estimate::Angle), 0x5EED_0091),
        Exercise::generated(Kind::Space(Trick::Centre), 0x5EED_0092),
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
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

/// A small cross at `p`, two strokes.
fn cross(session: &mut SheetSession, frame: &Frame, p: Point) {
    draw(session, frame, &[[p[0] - 0.02, p[1]], [p[0] + 0.02, p[1]]]);
    draw(session, frame, &[[p[0], p[1] - 0.02], [p[0], p[1] + 0.02]]);
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, 1_791_900_000).expect("practised");
}

fn compared<T: Compared + Clone>(session: &SheetSession) -> Option<T> {
    let practice = session.projection();
    practice.compared.as_ref()?.downcast::<T>().cloned()
}

/// The original in hand, as its kind's own type `T`.
fn in_hand<T: Original>(practice: &PracticeProjection) -> &T {
    let original = practice.original.as_deref().expect("in hand");
    original.downcast::<T>().expect("its kind's original")
}

#[test]
fn a_point_is_marked_compared_and_corrected_its_first_error_kept() {
    let mut session = opened();
    let practice = session.projection();
    let point = Kind::Flat(Estimate::Point);
    assert_eq!(practice.kind, Some(point));
    assert_eq!(practice.how, Some(point), "told the first time");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    assert!(!practice.aligned_offered);
    let frame = practice.sheet.pairs[0].copy;
    let flat = *in_hand::<Flat>(&practice);
    let FlatAnswer::Point(at) = flat.answer() else {
        panic!("a point")
    };
    // A construction line with Sketch is never read; the cross off by 0.05 across is.
    practise(&mut session, PracticeIntent::Pen(Pen::Sketch));
    draw(&mut session, &frame, &[[0.0, at[1]], [1.0, at[1]]]);
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    cross(&mut session, &frame, [at[0] + 0.05, at[1]]);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(FlatCompared { check, .. }) = compared(&session) else {
        panic!("the estimate's check")
    };
    let mark = check.mark.expect("the cross read");
    assert!((mark[0] - at[0] - 0.05).abs() < 1e-3 && (mark[1] - at[1]).abs() < 1e-3);
    let practice = session.projection();
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    assert!(session.practise(PracticeIntent::Aligned(true), 1).is_err());
    // Corrected onto the answer: read in place of the first, the first error kept.
    cross(&mut session, &frame, at);
    assert!(
        compared::<FlatCompared>(&session).is_none(),
        "the pen landing takes it away"
    );
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(FlatCompared { check, .. }) = compared(&session) else {
        panic!("the estimate's check")
    };
    let mark = check.mark.expect("the correction read");
    assert!((mark[0] - at[0]).abs() < 1e-3);
    let record = session
        .sheet_mut()
        .save_request()
        .expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    let across = value(&result.metrics, Metric::PointAcross).expect("kept");
    assert!((across - 0.05).abs() < 1e-3, "{across}");
}

#[test]
fn an_angle_s_last_line_is_read_and_a_point_in_space_marked_on_its_face() {
    let mut session = opened();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::Flat(Estimate::Angle)));
    let frame = practice.sheet.pairs[1].copy;
    let flat @ Flat::Angle { .. } = *in_hand::<Flat>(&practice) else {
        panic!("an angle")
    };
    let FlatAnswer::Line([a, b]) = flat.answer() else {
        panic!("a line")
    };
    draw(&mut session, &frame, &[a, b]);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(FlatCompared { check, .. }) = compared(&session) else {
        panic!("the angle's check")
    };
    let off = value(&check.metrics, Metric::AngleOff).expect("measured");
    assert!(off.abs() < 0.5, "drawn along the answer: {off}");
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::Space(Trick::Centre)));
    let frame = practice.sheet.pairs[2].copy;
    let task = in_hand::<SpaceTask>(&practice);
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
    let Some(SpaceCompared { check, .. }) = compared(&session) else {
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
    let Some(SpaceCompared { check, .. }) = compared(&session) else {
        panic!("the point's check")
    };
    assert!(value(&check.metrics, Metric::SpaceOff).is_some_and(|v| v < 0.02));
}

/// S98: Compare marks only what is off (a mark 20 px off its answer ringed, one on it nothing),
/// and lays the way to see it on request, off again at each Compare.
#[test]
fn an_estimate_rings_only_a_mark_off_and_shows_its_construction_on_request() {
    let mut session = opened();
    let construction = |session: &SheetSession| Some(session.projection().construction);
    let practice = session.projection();
    let frame = practice.sheet.pairs[0].copy;
    let flat = *in_hand::<Flat>(&practice);
    let FlatAnswer::Point(at) = flat.answer() else {
        panic!("a point")
    };
    cross(&mut session, &frame, [at[0] + 0.05, at[1]]);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(FlatCompared { check, .. }) = compared(&session) else {
        panic!("the estimate's check")
    };
    assert_eq!(check.off, vec![Off::Ring(at)]);
    assert_eq!(construction(&session), Some(false));
    practise(&mut session, PracticeIntent::Construction(true));
    assert_eq!(construction(&session), Some(true));
    cross(&mut session, &frame, at);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(FlatCompared { check, .. }) = compared(&session) else {
        panic!("the estimate's check")
    };
    assert!(check.off.is_empty(), "{:?}", check.off);
    assert_eq!(
        construction(&session),
        Some(false),
        "each Compare starts without it"
    );
}

#[test]
fn a_lesson_of_estimates_and_one_of_points_in_space_start_at_a_level_without_a_pack() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    for key in ["estimates", "points-in-space"] {
        let skill = Skill::of_key(key).expect("a skill");
        let start = Start::At(skill, SkillLevel::Second);
        let (record, title) = (log.start(start, 0x5EED_0090, 1_791_900_000)).expect(key);
        assert_eq!(title.as_str(), "Lesson 1");
        let laid: Vec<_> = record.exercises().collect();
        assert!(!laid.is_empty() && laid.len() <= 6, "{key}");
        for laid in laid {
            assert_eq!(Skill::of(laid.exercise.kind), Some(skill), "{key}");
            assert_eq!(laid.level, SkillLevel::Second.code(), "{key}");
        }
    }
}
