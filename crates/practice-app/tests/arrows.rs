//! Organic arrows on a practice sheet (S156l): the path dashed and the arrowhead printed in the one
//! frame; the widths marked with Sketch and the edges drawn with Draw; Compare lays the exact
//! ribbon over the copy and waits for the learner's verdict, the eye judging (D123): no reading, no
//! app result, nothing kept; Construction lays the width at even steps; the result chosen once.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Arrow, ArrowCompared, ArrowTask, Figure, Frame, Given, Judge, Kind, Layout, LinePen, Outcome,
    Point, Sheet, SheetMark, Skill, SkillLevel, Verdict,
};
use openpaint_practice_app::{
    Judged, Pen, PracticeIntent, PracticeLog, PracticePens, PracticeProjection, SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A first-level lesson of organic arrows as `log` starts it, open on a sheet laid ahead.
fn opened(log: &PracticeLog) -> SheetSession {
    let skill = Skill::of_key("organic-arrow").expect("the skill");
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

#[test]
fn compare_lays_the_ribbon_and_waits_for_the_learner_s_verdict() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let mut session = opened(&log);
    let practice = session.projection();
    let kind = Kind::Arrow(Arrow::Gentle);
    assert_eq!((practice.kind, practice.how), (Some(kind), Some(kind)));
    assert!(!practice.aligned_offered);
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = original.downcast::<ArrowTask>().expect("an arrow").clone();
    let judge = |outcome| PracticeIntent::Judge(outcome);
    assert!(
        session.practise(judge(Outcome::Clean), NOON).is_err(),
        "before Compare"
    );
    drawn(&mut session, &frame, &task);
    practise(&mut session, PracticeIntent::Compare(true));
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    assert!(compared.downcast::<ArrowCompared>().is_some());
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the learner's to judge");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    answer_laid(&practice, &task);
    // Nothing read or kept: no result until the learner gives one.
    let record = session.sheet().record().clone();
    let first = record.first(0).expect("the first Compare");
    assert!(first.metrics.is_empty());
    assert_eq!(record.laid(0).and_then(|laid| laid.result), None);
    // 看画法: the width laid across at even steps, faint.
    practise(&mut session, PracticeIntent::Construction(true));
    let overlay = session.projection().overlay.expect("the answer laid");
    let [faint, dashed] = lines(&overlay, LinePen::Faint);
    assert_eq!(faint + dashed, task.answer.widths.len());
    // The learner's verdict, once.
    practise(&mut session, judge(Outcome::Near));
    let judged = session.projection().judged.expect("judged");
    assert_eq!(judged.result, Some((Outcome::Near, Judge::Eye)));
    assert!(
        session.practise(judge(Outcome::Clean), NOON + 80).is_err(),
        "once"
    );
    let saved = session.sheet_mut().save_request();
    let result = saved.and_then(|sheet| sheet.laid(0).and_then(|laid| laid.result));
    assert_eq!(
        result,
        Some((Outcome::Near, Judge::Eye)),
        "kept with the record"
    );
}

/// A few widths marked with Sketch along the path, the edges' seen stretches with Draw, the first
/// a little off.
fn drawn(session: &mut SheetSession, frame: &Frame, task: &ArrowTask) {
    practise(session, PracticeIntent::Pen(Pen::Sketch));
    let edges = &task.answer.edges;
    for i in [120, 360, 600] {
        draw(session, frame, &[edges[0][i], edges[1][i]]);
    }
    practise(session, PracticeIntent::Pen(Pen::Draw));
    let off = |p: &Point| [p[0] + 0.01, p[1] + 0.01];
    let stretches = task.answer.stretches.iter().flatten().filter(|s| !s.hidden);
    for (k, stretch) in stretches.enumerate() {
        let points: Vec<Point> = if k == 0 {
            stretch.points.iter().map(off).collect()
        } else {
            stretch.points.clone()
        };
        draw(session, frame, &points);
    }
}

/// The answer over the copy: the ribbon's seen stretches red, its hidden ones dashed; nothing
/// ringed.
fn answer_laid(practice: &PracticeProjection, task: &ArrowTask) {
    let overlay = practice.overlay.clone().expect("the answer laid");
    let answer = &task.answer;
    let stretches = (answer.stretches.iter().flatten()).chain(answer.folds.iter().flatten());
    let hidden = stretches.clone().filter(|s| s.hidden).count();
    let [red, dashed] = lines(&overlay, LinePen::Answer);
    assert_eq!((red, dashed), (stretches.count() - hidden, hidden));
    let ringed = |mark: &SheetMark| matches!(mark.drawn, Figure::Ring(_));
    assert!(!overlay.iter().any(ringed));
}
