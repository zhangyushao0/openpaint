//! The stroke intent: pen contacts draw, other devices do not, cancel leaves no trace.

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StreamLine,
    StrokeIntent, Toolbox,
};
use openpaint_core::{Artwork, CanvasSize};

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(200, 100).expect("size"), brush)
}

fn sample(x: f32, time_ms: i64) -> InputSample {
    InputSample {
        position: [x, 50.0],
        pressure: 0.5,
        tilt: 0.3,
        azimuth: -1.0,
        time_ms,
    }
}

fn contact(id: i32, device: Device, phase: ContactPhase, samples: &[InputSample]) -> Intent {
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id,
        device,
        phase,
        samples: samples.to_vec(),
    }))
}

/// Down at x = 10, a move of two samples, up at x = 40; times from 1000 ms.
fn swipe(session: &mut Session, id: i32, device: Device) {
    let steps = [
        (ContactPhase::Down, vec![sample(10.0, 1000)]),
        (
            ContactPhase::Move,
            vec![sample(20.0, 1004), sample(30.0, 1008)],
        ),
        (ContactPhase::Up, vec![sample(40.0, 1012)]),
    ];
    for (phase, samples) in steps {
        session
            .dispatch(contact(id, device, phase, &samples))
            .expect("dispatch");
    }
}

fn strokes(artwork: &Artwork) -> usize {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.strokes().len())
        .sum()
}

#[test]
fn a_pen_swipe_commits_one_stroke_timed_from_its_start() {
    let mut session = session();
    let blank = session.artwork().clone();
    swipe(&mut session, 3, Device::Pen);
    assert!(session.active().is_none());
    let artwork = session.artwork();
    assert_eq!(strokes(artwork), 1);
    let layer = artwork.layer(artwork.active()).expect("layer");
    let stroke = layer.strokes().iter().next().expect("stroke");
    let times: Vec<u32> = stroke.samples().iter().map(|s| s.time_ms()).collect();
    assert_eq!(times, [0, 4, 8, 12]);
    let xs: Vec<f32> = stroke.samples().iter().map(|s| s.position().x()).collect();
    assert_eq!(xs, [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(stroke.samples()[0].pressure(), 0.5);
    assert!(!artwork.same_version(&blank));
}

#[test]
fn the_zero_pressure_lift_is_not_part_of_the_stroke() {
    let mut session = session();
    let steps = [
        (ContactPhase::Down, vec![sample(10.0, 0)]),
        (ContactPhase::Move, vec![sample(20.0, 4)]),
        // As Android reports the lift: pressure 0, a little further on.
        (
            ContactPhase::Up,
            vec![InputSample {
                pressure: 0.0,
                ..sample(26.0, 8)
            }],
        ),
    ];
    for (phase, samples) in steps {
        session
            .dispatch(contact(1, Device::Pen, phase, &samples))
            .expect("dispatch");
    }
    let artwork = session.artwork();
    let layer = artwork.layer(artwork.active()).expect("layer");
    let stroke = layer.strokes().iter().next().expect("committed");
    let xs: Vec<f32> = stroke.samples().iter().map(|s| s.position().x()).collect();
    assert_eq!(xs, [10.0, 20.0], "the stroke ends where the ink stopped");
}

#[test]
fn a_lift_with_pressure_still_draws_its_sample() {
    let mut session = session();
    swipe(&mut session, 2, Device::Pen);
    let artwork = session.artwork();
    let layer = artwork.layer(artwork.active()).expect("layer");
    let stroke = layer.strokes().iter().next().expect("committed");
    assert_eq!(stroke.samples().len(), 4);
}

#[test]
fn the_stroke_stays_beside_the_artwork_until_the_pen_lifts() {
    let mut session = session();
    let blank = session.artwork().clone();
    let down = [sample(10.0, 0)];
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Down, &down))
        .expect("down");
    let moved = [sample(15.0, 5)];
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Move, &moved))
        .expect("move");
    assert!(session.artwork().same_version(&blank));
    assert_eq!(session.active().expect("active").samples().len(), 2);
}

#[test]
fn fingers_and_mice_do_not_draw() {
    let mut session = session();
    let blank = session.artwork().clone();
    for device in [Device::Finger, Device::Mouse, Device::Unknown] {
        swipe(&mut session, 0, device);
    }
    assert!(session.artwork().same_version(&blank));
    assert!(session.active().is_none());
}

#[test]
fn the_eraser_end_draws_a_stroke_with_the_style_it_is_given() {
    let mut session = session();
    swipe(&mut session, 5, Device::Eraser);
    assert_eq!(strokes(session.artwork()), 1);
}

#[test]
fn a_cancelled_stroke_leaves_the_artwork_unchanged() {
    let mut session = session();
    let blank = session.artwork().clone();
    let down = [sample(10.0, 0)];
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Down, &down))
        .expect("down");
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Cancel, &down))
        .expect("cancel");
    assert!(session.active().is_none());
    assert!(session.artwork().same_version(&blank));
}

#[test]
fn a_stroke_ended_without_a_lift_commits_what_was_drawn_as_a_lift_would() {
    // S50: the app is left or its window goes while the pen is still down.
    let mut session = session();
    session.set_streamline(StreamLine::new(1.0).expect("amount"));
    let samples: Vec<InputSample> = (0..30u8)
        .map(|i| sample(10.0 + 5.0 * f32::from(i), 1000 + 4 * i64::from(i)))
        .collect();
    let down = contact(1, Device::Pen, ContactPhase::Down, &samples[..1]);
    session.dispatch(down).expect("down");
    let moving = contact(1, Device::Pen, ContactPhase::Move, &samples[1..]);
    session.dispatch(moving).expect("move");
    let before = session.artwork().clone();
    assert_eq!(session.end_stroke(), Ok(true));
    assert!(session.active().is_none());
    assert_eq!(strokes(session.artwork()), 1);
    let stroke = session.artwork().layers()[0]
        .strokes()
        .last()
        .expect("stroke");
    assert_eq!(stroke.samples().len(), 30, "every sample so far");
    let end = stroke.samples().last().expect("end").position();
    assert_eq!(end.x(), 155.0, "the line's end caught up with the pen");
    // One step: undo goes back to before the stroke.
    assert!(session.history().can_undo());
    assert_eq!(session.end_stroke(), Ok(false), "nothing left to end");
    // The pen's later events for it change nothing.
    let after = session.artwork().clone();
    let cancel = contact(1, Device::Pen, ContactPhase::Cancel, &samples[29..]);
    session.dispatch(cancel).expect("cancel");
    let up = contact(1, Device::Pen, ContactPhase::Up, &samples[29..]);
    session.dispatch(up).expect("up");
    assert!(session.artwork().same_version(&after));
    assert!(!after.same_version(&before));
}

#[test]
fn another_contact_neither_extends_nor_ends_the_stroke() {
    let mut session = session();
    let down = [sample(10.0, 0)];
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Down, &down))
        .expect("down");
    let other = [sample(90.0, 5)];
    for phase in [ContactPhase::Move, ContactPhase::Up, ContactPhase::Cancel] {
        session
            .dispatch(contact(2, Device::Pen, phase, &other))
            .expect("other");
    }
    assert_eq!(session.active().expect("active").samples().len(), 1);
}

#[test]
fn a_lost_lift_keeps_the_open_stroke_when_the_pen_touches_again() {
    let mut session = session();
    let down = [sample(10.0, 0)];
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Down, &down))
        .expect("down");
    session
        .dispatch(contact(4, Device::Pen, ContactPhase::Down, &down))
        .expect("second down");
    assert_eq!(strokes(session.artwork()), 1);
    assert!(session.active().is_some());
}

#[test]
fn an_invalid_sample_is_refused_and_the_stroke_keeps_its_samples() {
    let mut session = session();
    let down = [sample(10.0, 0)];
    session
        .dispatch(contact(1, Device::Pen, ContactPhase::Down, &down))
        .expect("down");
    let mut bad = sample(20.0, 4);
    bad.pressure = f32::NAN;
    let result = session.dispatch(contact(
        1,
        Device::Pen,
        ContactPhase::Move,
        &[sample(15.0, 2), bad],
    ));
    assert!(result.is_err());
    assert_eq!(session.active().expect("active").samples().len(), 1);
}

/// A pen moving right along y = 50 every 2 px with a wobble, pressure rising; `streamline` set
/// before it touches down. Returns the session after the lift and the trailing end seen while
/// the pen was still down at its last position.
fn wobbly(streamline: f32) -> (Session, [f32; 2]) {
    let mut session = session();
    session.set_streamline(StreamLine::new(streamline).expect("amount"));
    let at = |i: u16| InputSample {
        position: [
            10.0 + 2.0 * f32::from(i),
            50.0 + 6.0 * (f32::from(i) * 0.4).sin(),
        ],
        pressure: 0.2 + 0.005 * f32::from(i),
        tilt: 0.3,
        azimuth: -1.0,
        time_ms: 1000 + 4 * i64::from(i),
    };
    let down = contact(1, Device::Pen, ContactPhase::Down, &[at(0)]);
    session.dispatch(down).expect("down");
    let moves: Vec<InputSample> = (1..=80).map(at).collect();
    let moving = contact(1, Device::Pen, ContactPhase::Move, &moves);
    session.dispatch(moving).expect("move");
    let active = session.active().expect("drawing").samples();
    let end = active.last().expect("end").position();
    // StreamLine set while drawing waits for the next stroke.
    session.set_streamline(StreamLine::OFF);
    let lift = InputSample {
        pressure: 0.0,
        ..at(81)
    };
    let up = contact(1, Device::Pen, ContactPhase::Up, &[lift]);
    session.dispatch(up).expect("up");
    (session, [end.x(), end.y()])
}

#[test]
fn completed_geometry_reaches_the_lift_point_while_active_geometry_trails() {
    let pen = [170.0, 50.0 + 6.0 * (80.0f32 * 0.4).sin()];
    let (session, trailing) = wobbly(1.0);
    let lag = (pen[0] - trailing[0]).hypot(pen[1] - trailing[1]);
    // The Ink Pen is 12 px: its full leash is three sizes, 36 px.
    assert!(
        (lag - 36.0).abs() < 0.01,
        "the line trails the pen by the leash: {lag}"
    );
    let stroke = session.artwork().layers()[0]
        .strokes()
        .last()
        .expect("stroke")
        .clone();
    let end = stroke.samples().last().expect("end").position();
    assert_eq!(
        [end.x(), end.y()],
        pen,
        "the finished stroke ends where the pen left"
    );
    assert_eq!(stroke.samples().len(), 81, "the lift sample is not drawn");
    let (plain, _) = wobbly(0.0);
    let raw = plain.artwork().layers()[0]
        .strokes()
        .last()
        .expect("stroke")
        .clone();
    for (smooth, pen) in stroke.samples().iter().zip(raw.samples()) {
        assert_eq!(
            smooth.pressure(),
            pen.pressure(),
            "pressure is never filtered"
        );
        assert_eq!(smooth.time_ms(), pen.time_ms());
    }
    let wobble = |samples: &[openpaint_core::StrokeSample]| {
        samples[10..60]
            .iter()
            .map(|s| (s.position().y() - 50.0).abs())
            .fold(0.0f32, f32::max)
    };
    assert!(
        wobble(stroke.samples()) < 0.5 * wobble(raw.samples()),
        "smoothed"
    );
    assert_eq!(
        raw.samples()[40].position().x(),
        90.0,
        "off: exactly the pen"
    );
}
