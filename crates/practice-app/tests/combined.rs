//! A session from the second on: it opens on its warm-up (kinds met before, their instructions
//! told), then the mix, where combined lines come with their own instructions and are drawn with
//! the pens; a group is compared in place and aligned, its lines fitted together as one shape (the
//! sketch pen's marks left out), and corrected in blue.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StrokeIntent,
    Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{Family, Figure, First, Layout, LinePen, Original, Sheet};
use openpaint_practice_app::{Pen, PracticeIntent, PracticeLog, PracticePens, PracticeProjection};
use support::OnSheet;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn pens() -> PracticePens {
    PracticePens::built_in().expect("pens")
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
    let mut first = Sheet::session(1, 1, 1_791_201_600, openpaint_practice::stage_one(5, &[]));
    first.canvas = 3;
    first.lessons[0].finished = finished.then_some(1_791_202_600);
    for (index, at, next) in [(0, 1_791_201_700, 2), (10, 1_791_201_800, 4)] {
        let laid = first.laid_mut(index).expect("an exercise");
        laid.compared(First::of(at, next, Vec::new()));
    }
    let log = PracticeLog::new(vec![first.clone()], Vec::new(), Vec::new());
    let (record, _) = log.begin((1, None), 0x5EED_0079, 1_791_290_000);
    let [width, height] = Layout::of(&vec![None; record.exercises().count()]).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let before = if finished { vec![first] } else { Vec::new() };
    let view = support::view(&record, &[], [width, height]);
    openpaint_practice_app::open_sheet(
        sheet,
        (brush, pens()),
        (record, before, view, openpaint_practice::Tally::default()),
        &VIEWPORT,
    )
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
    if let Some(job) = session.sheet().fit_request() {
        session.sheet().fitted(job.sequence, job.run());
    }
}

fn shown(session: &Session) -> (Option<Family>, bool, (usize, usize), Option<Family>) {
    let p = PracticeProjection::of(session).expect("a practice sheet");
    (p.family, p.warming_up, p.place, p.instructions)
}

/// The original's lines laid over the copy (canvas pixels), while they show.
fn laid(session: &Session) -> Option<Vec<Vec<[f32; 2]>>> {
    let practice = PracticeProjection::of(session)?;
    let frame = practice.sheet.pairs[practice.current?].copy;
    let lines: Vec<Vec<[f32; 2]>> = (practice.overlay.as_ref()?.iter())
        .filter_map(|mark| match &mark.drawn {
            Figure::Line {
                points,
                pen: LinePen::Laid,
                ..
            } => Some(
                (points.iter())
                    .map(|p| frame.to_canvas(*p).map(|v| v as f32))
                    .collect(),
            ),
            _ => None,
        })
        .collect();
    (!lines.is_empty()).then_some(lines)
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
    let pen = PracticeProjection::of(&session).and_then(|p| p.pen);
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
    let practice = PracticeProjection::of(&session).expect("a practice sheet");
    let pair = practice.sheet.pairs[10];
    let Original::Lines(lines) = (*practice.original.as_deref().expect("in hand")).clone() else {
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
    let in_place = laid(&session).expect("the overlay");
    assert_eq!(in_place.len(), lines.len(), "every line laid");
    let placed = pair.copy.to_canvas(lines[0][0]).map(|v| v as f32);
    assert!(close(in_place[0][0], placed, 0.01), "in place");
    practise(&mut session, PracticeIntent::Aligned(true));
    let aligned = laid(&session).expect("the overlay");
    for (line, copy) in aligned.iter().zip(copies.iter().rev()) {
        let (Some(first), Some(last)) = (line.first(), line.last()) else {
            panic!("a line")
        };
        assert!(close(*first, copy[0], 1.5), "{first:?} {:?}", copy[0]);
        assert!(close(*last, copy[copy.len() - 1], 1.5));
    }
    let pen = PracticeProjection::of(&session).and_then(|p| p.pen);
    assert_eq!(pen.map(|(pen, _)| pen), Some(Pen::Correct), "blue now");
    // A correction takes the comparison away; Compare again counts.
    draw(&mut session, &copies[0][..10]);
    assert!(laid(&session).is_none());
    practise(&mut session, PracticeIntent::Compare(true));
    let record = session.sheet().save_request().expect("the record changed");
    assert_eq!(record.laid(10).map(|l| l.compares), Some(2));
    assert_eq!(record.warm_up(), 4);
}
