//! Objects on a practice sheet (S94, S145): an object's picture beside the box given by its front
//! corner, three edges and horizon; the box completed and its points found with Sketch; its
//! visible edges drawn with Draw; Compare lays the exact visible edges over the copy and the
//! learner's eye judges it: nothing read from the strokes, nothing kept but the choice (D123); a
//! lesson of objects as the path starts it.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Carve, Exercise, Figure, Frame, Judge, Kind, LinePen, OBJECT_SIDE, ObjectCompared, ObjectTask,
    Outcome, Point, SheetMark, Skill, SkillLevel, Verdict, resample,
};
use openpaint_practice_app::{
    Judged, Pen, PracticeIntent, PracticeLog, PracticePens, PracticeProjection, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> SheetSession {
    let block = Exercise::generated(Kind::Object(Carve::Block), 0x5EED_0094);
    let record = support::laid(vec![block], 0);
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

/// A pen stroke along `points` (frame units of `frame`), sampled every half pixel or so.
fn draw(session: &mut SheetSession, frame: &Frame, points: &[Point]) {
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

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, 1_791_900_000).expect("practised");
}

#[test]
fn an_object_is_constructed_drawn_and_compare_lays_its_edges_for_the_eye() {
    let mut session = opened();
    let practice = session.projection();
    let block = Kind::Object(Carve::Block);
    assert_eq!((practice.kind, practice.how), (Some(block), Some(block)));
    assert!(!practice.aligned_offered);
    let picture = practice.sheet.painted[0]
        .as_ref()
        .expect("the object's picture");
    assert_eq!(picture.size, [OBJECT_SIDE; 2]);
    let frame = practice.sheet.pairs[0].copy;
    let Some(task) = practice
        .original
        .as_deref()
        .expect("in hand")
        .downcast::<ObjectTask>()
    else {
        panic!("an object")
    };
    let task = task.clone();
    // The box completed and the construction drawn with Sketch.
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
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    let laid = compared
        .downcast::<ObjectCompared>()
        .expect("the object laid");
    assert_eq!(laid.task, task);
    laid_for_the_eye(&practice, &task);
    // The first edge corrected and compared again; the learner's choice kept, nothing read.
    draw(&mut session, &frame, &[a, b]);
    practise(&mut session, PracticeIntent::Compare(true));
    practise(&mut session, PracticeIntent::Judge(Outcome::Near));
    let record = session
        .sheet_mut()
        .save_request()
        .expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    assert!(result.metrics.is_empty(), "nothing read");
    let kept = record.laid(0).and_then(|laid| laid.result);
    assert_eq!(kept, Some((Outcome::Near, Judge::Eye)));
}

/// Every visible edge and seen curve of `task` red over the copy, nothing ringed or run on; the
/// pen blue, the eye's three asked.
fn laid_for_the_eye(practice: &PracticeProjection, task: &ObjectTask) {
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the eye's to judge");
    let overlay = practice.overlay.clone().expect("compared");
    let red = (overlay.iter())
        .filter(|mark| matches!(&mark.drawn, Figure::Line { pen, .. } if *pen == LinePen::Answer));
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    assert_eq!(red.count(), task.answer.sides.len() + seen.count());
    let ringed = |mark: &SheetMark| matches!(mark.drawn, Figure::Ring(_));
    assert!(!overlay.iter().any(ringed));
}

#[test]
fn a_lesson_of_objects_starts_at_a_level_without_a_pack_its_unit_s_path_tile_lacking_nothing() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let objects = Skill::of_key("objects").expect("the objects' skill");
    let start = Start::At(objects, SkillLevel::First);
    let (record, title) = (log.start(start, 0x5EED_0094, 1_791_900_000)).expect("a lesson");
    assert_eq!(title.as_str(), "Lesson 1");
    assert!(record.exercises().count() > 0);
    for laid in record.exercises() {
        assert_eq!(Skill::of(laid.exercise.kind), Some(objects));
    }
    let path = log.path();
    let tile = (path.units.iter().flat_map(|unit| &unit.skills)).find(|s| s.skill == objects);
    assert_eq!(tile.map(|tile| (tile.dots, tile.lacks)), Some((0, None)));
}
