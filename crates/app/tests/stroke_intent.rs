//! The stroke intent: pen contacts draw, other devices do not, cancel leaves no trace.

use openpaint_app::{Contact, ContactPhase, Device, InputSample, Intent, Session, StrokeIntent};
use openpaint_core::{Artwork, CanvasSize};

fn session() -> Session {
    Session::new(CanvasSize::new(200, 100).expect("size")).expect("session")
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
fn fingers_mice_and_the_eraser_do_not_draw() {
    let mut session = session();
    let blank = session.artwork().clone();
    for device in [
        Device::Finger,
        Device::Mouse,
        Device::Eraser,
        Device::Unknown,
    ] {
        swipe(&mut session, 0, device);
    }
    assert!(session.artwork().same_version(&blank));
    assert!(session.active().is_none());
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
