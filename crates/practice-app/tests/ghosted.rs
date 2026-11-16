//! Ghosted lines on a practice sheet (S156): unit 1's first skill, drawn with Draw alone. A lesson
//! at each level lays its variant in one frame; the strokes drawn between the given points, Compare
//! lays the exact lines over the copy and waits for the learner's verdict, the eye judging (D123):
//! nothing read or kept, the pen still Draw; at 稳 Construction runs a plane's edges on toward where
//! they meet; the result chosen once.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Title, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Figure, Frame, Ghosted, GhostedCompared, GhostedTask, Given, Judge, Kind, Layout, LinePen,
    Outcome, Point, Sheet, SheetMark, Skill, SkillLevel, Verdict,
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

/// A lesson of ghosted lines at `level` as `log` starts it, open on a sheet laid ahead.
fn opened(log: &PracticeLog, level: SkillLevel) -> SheetSession {
    let skill = Skill::of_key("ghosted-lines").expect("the skill");
    let (record, _) = (log.start(Start::At(skill, level), 7, NOON)).expect("a lesson");
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

/// The overlay's lines in `pen`.
fn lines(overlay: &[SheetMark], pen: LinePen) -> Vec<Vec<Point>> {
    (overlay.iter())
        .filter_map(|mark| match &mark.drawn {
            Figure::Line { points, pen: p, .. } if *p == pen => Some(points.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn compare_lays_the_exact_lines_and_waits_for_the_learner_s_verdict() {
    let log = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let levels = [
        (SkillLevel::First, Ghosted::Superimposed, 3),
        (SkillLevel::Second, Ghosted::Pairs, 2),
        (SkillLevel::Third, Ghosted::Plane, 3),
    ];
    for (level, ghosted, count) in levels {
        let mut session = opened(&log, level);
        let task = drawn_a_little_off(&mut session, ghosted, count);
        practise(&mut session, PracticeIntent::Compare(true));
        laid_for_the_eye(&session, &task);
        // 看画法 at 稳 only: a plane's edges run on faint toward where they meet.
        let construction = session.practise(PracticeIntent::Construction(true), NOON + 70);
        if ghosted == Ghosted::Plane {
            construction.expect("a plane's construction");
            let overlay = session.projection().overlay.expect("the answer laid");
            assert!(!lines(&overlay, LinePen::Faint).is_empty());
            practise(&mut session, PracticeIntent::Construction(false));
        }
        // The learner's verdict, once.
        let judge = |outcome| PracticeIntent::Judge(outcome);
        practise(&mut session, judge(Outcome::Near));
        let judged = session.projection().judged.expect("judged");
        assert_eq!(judged.result, Some((Outcome::Near, Judge::Eye)));
        let again = session.practise(judge(Outcome::Clean), NOON + 80);
        assert!(again.is_err(), "once");
    }
}

/// The lesson shows `count` of `ghosted` with Draw alone and asks nothing before Compare; each
/// of its first exercise's lines drawn a little off at its far end. Its task.
fn drawn_a_little_off(session: &mut SheetSession, ghosted: Ghosted, count: usize) -> GhostedTask {
    let practice = session.projection();
    let kind = Kind::Ghosted(ghosted);
    assert_eq!((practice.kind, practice.how), (Some(kind), Some(kind)));
    assert_eq!(practice.sheet.kinds, vec![kind; count], "{ghosted:?}");
    assert!(!practice.aligned_offered);
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Draw));
    for refused in [Pen::Sketch, Pen::Correct] {
        let chosen = session.practise(PracticeIntent::Pen(refused), NOON);
        assert!(chosen.is_err(), "{refused:?}");
    }
    let judged = session.practise(PracticeIntent::Judge(Outcome::Clean), NOON);
    assert!(judged.is_err(), "before Compare");
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = (original.downcast::<GhostedTask>()).expect("ghosted lines");
    for [a, b] in &task.lines {
        draw(session, &frame, &[*a, [b[0] + 0.01, b[1] - 0.012]]);
    }
    task.clone()
}

/// Compared: the exact lines red over the copy, the eye's to judge, the pen still Draw, nothing
/// read or kept.
fn laid_for_the_eye(session: &SheetSession, task: &GhostedTask) {
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    assert!(compared.downcast::<GhostedCompared>().is_some());
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the learner's to judge");
    let pen = practice.pen.map(|(pen, _)| pen);
    assert_eq!(pen, Some(Pen::Draw), "no blue pen");
    let overlay = practice.overlay.clone().expect("the answer laid");
    let exact: Vec<Vec<Point>> = task.lines.iter().map(|line| line.to_vec()).collect();
    assert_eq!(lines(&overlay, LinePen::Answer), exact);
    let record = session.sheet().record();
    let first = record.first(0).expect("the first Compare");
    assert!(first.metrics.is_empty());
    assert_eq!(record.laid(0).and_then(|laid| laid.result), None);
}
