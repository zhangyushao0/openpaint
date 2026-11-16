//! Solids on a practice sheet (S93, S145): a cylinder found in its box, its construction with
//! Sketch, its ends and sides drawn with Draw; Compare lays the exact solid over the copy (its
//! hidden part dashed, V大's construction on request) and the learner's eye judges it: nothing read
//! from the strokes, nothing kept but the choice (D123); an end corrected and compared again.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Ellipse, Exercise, FRAME, Figure, Frame, Judge, Kind, LinePen, Outcome, PEN_WIDTH, Point,
    Solid, SolidCompared, SolidTask, Verdict, resample,
};
use openpaint_practice_app::{Judged, Pen, PracticeIntent, PracticePens, SheetSession};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> SheetSession {
    let cylinder = Exercise::generated(Kind::Solid(Solid::Cylinder), 0x5EED_0093);
    let record = support::laid(vec![cylinder], 0);
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

/// The overlay's red lines: whole and dashed; and whether anything is ringed.
fn red(session: &SheetSession) -> ([usize; 2], bool) {
    let overlay = session.projection().overlay.expect("compared");
    let of = |dashed: bool| {
        (overlay.iter())
            .filter(|mark| {
                matches!(&mark.drawn, Figure::Line { pen, dashed: d, .. } if *pen == LinePen::Answer && *d == dashed)
            })
            .count()
    };
    let ringed = (overlay.iter()).any(|mark| matches!(mark.drawn, Figure::Ring(_)));
    ([of(false), of(true)], ringed)
}

#[test]
fn a_cylinder_is_constructed_drawn_and_compare_lays_the_exact_one_for_the_eye() {
    // The pen the red answer is as thick as: the Draw pen's width on a sheet's frame.
    assert!((f64::from(openpaint_practice_app::INK_DIAMETER) / FRAME - PEN_WIDTH).abs() < 1e-12);
    let mut session = opened();
    let practice = session.projection();
    let cylinder = Kind::Solid(Solid::Cylinder);
    assert_eq!(
        (practice.kind, practice.how),
        (Some(cylinder), Some(cylinder))
    );
    assert!(!practice.aligned_offered);
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = original.downcast::<SolidTask>().expect("a solid").clone();
    let [near, far] = [task.answer.ellipses[0], task.answer.ellipses[1]];
    // The near end's diagonals with Sketch; its ellipse drawn a tenth too wide.
    practise(&mut session, PracticeIntent::Pen(Pen::Sketch));
    for [a, b] in openpaint_practice::diagonals(&task.squares[0].corners) {
        draw(&mut session, &frame, &[a, b]);
    }
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    let wide = Ellipse {
        axes: [near.axes[0] * 1.1, near.axes[1] * 1.1],
        ..near
    };
    draw(&mut session, &frame, &wide.outline(120));
    draw(&mut session, &frame, &far.outline(120));
    for side in &task.answer.sides {
        draw(&mut session, &frame, side);
    }
    practise(&mut session, PracticeIntent::Compare(true));
    let practice = session.projection();
    let compared = practice.compared.as_ref().expect("compared");
    let laid = compared
        .downcast::<SolidCompared>()
        .expect("the solid laid");
    assert_eq!(laid.task, task);
    // The exact cylinder red, its far end's hidden half dashed; nothing ringed or run on.
    let ([whole, dashed], ringed) = red(&session);
    assert!(whole >= 4 && dashed >= 1 && !ringed, "{whole} {dashed}");
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(practice.judged, Some(waiting), "the eye's to judge");
    // The near end corrected and compared again; nothing read or kept but the compares.
    draw(&mut session, &frame, &near.outline(120));
    practise(&mut session, PracticeIntent::Compare(true));
    practise(&mut session, PracticeIntent::Judge(Outcome::Clean));
    let record = session
        .sheet_mut()
        .save_request()
        .expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    assert!(result.metrics.is_empty(), "nothing read");
    let kept = record.laid(0).and_then(|laid| laid.result);
    assert_eq!(kept, Some((Outcome::Clean, Judge::Eye)));
}
