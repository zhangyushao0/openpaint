use super::*;
use openpaint_app::InputSample;

const AREA: ScreenArea = ScreenArea {
    origin: [0, 84],
    size: [400, 300],
};

/// A canvas as large as the area, so at fit it shows at 0.92 around the area's centre.
fn frame(covers: &[ScreenArea]) -> CanvasFrame<'_> {
    CanvasFrame {
        area: AREA,
        covers,
        canvas: CanvasSize::new(400, 300).expect("size"),
        view: CanvasView::FIT,
        pixels_per_point: 1.0,
        now_ms: 0,
        picking: false,
        drawing: false,
        control: false,
    }
}

fn contact(id: i32, device: Device, phase: ContactPhase, x: f32, y: f32) -> Contact {
    let sample = InputSample {
        position: [x, y],
        pressure: 1.0,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms: 0,
    };
    Contact {
        id,
        device,
        phase,
        samples: vec![sample],
    }
}

fn pen(id: i32, phase: ContactPhase, x: f32, y: f32) -> InputBatch {
    InputBatch {
        contacts: vec![contact(id, Device::Pen, phase, x, y)],
    }
}

fn fingers(phase: ContactPhase, points: &[[f32; 2]]) -> InputBatch {
    let contacts = (0..)
        .zip(points)
        .map(|(id, [x, y])| contact(10 + id, Device::Finger, phase, *x, *y))
        .collect();
    InputBatch { contacts }
}

fn positions(intents: &[Intent]) -> Vec<[f32; 2]> {
    intents
        .iter()
        .filter_map(|intent| match intent {
            Intent::Stroke(StrokeIntent::Contact(contact)) => Some(contact.samples[0].position),
            _ => None,
        })
        .collect()
}

/// Where the screen point `at` lands on the canvas at fit.
fn at_fit(at: [f32; 2]) -> [f32; 2] {
    frame(&[]).mapping().to_canvas(at)
}

#[test]
fn a_contact_that_starts_on_a_panel_over_the_canvas_is_not_routed() {
    let mut input = CanvasInput::default();
    let panel = ScreenArea {
        origin: [200, 100],
        size: [100, 100],
    };
    let side = ScreenArea {
        origin: [0, 300],
        size: [20, 50],
    };
    let batches = [
        pen(1, ContactPhase::Down, 250.0, 150.0),
        pen(1, ContactPhase::Move, 50.0, 150.0),
        pen(1, ContactPhase::Up, 40.0, 150.0),
        pen(2, ContactPhase::Down, 150.0, 150.0),
        pen(3, ContactPhase::Down, 10.0, 320.0),
    ];
    let intents = input.route(&frame(&[side, panel]), &batches);
    assert_eq!(positions(&intents), [at_fit([150.0, 150.0])]);
}

#[test]
fn the_pen_is_owned_until_it_lifts_and_lands_on_the_canvas_through_the_view() {
    let mut input = CanvasInput::default();
    let batches = [
        pen(1, ContactPhase::Down, 100.0, 200.0),
        // Moving over the top bar keeps drawing.
        pen(1, ContactPhase::Move, 112.0, 20.0),
        pen(1, ContactPhase::Up, 114.0, 190.0),
        pen(1, ContactPhase::Move, 116.0, 220.0),
    ];
    let intents = input.route(&frame(&[]), &batches);
    let expected = [[100.0, 200.0], [112.0, 20.0], [114.0, 190.0]].map(at_fit);
    assert_eq!(positions(&intents), expected);
    // The area's centre is the canvas's centre at fit.
    assert_eq!(at_fit([200.0, 234.0]), [200.0, 150.0]);
}

#[test]
fn the_pen_draws_nothing_from_outside_the_canvas_or_the_area() {
    let mut input = CanvasInput::default();
    let batches = [
        // The surround of the fitted canvas, inside the area.
        pen(2, ContactPhase::Down, 5.0, 200.0),
        pen(2, ContactPhase::Move, 100.0, 200.0),
        pen(2, ContactPhase::Up, 100.0, 210.0),
        // The top bar.
        pen(3, ContactPhase::Down, 100.0, 40.0),
        pen(4, ContactPhase::Down, 500.0, 200.0),
    ];
    assert!(input.route(&frame(&[]), &batches).is_empty());
}

#[test]
fn cancel_is_passed_on_and_ends_ownership() {
    let mut input = CanvasInput::default();
    let batches = [
        pen(1, ContactPhase::Down, 100.0, 200.0),
        pen(1, ContactPhase::Cancel, 100.0, 200.0),
        pen(1, ContactPhase::Move, 100.0, 210.0),
        pen(1, ContactPhase::Hover, 100.0, 210.0),
    ];
    assert_eq!(input.route(&frame(&[]), &batches).len(), 2);
}

#[test]
fn finger_taps_undo_and_redo_and_never_draw() {
    let mut input = CanvasInput::default();
    let two = [[150.0, 200.0], [250.0, 200.0]];
    let three = [[150.0, 200.0], [200.0, 200.0], [250.0, 200.0]];
    input.route(&frame(&[]), &[fingers(ContactPhase::Down, &two)]);
    let undo = input.route(&frame(&[]), &[fingers(ContactPhase::Up, &two)]);
    assert_eq!(undo, [Intent::History(HistoryIntent::Undo)]);
    input.route(&frame(&[]), &[fingers(ContactPhase::Down, &three)]);
    let redo = input.route(&frame(&[]), &[fingers(ContactPhase::Up, &three)]);
    assert_eq!(redo, [Intent::History(HistoryIntent::Redo)]);
}

#[test]
fn a_control_that_takes_a_contact_resets_the_gestures() {
    let mut input = CanvasInput::default();
    let two = [[150.0, 200.0], [250.0, 200.0]];
    input.route(&frame(&[]), &[fingers(ContactPhase::Down, &two)]);
    let control = CanvasFrame {
        control: true,
        ..frame(&[])
    };
    assert!(input.route(&control, &[]).is_empty());
    let lifted = input.route(&frame(&[]), &[fingers(ContactPhase::Up, &two)]);
    assert!(lifted.is_empty(), "the lift is no tap any more: {lifted:?}");
}

#[test]
fn a_pinch_moves_the_view_until_the_fingers_lift() {
    let mut input = CanvasInput::default();
    input.route(
        &frame(&[]),
        &[fingers(
            ContactPhase::Down,
            &[[180.0, 200.0], [220.0, 200.0]],
        )],
    );
    let spread = [[100.0, 200.0], [300.0, 200.0]];
    let moved = input.route(&frame(&[]), &[fingers(ContactPhase::Move, &spread)]);
    let Some(Intent::View(ViewIntent::Gesture { gesture, viewport })) = moved.first() else {
        panic!("a view gesture: {moved:?}");
    };
    assert!(gesture.zoom > 4.0 && viewport.size == [400.0, 300.0]);
    assert_eq!(input.navigated(), Some(i64::MAX), "navigating");
    let lifted = input.route(&frame(&[]), &[fingers(ContactPhase::Up, &spread)]);
    assert!(lifted.is_empty(), "a pinch is no tap: {lifted:?}");
    assert_eq!(input.navigated(), Some(0), "stopped at the input clock's 0");
}

#[test]
fn while_picking_a_finger_picks_instead_of_navigating() {
    let mut input = CanvasInput::default();
    let picking = CanvasFrame {
        picking: true,
        ..frame(&[])
    };
    let batch = fingers(ContactPhase::Down, &[[5.0, 200.0]]);
    let intents = input.route(&picking, &[batch]);
    assert_eq!(positions(&intents), [at_fit([5.0, 200.0])]);
}
