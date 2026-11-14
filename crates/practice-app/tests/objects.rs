//! Objects on a practice sheet (S94): an object's picture beside the box given by its front
//! corner, three edges and horizon; the box completed and its points found with Sketch, never
//! read; its visible edges drawn with Draw and compared against the exact answer (each edge read,
//! the error kept at the first Compare); an edge corrected in place of what it redraws; a lesson
//! of objects as the path starts it.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Carve, Exercise, Frame, Kind, Metric, OBJECT_SIDE, ObjectTask, Point, Skill, SkillLevel,
    SolidCheck, resample, value,
};
use openpaint_practice_app::{Pen, PracticeIntent, PracticeLog, PracticePens, SheetSession, Start};

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

/// The object's check over the copy.
fn checked(session: &SheetSession) -> SolidCheck {
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    let object = compared.downcast::<openpaint_practice::ObjectCompared>();
    object.expect("the object's check").check.clone()
}

#[test]
fn an_object_is_constructed_drawn_compared_and_an_edge_corrected() {
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
    let pen = session.projection().pen;
    assert_eq!(pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let first = value(&check.metrics, Metric::ObjectOff).expect("measured");
    assert!(first > 0.0 && first < 0.02, "{first}");
    // The first edge corrected: read alone in place of the first try, the first error kept.
    draw(&mut session, &frame, &[a, b]);
    practise(&mut session, PracticeIntent::Compare(true));
    let now = value(&checked(&session).metrics, Metric::ObjectOff).expect("measured");
    assert!(now < first, "{now} against {first}");
    let record = session
        .sheet_mut()
        .save_request()
        .expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    assert_eq!(value(&result.metrics, Metric::ObjectOff), Some(first));
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
