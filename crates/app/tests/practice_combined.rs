//! A session from the second on: it opens on its warm-up (kinds met before, their instructions
//! told), then the mix, where combined lines come with their own instructions and are drawn with
//! the pens; a group is compared in place and aligned, its lines fitted together as one shape (the
//! sketch pen's marks left out), and corrected in blue.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Overlay, Pen, PracticeIntent,
    PracticeLog, PracticePens, Session, SheetView, StrokeIntent, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{ExerciseResult, Family, Original, SessionRecord, Sheet};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn pens() -> PracticePens {
    PracticePens::of(&BrushLibrary::built_in().expect("built-in brushes")).expect("pens")
}

/// The second session after a first that compared a line and a colour.
fn second() -> Session {
    after_first(true)
}

/// The session after a first that compared a line and a colour, finished or still open (the
/// test-session chore's: the summary's earlier sessions are the finished ones only).
fn after_first(finished: bool) -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let mut first = SessionRecord::stage_one(1, 1, 5, 1_791_201_600, &[]);
    first.canvas = 3;
    first.finished = finished.then_some(1_791_202_600);
    first.results = vec![
        ExerciseResult::first(0, 1_791_201_700, 2, Vec::new()),
        ExerciseResult::first(10, 1_791_201_800, 4, Vec::new()),
    ];
    let log = PracticeLog::new(vec![first.clone()], Vec::new());
    let (record, _) = log.begin(1, 0x5EED_0079, 1_791_290_000);
    let [width, height] = Sheet::of(record.exercises.len()).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let before = if finished { vec![first] } else { Vec::new() };
    let view = SheetView::of(&record, &[]);
    Session::practising(sheet, (brush, pens()), (record, before, view), &VIEWPORT)
}

/// A pen stroke through `points` (canvas pixels), 8 ms apart.
fn draw(session: &mut Session, points: &[[f32; 2]]) {
    let sample = |index: usize| InputSample {
        position: points[index],
        pressure: 0.6,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    let last = points.len() - 1;
    let steps = [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ];
    for (phase, samples) in steps {
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

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_290_100).expect("practised");
    if let Some(job) = session.practice_fit_request() {
        session.practice_fitted(job.sequence, job.run());
    }
}

fn shown(session: &Session) -> (Option<Family>, bool, (usize, usize), Option<Family>) {
    let projection = session.projection();
    let p = projection.practice().expect("a practice sheet");
    (p.family, p.warming_up, p.place, p.instructions)
}

fn overlay(session: &Session) -> Option<Vec<Vec<[f32; 2]>>> {
    match session.projection().practice()?.overlay.clone()? {
        Overlay::Lines(lines) => Some(lines),
        Overlay::Colour { .. } | Overlay::Picture { .. } => None,
    }
}

fn close(a: [f32; 2], b: [f32; 2], within: f32) -> bool {
    (a[0] - b[0]).abs() < within && (a[1] - b[1]).abs() < within
}

#[test]
fn a_second_session_warms_up_then_meets_combined_lines_with_their_instructions() {
    let mut session = second();
    let warm_up = (Some(Family::Line), true, (1, 4), None);
    assert_eq!(shown(&session), warm_up, "lines met: no instructions");
    for _ in 0..2 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    assert_eq!(shown(&session), (Some(Family::Colour), true, (3, 4), None));
    for _ in 0..2 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    assert_eq!(shown(&session), (Some(Family::Line), false, (1, 6), None));
    for _ in 0..6 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    let group = (
        Some(Family::Combined),
        false,
        (1, 4),
        Some(Family::Combined),
    );
    assert_eq!(shown(&session), group, "their instructions by themselves");
    let pen = session.projection().practice().and_then(|p| p.pen);
    assert_eq!(
        pen.map(|(pen, _)| pen),
        Some(Pen::Draw),
        "drawn with the pens"
    );
}

#[test]
fn a_warm_up_tells_no_instructions_after_a_session_still_open() {
    let session = after_first(false);
    let warm_up = (Some(Family::Line), true, (1, 4), None);
    assert_eq!(shown(&session), warm_up, "its kinds were met, so known");
}

#[test]
fn combined_lines_are_compared_in_place_and_aligned_as_one_shape() {
    let mut session = second();
    for _ in 0..10 {
        practise(&mut session, PracticeIntent::Next(VIEWPORT));
    }
    let projection = session.projection();
    let practice = projection.practice().expect("a practice sheet");
    let pair = practice.sheet.pairs[10];
    let Original::Lines(lines) = practice.sheet.originals[10].clone() else {
        panic!("combined lines")
    };
    assert!(lines.len() >= 2);
    // Key points marked with the sketch pen, never fitted.
    practise(&mut session, PracticeIntent::Pen(Pen::Sketch));
    let mark = pair.copy.to_canvas([0.95, 0.05]).map(|v| v as f32);
    draw(&mut session, &[mark, [mark[0] - 6.0, mark[1] + 6.0]]);
    // The lines in reverse order, each smaller about the frame's middle and 20 px to the right.
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    let copied = |p: &[f64; 2]| {
        let p = [0.5 + (p[0] - 0.5) * 0.85 + 0.05, 0.5 + (p[1] - 0.5) * 0.85];
        pair.copy.to_canvas(p).map(|v| v as f32)
    };
    let copies: Vec<Vec<[f32; 2]>> = (lines.iter().rev())
        .map(|line| {
            line.iter()
                .step_by(6)
                .chain(line.last())
                .map(copied)
                .collect()
        })
        .collect();
    for copy in &copies {
        draw(&mut session, copy);
    }
    practise(&mut session, PracticeIntent::Compare(true));
    let in_place = overlay(&session).expect("the overlay");
    assert_eq!(in_place.len(), lines.len(), "every line laid");
    let placed = pair.copy.to_canvas(lines[0][0]).map(|v| v as f32);
    assert!(close(in_place[0][0], placed, 0.01), "in place");
    practise(&mut session, PracticeIntent::Aligned(true));
    let aligned = overlay(&session).expect("the overlay");
    for (line, copy) in aligned.iter().zip(copies.iter().rev()) {
        let (Some(first), Some(last)) = (line.first(), line.last()) else {
            panic!("a line")
        };
        assert!(close(*first, copy[0], 1.5), "{first:?} {:?}", copy[0]);
        assert!(close(*last, copy[copy.len() - 1], 1.5));
    }
    let pen = session.projection().practice().and_then(|p| p.pen);
    assert_eq!(pen.map(|(pen, _)| pen), Some(Pen::Correct), "blue now");
    // A correction takes the comparison away; Compare again counts.
    draw(&mut session, &copies[0][..10]);
    assert!(overlay(&session).is_none());
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.practice_save_request().expect("the record changed");
    assert_eq!(record.result(10).map(|r| r.compares), Some(2));
    assert_eq!(record.warm_up, 4);
}
