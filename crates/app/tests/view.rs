//! The canvas view through the session and the finger gestures that drive it: navigation never
//! touches the artwork or its history, a quick pinch fits, and two- and three-finger taps ask
//! for undo and redo; three fingers swiped down open the copy menu (S40).

use openpaint_app::{
    BrushLibrary, CanvasView, ContactPhase, GestureFrame, Gestures, Intent, Session, Toolbox,
    Touch, TouchAction, ViewGesture, ViewIntent, Viewport,
};
use openpaint_core::CanvasSize;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 96.0],
    size: [640.0, 304.0],
    pixels_per_point: 2.0,
};

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(640, 304).expect("size"), brush)
}

fn pinch(zoom: f32) -> Intent {
    Intent::View(ViewIntent::Gesture {
        gesture: ViewGesture {
            pivot: [320.0, 248.0],
            translation: [10.0, 0.0],
            zoom,
            rotation: 0.1,
        },
        viewport: VIEWPORT,
    })
}

#[test]
fn navigation_changes_the_view_and_nothing_else() {
    let mut session = session();
    let artwork = session.artwork().clone();
    assert_eq!(session.projection().view(), CanvasView::FIT, "opens at fit");
    session.dispatch(pinch(2.0)).expect("pinch");
    session.dispatch(pinch(1.5)).expect("pinch");
    let view = session.projection().view();
    assert!((view.zoom() - 3.0).abs() < 1e-5);
    assert!(session.artwork().same_version(&artwork));
    assert_eq!(session.history().undo_steps(), 0, "no history step");
    assert_eq!(session.view(), view);
    session
        .dispatch(Intent::View(ViewIntent::Fit))
        .expect("fit");
    assert_eq!(session.view(), CanvasView::FIT);
}

const FRAME: GestureFrame = GestureFrame {
    pixels_per_point: 2.0,
    now_ms: 0,
    drawing: false,
    pen: false,
};

fn fingers(phase: ContactPhase, spread: f32, time_ms: i64) -> Vec<Touch> {
    [(1, 300.0 - spread), (2, 340.0 + spread)]
        .map(|(id, x)| Touch {
            id,
            phase,
            position: [x, 250.0],
            time_ms,
            open: true,
        })
        .to_vec()
}

fn at(time_ms: i64) -> GestureFrame {
    GestureFrame {
        now_ms: time_ms,
        ..FRAME
    }
}

#[test]
fn a_spread_zooms_in_until_the_fingers_lift() {
    let mut gestures = Gestures::default();
    assert_eq!(
        gestures.update(&fingers(ContactPhase::Down, 0.0, 0), at(0)),
        Default::default()
    );
    let mut zoom = 1.0;
    for step in 1..=10 {
        let moved = gestures.update(
            &fingers(ContactPhase::Move, step as f32 * 20.0, step),
            at(step),
        );
        zoom *= moved.view.map_or(1.0, |view| view.zoom);
        assert!(moved.action.is_none());
    }
    // The span grew elevenfold; the zoom dead zone (ln 0.04) is taken off once.
    let expected = 11.0 * (-0.04f32).exp();
    assert!((zoom - expected).abs() < 1e-3, "{zoom}");
    assert!(gestures.navigating());
    let lifted = gestures.update(&fingers(ContactPhase::Up, 200.0, 600), at(600));
    assert!(lifted.action.is_none(), "a spread is no tap");
    gestures.update(&[], at(616));
    assert!(!gestures.navigating(), "done once the fingers are gone");
}

#[test]
fn taps_ask_for_history_and_a_quick_pinch_fits() {
    let mut gestures = Gestures::default();
    gestures.update(&fingers(ContactPhase::Down, 0.0, 0), at(0));
    let tap = gestures.update(&fingers(ContactPhase::Up, 0.0, 80), at(80));
    assert_eq!(tap.action, Some(TouchAction::Undo));
    gestures.update(&fingers(ContactPhase::Down, 100.0, 200), at(200));
    gestures.update(&fingers(ContactPhase::Move, 10.0, 300), at(300));
    let pinch = gestures.update(&fingers(ContactPhase::Up, 10.0, 350), at(350));
    assert_eq!(pinch.action, Some(TouchAction::Fit));
}

#[test]
fn a_finger_never_pans_while_the_pen_is_in_the_frame() {
    let mut gestures = Gestures::default();
    let one = |phase, x| Touch {
        id: 7,
        phase,
        position: [x, 200.0],
        time_ms: 0,
        open: true,
    };
    let pen = GestureFrame { pen: true, ..FRAME };
    gestures.update(&[one(ContactPhase::Down, 100.0)], pen);
    let moved = gestures.update(&[one(ContactPhase::Move, 200.0)], pen);
    assert_eq!(moved.view, None);
    gestures.update(&[one(ContactPhase::Up, 200.0)], FRAME);
    gestures.update(&[one(ContactPhase::Down, 100.0)], FRAME);
    let moved = gestures.update(&[one(ContactPhase::Move, 200.0)], FRAME);
    assert_eq!(moved.view.map(|view| view.translation), Some([84.0, 0.0]));
}

/// Three fingers 120 px apart, `down` px below where they landed.
fn three(phase: ContactPhase, down: f32, time_ms: i64) -> Vec<Touch> {
    (1..=3)
        .map(|id| Touch {
            id,
            phase,
            position: [200.0 + id as f32 * 120.0, 200.0 + down],
            time_ms,
            open: true,
        })
        .collect()
}

#[test]
fn three_fingers_swiped_down_open_the_copy_menu_and_a_three_finger_tap_still_redoes() {
    let mut gestures = Gestures::default();
    gestures.update(&three(ContactPhase::Down, 0.0, 0), at(0));
    let mut actions = Vec::new();
    for step in 1..=6 {
        let moved = gestures.update(
            &three(ContactPhase::Move, step as f32 * 20.0, step * 30),
            at(step * 30),
        );
        assert_eq!(moved.view, None, "three fingers never navigate");
        actions.extend(moved.action);
    }
    let lifted = gestures.update(&three(ContactPhase::Up, 120.0, 200), at(200));
    actions.extend(lifted.action);
    assert_eq!(
        actions,
        [TouchAction::CopyPaste],
        "once, and no redo on the lift"
    );
    gestures.update(&three(ContactPhase::Down, 0.0, 400), at(400));
    let tap = gestures.update(&three(ContactPhase::Up, 4.0, 480), at(480));
    assert_eq!(tap.action, Some(TouchAction::Redo));
    // Two fingers moved down together pan; they open no menu.
    gestures.update(&fingers(ContactPhase::Down, 0.0, 600), at(600));
    let pan = [1, 2].map(|id| Touch {
        id,
        phase: ContactPhase::Move,
        position: [260.0 + id as f32 * 60.0, 400.0],
        time_ms: 700,
        open: true,
    });
    let panned = gestures.update(&pan, at(700));
    assert!(panned.view.is_some() && panned.action.is_none());
}
