//! Solids on a practice sheet (S93): a cylinder found in its box, its construction with Sketch
//! never read, its ends and sides drawn with Draw and compared against the exact solid (each end
//! fitted, the sides read, the error kept at the first Compare), an end corrected in place of what
//! it redraws.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StrokeIntent,
    Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Ellipse, Exercise, FRAME, Frame, Kind, Metric, PEN_WIDTH, Point, Solid, SolidTask, distance,
    resample, value,
};
use openpaint_practice_app::{Pen, PracticeIntent, PracticePens, PracticeProjection};
use support::OnSheet;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn opened() -> Session {
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
    openpaint_practice_app::open_sheet(sheet, (brush, pens), opening, &VIEWPORT)
}

/// A pen stroke along `points` (frame units of `frame`), sampled every half pixel or so.
fn draw(session: &mut Session, frame: &Frame, points: &[Point]) {
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

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_900_000).expect("practised");
}

/// The solid's check over the copy.
fn checked(session: &Session) -> openpaint_practice::SolidCheck {
    let practice = PracticeProjection::of(session).expect("a sheet");
    let compared = practice.compared.as_ref().expect("compared");
    let solid = compared.downcast::<openpaint_practice::SolidCompared>();
    solid.expect("the solid's check").check.clone()
}

#[test]
fn a_cylinder_is_constructed_drawn_compared_and_an_end_corrected() {
    // The pen the red answer is as thick as: the Draw pen's width on a sheet's frame.
    assert!((f64::from(openpaint_practice_app::INK_DIAMETER) / FRAME - PEN_WIDTH).abs() < 1e-12);
    let mut session = opened();
    let practice = PracticeProjection::of(&session).expect("a sheet");
    let cylinder = Kind::Solid(Solid::Cylinder);
    assert_eq!(
        (practice.kind, practice.how),
        (Some(cylinder), Some(cylinder))
    );
    assert!(!practice.aligned_offered);
    let frame = practice.sheet.pairs[0].copy;
    let original = practice.original.as_deref().expect("in hand");
    let task = original.downcast::<SolidTask>().expect("a solid");
    let [near, far] = [task.answer.ellipses[0], task.answer.ellipses[1]];
    // The near end's diagonals with Sketch, never read; its ellipse drawn a tenth too wide.
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
    let check = checked(&session);
    let fitted = check.curves[0].fitted.expect("the near end fitted");
    assert!(
        (fitted.axes[0] / near.axes[0] - 1.1).abs() < 0.02,
        "{fitted:?}"
    );
    let far_fit = check.curves[1].fitted.expect("the far end fitted");
    assert!(distance(far_fit.centre, far.centre) < 0.003);
    assert!(check.sides.iter().all(|side| !side.is_empty()));
    let pen = PracticeProjection::of(&session).and_then(|p| p.pen);
    assert_eq!(pen.map(|(pen, _)| pen), Some(Pen::Correct));
    let first = value(&check.metrics, Metric::SolidOff).expect("measured");
    assert!(first > 0.0);
    // The near end corrected: read alone in place of the first try, the first error kept.
    draw(&mut session, &frame, &near.outline(120));
    practise(&mut session, PracticeIntent::Compare(true));
    let check = checked(&session);
    let fitted = check.curves[0].fitted.expect("the correction fitted");
    assert!(
        (fitted.axes[0] / near.axes[0] - 1.0).abs() < 0.01,
        "{fitted:?}"
    );
    let now = value(&check.metrics, Metric::SolidOff).expect("measured");
    assert!(now < first, "{now} against {first}");
    let record = session.sheet().save_request().expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    assert_eq!(value(&result.metrics, Metric::SolidOff), Some(first));
}
