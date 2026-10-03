//! Layer intents: each panel action is one history step that undo takes back, the projection
//! offers only what can act, choosing a layer is no step, and one opacity touch is one step.

use openpaint_app::{
    AppError, Availability, Contact, ContactPhase, Device, EditorAction, HistoryIntent,
    InputSample, Intent, LayerIntent, Session, StrokeIntent,
};
use openpaint_app::{BrushLibrary, Toolbox};
use openpaint_core::{CanvasSize, LayerId};

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(200, 100).expect("size"), brush)
}

fn layer(session: &mut Session, intent: LayerIntent) -> Result<(), AppError> {
    session.dispatch(Intent::Layer(intent))
}

fn undo(session: &mut Session) {
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
}

fn redo(session: &mut Session) {
    session
        .dispatch(Intent::History(HistoryIntent::Redo))
        .expect("redo");
}

fn pen(phase: ContactPhase, x: f32) -> Intent {
    let sample = InputSample {
        position: [x, 50.0],
        pressure: 0.5,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms: 0,
    };
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    }))
}

fn draw(session: &mut Session, x: f32) {
    for (phase, dx) in [(ContactPhase::Down, 0.0), (ContactPhase::Up, 10.0)] {
        session.dispatch(pen(phase, x + dx)).expect("stroke");
    }
}

/// Bottom to top.
fn ids(session: &Session) -> Vec<LayerId> {
    session.artwork().layers().iter().map(|l| l.id()).collect()
}

fn offered(session: &Session, action: EditorAction) -> Availability {
    session.projection().availability(action)
}

#[test]
fn every_panel_action_is_one_step_that_undo_takes_back() {
    let mut session = session();
    draw(&mut session, 10.0);
    let bottom = session.artwork().active();
    layer(&mut session, LayerIntent::Add).expect("add");
    let top = session.artwork().active();
    assert_eq!(ids(&session), [bottom, top]);
    let actions = [
        LayerIntent::SetVisible {
            layer: top,
            visible: false,
        },
        LayerIntent::SetOpacity {
            layer: top,
            opacity: 0.25,
            settled: true,
        },
        LayerIntent::Lower(top),
        LayerIntent::Raise(top),
        LayerIntent::Delete(bottom),
    ];
    for intent in actions {
        let before = session.artwork().clone();
        layer(&mut session, intent.clone()).expect("applies");
        let after = session.artwork().clone();
        assert!(!after.same_version(&before), "{intent:?} changed nothing");
        undo(&mut session);
        assert!(session.artwork().same_version(&before), "{intent:?} undo");
        redo(&mut session);
        assert!(session.artwork().same_version(&after), "{intent:?} redo");
    }
    assert_eq!(ids(&session), [top]);
    assert_eq!(
        session.history().undo_steps(),
        7,
        "stroke, add and five actions"
    );
}

#[test]
fn the_projection_offers_only_what_can_act_on_the_active_layer() {
    let mut session = session();
    use Availability::{Available, NothingLeft};
    let lone = |s: &Session| {
        [
            EditorAction::AddLayer,
            EditorAction::DeleteLayer,
            EditorAction::RaiseLayer,
            EditorAction::LowerLayer,
        ]
        .map(|action| offered(s, action))
    };
    assert_eq!(
        lone(&session),
        [Available, NothingLeft, NothingLeft, NothingLeft]
    );
    let first = session.artwork().active();
    layer(&mut session, LayerIntent::Add).expect("add");
    assert_eq!(
        lone(&session),
        [Available, Available, NothingLeft, Available]
    );
    layer(&mut session, LayerIntent::Select(first)).expect("select");
    assert_eq!(
        lone(&session),
        [Available, Available, Available, NothingLeft]
    );
    let refused = layer(&mut session, LayerIntent::Lower(first)).expect_err("bottom");
    assert_eq!(
        refused,
        AppError::Unavailable {
            action: EditorAction::LowerLayer,
            availability: NothingLeft
        }
    );
    let list = session.projection().layers().cloned().expect("layers");
    assert_eq!(list.rows.len(), 2);
    assert_eq!(list.active_row().map(|row| row.id), Some(first));
}

#[test]
fn choosing_a_layer_is_no_step_and_keeps_redo() {
    let mut session = session();
    let first = session.artwork().active();
    layer(&mut session, LayerIntent::Add).expect("add");
    draw(&mut session, 20.0);
    undo(&mut session);
    let steps = session.history().undo_steps();
    layer(&mut session, LayerIntent::Select(first)).expect("select");
    assert_eq!(session.artwork().active(), first);
    assert_eq!(session.history().undo_steps(), steps);
    assert!(
        session.history().can_redo(),
        "choosing a layer is not an edit"
    );
    undo(&mut session);
    assert_eq!(ids(&session), [first], "undo goes back past the choice");
}

#[test]
fn one_opacity_touch_is_one_step() {
    let mut session = session();
    let only = session.artwork().active();
    draw(&mut session, 10.0);
    let drawn = session.artwork().clone();
    let slide = |opacity, settled| LayerIntent::SetOpacity {
        layer: only,
        opacity,
        settled,
    };
    // The first value equals the current one: it must not amend the stroke's step.
    for (opacity, settled) in [(1.0, false), (0.8, false), (0.5, false), (0.4, true)] {
        layer(&mut session, slide(opacity, settled)).expect("slide");
    }
    assert_eq!(session.history().undo_steps(), 2);
    let faded = session.artwork().layers()[0].props().opacity();
    assert!((faded - 0.4).abs() < f32::EPSILON);
    // A second touch is a step of its own.
    layer(&mut session, slide(0.2, false)).expect("slide");
    layer(&mut session, slide(0.1, true)).expect("slide");
    assert_eq!(session.history().undo_steps(), 3);
    undo(&mut session);
    undo(&mut session);
    assert!(session.artwork().same_version(&drawn));
}

#[test]
fn layers_wait_while_a_stroke_is_drawn() {
    let mut session = session();
    session
        .dispatch(pen(ContactPhase::Down, 10.0))
        .expect("down");
    assert_eq!(
        offered(&session, EditorAction::AddLayer),
        Availability::StrokeInProgress
    );
    assert!(layer(&mut session, LayerIntent::Add).is_err());
    session.dispatch(pen(ContactPhase::Up, 20.0)).expect("up");
    layer(&mut session, LayerIntent::Add).expect("add after the stroke");
    assert_eq!(ids(&session).len(), 2);
}
