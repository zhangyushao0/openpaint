use super::*;
use openpaint_app::{CanvasView, Contact, InputSample};
use openpaint_core::CanvasSize;

const AREA: ScreenArea = ScreenArea {
    origin: [0, 84],
    size: [400, 300],
};
const SWATCH: ScreenArea = ScreenArea {
    origin: [350, 0],
    size: [48, 48],
};
const SIDE: ScreenArea = ScreenArea {
    origin: [0, 200],
    size: [40, 100],
};

fn frame(now_ms: i64) -> CanvasFrame<'static> {
    CanvasFrame {
        area: crate::components::CanvasArea {
            screen: AREA,
            fit: AREA,
        },
        covers: &[SIDE],
        canvas: CanvasSize::new(400, 300).expect("size"),
        view: CanvasView::FIT,
        pixels_per_point: 1.0,
        now_ms,
        picking: false,
        selecting: false,
        transform: None,
        adjusting: false,
        drawing: false,
        buttons: openpaint_app::ButtonMap::default(),
        control: false,
    }
}

fn drop_frame() -> DropFrame {
    DropFrame {
        swatch: Some(SWATCH),
        color: Some(Color::BLACK),
        threshold: 0.6,
        bar: None,
    }
}

fn batch(device: Device, phase: ContactPhase, at: [f32; 2], time_ms: i64) -> InputBatch {
    let sample = InputSample {
        position: at,
        pressure: 1.0,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    InputBatch {
        contacts: vec![Contact {
            id: 3,
            device,
            phase,
            samples: vec![sample],
        }],
        pen: None,
    }
}

/// Routes one batch at `time_ms` (the frame's clock too); returns the fill intents and how many
/// contacts were left for the canvas.
fn step(
    drop: &mut ColorDrop,
    device: Device,
    phase: ContactPhase,
    at: [f32; 2],
    time_ms: i64,
) -> (Vec<Intent>, usize) {
    let (intents, left) = drop.route(
        &frame(time_ms),
        &drop_frame(),
        &[batch(device, phase, at, time_ms)],
    );
    (intents, left.iter().map(|b| b.contacts.len()).sum())
}

fn requests(intents: &[Intent]) -> Vec<([f32; 2], f32)> {
    intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::Fill(FillIntent::Request {
                point, threshold, ..
            }) => Some((*point, *threshold)),
            _ => None,
        })
        .collect()
}

fn has(intents: &[Intent], wanted: &FillIntent) -> bool {
    intents.contains(&Intent::Fill(wanted.clone()))
}

#[test]
fn the_threshold_moves_past_a_dead_zone_in_half_percent_steps() {
    assert_eq!(adjusted(0.6, 0.0), 0.6);
    assert_eq!(adjusted(0.6, 4.0), 0.6);
    assert_eq!(adjusted(0.6, -3.0), 0.6);
    assert!((adjusted(0.6, 64.0) - 0.7).abs() < 1e-6);
    assert!(
        (adjusted(0.6, -7.0) - 0.595).abs() < 1e-6,
        "rounded to 0.5 %"
    );
    assert_eq!(adjusted(0.6, -1000.0), 0.0);
    assert_eq!(adjusted(0.6, 1000.0), 1.0);
}

#[test]
fn a_drop_dragged_from_the_swatch_fills_where_it_lifts_and_never_draws() {
    let mut drop = ColorDrop::default();
    let (intents, left) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Down,
        [370.0, 20.0],
        0,
    );
    assert_eq!(
        (intents.len(), left),
        (0, 0),
        "the press belongs to the drop"
    );
    assert_eq!(drop.carried(), None, "not before 8 points");
    let (intents, left) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Move,
        [200.0, 200.0],
        50,
    );
    assert_eq!((intents.len(), left), (0, 0));
    assert_eq!(drop.carried(), Some([200.0, 200.0]));
    assert_eq!(drop.bar(), Some(DropBar::Dropping(None)));
    let (intents, _) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Up,
        [200.0, 200.0],
        100,
    );
    let point = frame(0).mapping().to_canvas([200.0, 200.0]);
    assert_eq!(requests(&intents), [(point, 0.6)]);
    assert!(has(&intents, &FillIntent::Commit));
    assert_eq!(drop.bar(), Some(DropBar::Offered));
}

#[test]
fn a_tap_on_the_swatch_is_no_drop_and_a_lift_over_a_control_cancels() {
    let mut drop = ColorDrop::default();
    step(&mut drop, Device::Pen, ContactPhase::Down, [370.0, 20.0], 0);
    let (intents, _) = step(&mut drop, Device::Pen, ContactPhase::Up, [372.0, 21.0], 60);
    assert!(intents.is_empty(), "the colour panel opens instead");
    step(
        &mut drop,
        Device::Pen,
        ContactPhase::Down,
        [370.0, 20.0],
        100,
    );
    step(
        &mut drop,
        Device::Pen,
        ContactPhase::Move,
        [20.0, 250.0],
        150,
    );
    let (intents, _) = step(&mut drop, Device::Pen, ContactPhase::Up, [20.0, 250.0], 200);
    assert_eq!(intents, [Intent::Fill(FillIntent::Cancel)]);
    assert_eq!(drop.bar(), None);
}

#[test]
fn held_still_the_drop_anchors_and_sliding_sets_the_threshold() {
    let mut drop = ColorDrop::default();
    step(
        &mut drop,
        Device::Finger,
        ContactPhase::Down,
        [370.0, 20.0],
        0,
    );
    step(
        &mut drop,
        Device::Finger,
        ContactPhase::Move,
        [200.0, 200.0],
        100,
    );
    let (intents, _) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Move,
        [201.0, 201.0],
        300,
    );
    assert!(requests(&intents).is_empty(), "not yet still for 0.4 s");
    let (intents, _) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Move,
        [202.0, 200.0],
        520,
    );
    let anchor = frame(0).mapping().to_canvas([202.0, 200.0]);
    assert_eq!(requests(&intents), [(anchor, 0.6)], "anchored: the preview");
    assert_eq!(drop.bar(), Some(DropBar::Dropping(Some(0.6))));
    let (intents, _) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Move,
        [266.0, 230.0],
        600,
    );
    let [(point, threshold)] = requests(&intents)[..] else {
        panic!("one request: {intents:?}");
    };
    assert_eq!(point, anchor, "the seed stays where it anchored");
    assert!((threshold - 0.7).abs() < 1e-6);
    let (intents, _) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Up,
        [266.0, 230.0],
        650,
    );
    assert_eq!(
        intents,
        [Intent::Fill(FillIntent::Commit)],
        "the last request is kept"
    );
}

#[test]
fn continuing_lets_pen_taps_fill_and_fingers_still_navigate() {
    let mut drop = ColorDrop::default();
    drop.set_continuing(true);
    assert_eq!(drop.bar(), Some(DropBar::Continuing));
    let (_, left) = step(
        &mut drop,
        Device::Finger,
        ContactPhase::Down,
        [100.0, 150.0],
        0,
    );
    assert_eq!(left, 1, "a finger goes on to the canvas");
    step(
        &mut drop,
        Device::Finger,
        ContactPhase::Up,
        [100.0, 150.0],
        50,
    );
    let (_, left) = step(
        &mut drop,
        Device::Pen,
        ContactPhase::Down,
        [150.0, 150.0],
        100,
    );
    assert_eq!(left, 0);
    let (intents, _) = step(
        &mut drop,
        Device::Pen,
        ContactPhase::Up,
        [150.0, 150.0],
        160,
    );
    assert_eq!(requests(&intents).len(), 1);
    assert!(has(&intents, &FillIntent::Commit));
    assert_eq!(drop.bar(), Some(DropBar::Continuing), "until Done");
    drop.set_continuing(false);
    let (_, left) = step(
        &mut drop,
        Device::Pen,
        ContactPhase::Down,
        [150.0, 150.0],
        200,
    );
    assert_eq!(left, 1, "the pen draws again");
}
