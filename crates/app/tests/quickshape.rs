//! QuickShape through the session (S42): a stroke held still at its end snaps to its shape,
//! stored as samples along the exact shape with the drawn pressure, one undo step for drawing,
//! snapping and editing.

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, LayerIntent, Session,
    StrokeIntent, Toolbox,
};
use openpaint_core::{Artwork, CanvasSize, Stroke};

const STILL: f32 = 10.0;

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(1200, 900).expect("size"), brush)
}

fn contact(phase: ContactPhase, samples: Vec<InputSample>) -> Intent {
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples,
    }))
}

fn sample([x, y]: [f32; 2], pressure: f32, time_ms: i64) -> InputSample {
    InputSample {
        position: [x, y],
        pressure,
        tilt: 0.2,
        azimuth: 0.5,
        time_ms,
    }
}

/// Draws through `points` 4 ms apart, pressure rising from 0.3 to 0.7, then rests at the end
/// for `rest_ms` while the frames say so, and lifts unless `lift` is false.
fn draw(session: &mut Session, points: &[[f32; 2]], rest_ms: i64, lift: bool) {
    let n = points.len();
    let pressure = |i: usize| 0.3 + 0.4 * i as f32 / (n - 1) as f32;
    let dispatch = |session: &mut Session, intent| session.dispatch(intent).expect("dispatch");
    dispatch(
        session,
        contact(ContactPhase::Down, vec![sample(points[0], pressure(0), 0)]),
    );
    for (i, point) in points.iter().enumerate().skip(1) {
        let now = 4 * i as i64;
        dispatch(
            session,
            contact(ContactPhase::Move, vec![sample(*point, pressure(i), now)]),
        );
        dispatch(
            session,
            Intent::Stroke(StrokeIntent::Hold {
                now_ms: now,
                still: STILL,
            }),
        );
    }
    let end = 4 * (n as i64 - 1);
    for frame in 1..=rest_ms / 16 {
        let now = end + 16 * frame;
        let rest = sample(points[n - 1], 0.7, now);
        dispatch(session, contact(ContactPhase::Move, vec![rest]));
        dispatch(
            session,
            Intent::Stroke(StrokeIntent::Hold {
                now_ms: now,
                still: STILL,
            }),
        );
    }
    if lift {
        let up = sample(points[n - 1], 0.7, end + rest_ms + 4);
        dispatch(session, contact(ContactPhase::Up, vec![up]));
    }
}

/// A wobbly circle of radius 200 round (600, 450), 90 samples, ending near its start.
fn circle() -> Vec<[f32; 2]> {
    (0..90)
        .map(|k| {
            let t = (k as f32 * 4.0).to_radians();
            let r = 200.0 + 3.0 * (k as f32 * 0.7).sin();
            [600.0 + r * t.cos(), 450.0 + r * t.sin()]
        })
        .collect()
}

fn last_stroke(artwork: &Artwork) -> Stroke {
    let layer = artwork.layer(artwork.active()).expect("layer");
    layer.strokes().last().expect("a stroke").clone()
}

fn radii(stroke: &Stroke, centre: [f32; 2]) -> (f32, f32) {
    let r: Vec<f32> = stroke
        .samples()
        .iter()
        .map(|s| (s.position().x() - centre[0]).hypot(s.position().y() - centre[1]))
        .collect();
    let (min, max) = r
        .iter()
        .fold((f32::MAX, 0.0f32), |(a, b), r| (a.min(*r), b.max(*r)));
    (min, max)
}

#[test]
fn a_held_circle_is_stored_as_samples_on_one_circle_with_the_drawn_pressure() {
    let mut session = session();
    draw(&mut session, &circle(), 600, false);
    let active = session.active().expect("still drawing");
    let first = active.samples()[0];
    assert!(active.samples().len() > 60, "dense along the shape");
    draw_lift(&mut session);
    let stroke = last_stroke(session.artwork());
    let handles = session
        .projection()
        .shape()
        .expect("offered")
        .handles
        .clone();
    let centre = [
        handles.iter().map(|h| h[0]).sum::<f32>() / 4.0,
        handles.iter().map(|h| h[1]).sum::<f32>() / 4.0,
    ];
    let (min, max) = radii(&stroke, centre);
    assert!(max - min < 0.01, "one circle: {min} to {max}");
    assert!((min - 200.0).abs() < 4.0, "{min}");
    let samples = stroke.samples();
    assert_eq!(samples[0], first);
    assert!(
        (samples[0].pressure() - 0.3).abs() < 0.02,
        "{}",
        samples[0].pressure()
    );
    let last = samples.last().expect("end");
    assert!(
        (last.pressure() - 0.7).abs() < 0.02,
        "the drawn taper: {}",
        last.pressure()
    );
    assert!(
        samples
            .windows(2)
            .all(|pair| pair[0].time_ms() <= pair[1].time_ms())
    );
    assert_eq!(session.history().undo_steps(), 1);
    session
        .dispatch(Intent::History(openpaint_app::HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(
        session.artwork().layers()[0].strokes().len(),
        0,
        "back to before the stroke"
    );
}

fn draw_lift(session: &mut Session) {
    let up = sample([0.0, 0.0], 0.0, 10_000);
    session
        .dispatch(contact(ContactPhase::Up, vec![up]))
        .expect("lift");
}

#[test]
fn a_short_rest_or_a_scribble_stays_as_drawn() {
    for (points, rest) in [(circle(), 300), (scribble(), 700)] {
        let mut session = session();
        draw(&mut session, &points, rest, true);
        let stroke = last_stroke(session.artwork());
        assert!(stroke.samples().len() >= points.len(), "as drawn");
        assert_eq!(stroke.samples()[1].position().x(), points[1][0]);
        assert!(session.projection().shape().is_none());
    }
}

fn scribble() -> Vec<[f32; 2]> {
    (0..120)
        .map(|k| {
            let t = k as f32 * 0.08;
            [
                500.0 + 150.0 * (t * 1.3).sin() + 60.0 * (t * 3.7).cos(),
                400.0 + 120.0 * (t * 2.1).cos(),
            ]
        })
        .collect()
}

#[test]
fn still_pressing_the_snapped_line_follows_the_pen_to_its_end() {
    let mut session = session();
    let line: Vec<[f32; 2]> = (0..60)
        .map(|k| [100.0 + 8.0 * k as f32, 300.0 + 0.5 * (k as f32).sin()])
        .collect();
    draw(&mut session, &line, 600, false);
    let moved = sample([500.0, 600.0], 0.6, 2_000);
    session
        .dispatch(contact(ContactPhase::Move, vec![moved]))
        .expect("move");
    session
        .dispatch(contact(
            ContactPhase::Up,
            vec![sample([500.0, 600.0], 0.0, 2_004)],
        ))
        .expect("up");
    let stroke = last_stroke(session.artwork());
    let (first, last) = (stroke.samples()[0], *stroke.samples().last().expect("end"));
    assert_eq!([first.position().x(), first.position().y()], line[0]);
    let end = [last.position().x(), last.position().y()];
    assert!(
        (end[0] - 500.0).abs() < 0.5 && (end[1] - 600.0).abs() < 0.5,
        "{end:?}"
    );
    // Every sample on the straight line between.
    for s in stroke.samples() {
        let (x, y) = (s.position().x() - 100.0, s.position().y() - 300.0);
        assert!((x * 300.0 - y * 400.0).abs() / 500.0 < 0.01, "{x} {y}");
    }
}

#[test]
fn editing_the_shape_replaces_it_in_the_same_step_until_anything_else() {
    let mut session = session();
    draw(&mut session, &circle(), 600, true);
    let shape = session.projection().shape().cloned().expect("offered");
    assert!(!shape.editing);
    assert_eq!(shape.handles.len(), 4, "an ellipse's axis ends");
    session
        .dispatch(Intent::Stroke(StrokeIntent::EditShape(true)))
        .expect("edit");
    assert!(session.projection().shape().expect("editing").editing);
    let [x, y] = shape.handles[0];
    let (cx, cy) = (
        shape.handles.iter().map(|h| h[0]).sum::<f32>() / 4.0,
        shape.handles.iter().map(|h| h[1]).sum::<f32>() / 4.0,
    );
    let to = [cx + (x - cx) * 1.5, cy + (y - cy) * 1.5];
    session
        .dispatch(Intent::Stroke(StrokeIntent::MoveHandle { handle: 0, to }))
        .expect("drag");
    let handles = session
        .projection()
        .shape()
        .expect("editing")
        .handles
        .clone();
    assert!((handles[0][0] - to[0]).abs() < 0.01 && (handles[0][1] - to[1]).abs() < 0.01);
    let stroke = last_stroke(session.artwork());
    let (min, max) = radii(&stroke, [cx, cy]);
    assert!(
        max > 280.0 && min < 210.0,
        "stretched to an ellipse: {min} {max}"
    );
    assert_eq!(session.artwork().layers()[0].strokes().len(), 1);
    assert_eq!(
        session.history().undo_steps(),
        1,
        "drawing, snapping and editing: one step"
    );
    session
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("add layer");
    assert!(
        session.projection().shape().is_none(),
        "any other edit ends editing"
    );
    session
        .dispatch(Intent::History(openpaint_app::HistoryIntent::Undo))
        .expect("undo layer");
    session
        .dispatch(Intent::History(openpaint_app::HistoryIntent::Undo))
        .expect("undo shape");
    assert_eq!(session.artwork().layers()[0].strokes().len(), 0);
}
