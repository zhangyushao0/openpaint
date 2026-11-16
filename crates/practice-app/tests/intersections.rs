//! Form intersections on a practice sheet (S156k): both forms printed drawn through in the one
//! frame; the pierce points found with Sketch and the line drawn with Draw; Compare lays the exact
//! line where the forms meet over the copy and waits for the learner's verdict, the eye judging
//! (D123): no reading, no app result, nothing kept; Construction lays the way to find it; the
//! result chosen once.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Figure, Frame, Given, IntersectionCompared, IntersectionTask, Judge, Kind, Layout, LinePen,
    Meeting, Outcome, Point, Sheet, SheetMark, Skill, SkillLevel, Verdict,
};
use openpaint_practice_app::{
    Judged, Pen, PracticeIntent, PracticeLog, PracticePens, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A first-level lesson of form intersections as `log` starts it, open on a sheet laid ahead.
fn opened(log: &PracticeLog) -> SheetSession {
    let skill = Skill::of_key("intersections").expect("the skill");
    let (record, _) = (log.start(Start::At(skill, SkillLevel::First), 7, NOON)).expect("a lesson");
    let record = Sheet {
        canvas: 40,
        ..record
    };
    let size = Layout::laid_ahead(&record);
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let printed = Given::of_sheet(&record, log.packs());
    let opening = log.practising(&record, &printed, (size, &Title::untitled()));
    SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT)
}

/// A pen stroke along `points` (frame units of `frame`).
fn draw(session: &mut SheetSession, frame: &Frame, points: &[Point]) {
    let path: Vec<[f32; 2]> = (openpaint_practice::resample(points, 0.002).into_iter())
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
        let intent = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(intent).expect("dispatch");
    }
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOON + 60).expect("practised");
}

/// The overlay's lines in `pen`: whole and dashed.
fn lines(overlay: &[SheetMark], pen: LinePen) -> [usize; 2] {
    let of = |dashed: bool| {
        (overlay.iter())
            .filter(|mark| {
                matches!(&mark.drawn, Figure::Line { pen: p, dashed: d, .. } if *p == pen && *d == dashed)
            })
            .count()
    };
    [of(false), of(true)]
}

/// The pierce points with Sketch, the seen line with Draw, a little off.
fn drawn(session: &mut SheetSession, frame: &Frame, task: &IntersectionTask) {
    practise(session, PracticeIntent::Pen(Pen::Sketch));
    for p in &task.pierced {
        draw(session, frame, &[[p[0] - 0.01, p[1]], [p[0] + 0.01, p[1]]]);
    }
    practise(session, PracticeIntent::Pen(Pen::Draw));
    for stretch in task.answer.iter().filter(|s| !s.hidden) {
        let off: Vec<Point> = stretch.points.iter().map(|p| [p[0] + 0.01, p[1]]).collect();
        draw(session, frame, &off);
    }
}

#[test]
fn compare_lays_the_meeting_line_and_waits_for_the_learner_s_verdict() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let mut session = opened(&log);
    let practice = session.projection();
    let kind = practice.kind.expect("in hand");
    assert!(matches!(
        kind,
        Kind::Intersection(Meeting::Edge | Meeting::Corner)
    ));
    assert_eq!(practice.how, Some(kind));
    assert!(practice.sheet.painted[0].is_none(), "no picture: one frame");
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = (original.downcast::<IntersectionTask>())
        .expect("an intersection")
        .clone();
    drawn(&mut session, &frame, &task);
    practise(&mut session, PracticeIntent::Compare(true));
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    assert!(compared.downcast::<IntersectionCompared>().is_some());
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the learner's to judge");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let overlay = practice.overlay.clone().expect("the answer laid");
    let count = |hidden: bool| task.answer.iter().filter(|s| s.hidden == hidden).count();
    assert_eq!(
        lines(&overlay, LinePen::Answer),
        [count(false), count(true)]
    );
    let record = session.sheet().record().clone();
    let first = record.first(0).expect("the first Compare");
    assert!(first.metrics.is_empty());
    assert_eq!(record.laid(0).and_then(|laid| laid.result), None);
    // 看画法: the edges carried into the faces, the pierce points ringed.
    practise(&mut session, PracticeIntent::Construction(true));
    let overlay = session.projection().overlay.expect("the answer laid");
    let [_, dashed] = lines(&overlay, LinePen::Pencil);
    assert_eq!(dashed, task.found.lines.len());
    let ringed = overlay
        .iter()
        .filter(|m| matches!(m.drawn, Figure::Found(_)));
    assert_eq!(ringed.count(), task.pierced.len());
    practise(&mut session, PracticeIntent::Judge(Outcome::Clean));
    let judged = session.projection().judged.expect("judged");
    assert_eq!(judged.result, Some((Outcome::Clean, Judge::Eye)));
    let saved = session.sheet_mut().save_request();
    let result = saved.and_then(|sheet| sheet.laid(0).and_then(|laid| laid.result));
    assert_eq!(result, Some((Outcome::Clean, Judge::Eye)));
}
