//! Contours on a practice sheet (S156k): the lit sausage beside its outline, spine, ticks and the
//! near end's contour printed; the ellipses drawn with Draw; Compare lays each asked section's exact
//! ellipse over the copy and waits for the learner's verdict, the eye judging (D123): no reading, no
//! app result, nothing kept; Construction lays the spine and the sections' squares; the result
//! chosen once.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Bend, ContourCompared, ContourTask, CopyRef, Figure, Frame, Given, Judge, Kind, Layout,
    LinePen, OBJECT_SIDE, Outcome, Point, Sheet, SheetMark, Skill, SkillLevel, Verdict,
};
use openpaint_practice_app::{
    Judged, Pen, PracticeIntent, PracticeLog, PracticePens, SheetSession, Start, stored_card,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// A first-level lesson of contours as `log` starts it, open on a sheet laid ahead.
fn opened(log: &PracticeLog) -> SheetSession {
    let skill = Skill::of_key("contours").expect("the skill");
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

/// Each ellipse through, with Draw, the first a little narrow.
fn drawn(session: &mut SheetSession, frame: &Frame, task: &ContourTask) {
    practise(session, PracticeIntent::Pen(Pen::Draw));
    for (k, contour) in task.contours.iter().enumerate() {
        let ellipse = contour.guide.exact;
        let narrow = if k == 0 { 0.8 } else { 1.0 };
        let (sin, cos) = ellipse.angle.sin_cos();
        let [a, b] = [ellipse.axes[0], ellipse.axes[1] * narrow];
        let points: Vec<Point> = (0..=48)
            .map(|i| {
                let t = f64::from(i) / 48.0 * std::f64::consts::TAU;
                let (x, y) = (a * t.cos(), b * t.sin());
                [
                    ellipse.centre[0] + x * cos - y * sin,
                    ellipse.centre[1] + x * sin + y * cos,
                ]
            })
            .collect();
        draw(session, frame, &points);
    }
}

#[test]
fn compare_lays_the_sections_and_waits_for_the_learner_s_verdict() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let mut session = opened(&log);
    let practice = session.projection();
    let kind = Kind::Contour(Bend::Gentle);
    assert_eq!((practice.kind, practice.how), (Some(kind), Some(kind)));
    assert!(!practice.aligned_offered);
    let picture = (practice.sheet.painted[0].as_ref()).expect("the lit sausage");
    assert_eq!(picture.size, [OBJECT_SIDE; 2]);
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = (original.downcast::<ContourTask>())
        .expect("contours")
        .clone();
    let judge = PracticeIntent::Judge;
    assert!(
        session.practise(judge(Outcome::Clean), NOON).is_err(),
        "before Compare"
    );
    drawn(&mut session, &frame, &task);
    practise(&mut session, PracticeIntent::Compare(true));
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    assert!(compared.downcast::<ContourCompared>().is_some());
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the learner's to judge");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let overlay = practice.overlay.clone().expect("the answer laid");
    let stretches = task.contours.iter().flat_map(|c| &c.stretches);
    let count = |hidden: bool| stretches.clone().filter(|s| s.hidden == hidden).count();
    assert_eq!(
        lines(&overlay, LinePen::Answer),
        [count(false), count(true)]
    );
    let ringed = |mark: &SheetMark| matches!(mark.drawn, Figure::Ring(_));
    assert!(!overlay.iter().any(ringed));
    // Nothing read or kept: no result until the learner gives one.
    let record = session.sheet().record().clone();
    let first = record.first(0).expect("the first Compare");
    assert!(first.metrics.is_empty());
    assert_eq!(record.laid(0).and_then(|laid| laid.result), None);
    // 看画法: the spine and each section's square.
    practise(&mut session, PracticeIntent::Construction(true));
    let overlay = session.projection().overlay.expect("the answer laid");
    assert_eq!(lines(&overlay, LinePen::Pencil), [1, task.contours.len()]);
    // The learner's verdict, once, kept with the record.
    practise(&mut session, judge(Outcome::Near));
    let judged = session.projection().judged.expect("judged");
    assert_eq!(judged.result, Some((Outcome::Near, Judge::Eye)));
    assert!(
        session.practise(judge(Outcome::Clean), NOON + 80).is_err(),
        "once"
    );
    let saved = session.sheet_mut().save_request();
    let result = saved.and_then(|sheet| sheet.laid(0).and_then(|laid| laid.result));
    assert_eq!(result, Some((Outcome::Near, Judge::Eye)));
}

/// A blind pair's card lays the sections' exact ellipses over the copy as Compare does, its
/// strokes never read: the card asks the kind's Compare (S157).
#[test]
fn a_pair_s_card_lays_the_exact_sections() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let skill = Skill::of_key("contours").expect("the skill");
    let (record, _) = (log.start(Start::At(skill, SkillLevel::First), 7, NOON)).expect("a lesson");
    let record = Sheet {
        canvas: 40,
        ..record
    };
    let size = Layout::laid_ahead(&record);
    let artwork = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let printed = Given::of_sheet(&record, log.packs());
    let copy = CopyRef {
        sheet: record.id,
        exercise: 0,
    };
    let card = stored_card((&artwork, &record, &printed), (log.packs(), None), copy, 0)
        .expect("the copy's card");
    assert!(!card.laid.is_empty(), "the exact sections");
    assert!(card.laid.iter().all(|(line, _)| line.len() > 8), "ellipses");
}
