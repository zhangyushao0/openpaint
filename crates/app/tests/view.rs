//! The canvas view through the session and the finger gestures that drive it: navigation never
//! touches the artwork or its history, a quick pinch fits, a view fitting a rectangle of the
//! canvas maps it as a canvas of its own (S121a), and two- and three-finger taps ask for undo
//! and redo; three fingers swiped down open the copy menu (S40).

use openpaint_app::{
    BrushLibrary, CanvasView, ContactPhase, FIT_MARGIN, Fitted, GestureFrame, Gestures, Intent,
    MAX_ZOOM, MIN_ZOOM, Session, Toolbox, Touch, TouchAction, ViewGesture, ViewIntent, Viewport,
};
use openpaint_core::{Artwork, CanvasSize};

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

#[test]
fn views_are_checked() {
    assert!(CanvasView::new([0.0, 0.0], 8.5, 0.0).is_none());
    assert!(CanvasView::new([f32::INFINITY, 0.0], 1.0, 0.0).is_none());
    assert!(CanvasView::new([0.0, 0.0], 1.0, f32::NAN).is_none());
    assert_eq!(CanvasView::new([0.0; 2], 1.0, 0.0), Some(CanvasView::FIT));
}

/// A sheet 2920 x 3160 whose view fits its second row.
const SHEET: [u32; 2] = [2920, 3160];
const ROW: Fitted = Fitted {
    origin: [0, 520],
    size: [2920, 560],
};

/// A sheet's session showing its row, as a mode opens it.
fn sheet() -> (CanvasSize, Session) {
    let canvas = CanvasSize::new(SHEET[0], SHEET[1]).expect("size");
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    let mut held = Session::open(Artwork::new(canvas), brush);
    *held.view_mut() = CanvasView::fitting(Some(ROW));
    (canvas, held)
}

fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
}

/// The view's map of `canvas` and the same view, fitting the whole canvas, of a canvas of the
/// row's size agree on every point once shifted by the row's origin.
fn maps_as_the_row_alone(view: CanvasView, canvas: CanvasSize) {
    let alone = CanvasSize::new(ROW.size[0], ROW.size[1]).expect("size");
    let whole = CanvasView::new(view.pan(), view.zoom(), view.rotation()).expect("view");
    let [row, single] = [
        view.mapping(canvas, &VIEWPORT),
        whole.mapping(alone, &VIEWPORT),
    ];
    assert!((row.scale() - single.scale()).abs() < 1e-6);
    for [x, y] in [[0.0, 0.0], [1460.0, 280.0], [2919.0, 559.0], [-40.0, 900.0]] {
        let shifted = [x + ROW.origin[0] as f32, y + ROW.origin[1] as f32];
        let [a, b] = [row.to_screen(shifted), single.to_screen([x, y])];
        assert!(near(a, b), "{view:?} at {x}, {y}: {a:?} against {b:?}");
    }
}

#[test]
fn a_view_fitting_a_rectangle_maps_it_as_a_canvas_of_its_size_and_keeps_it() {
    let (canvas, mut session) = sheet();
    maps_as_the_row_alone(session.view(), canvas);
    session.dispatch(pinch(2.0)).expect("pinch");
    let pinched = session.view();
    assert_eq!(pinched.fits(), Some(ROW), "a pinch keeps the rectangle");
    assert!((pinched.zoom() - 2.0).abs() < 1e-5);
    maps_as_the_row_alone(pinched, canvas);
    // The zoom's limits are of the row's fit, not the sheet's.
    let row_fit = (640.0 / 2920.0f32).min(304.0 / 560.0) * FIT_MARGIN;
    session.dispatch(pinch(100.0)).expect("pinch");
    let scale = session.view().mapping(canvas, &VIEWPORT).scale();
    assert!((scale - row_fit * MAX_ZOOM).abs() < 1e-5, "{scale}");
    session.dispatch(pinch(1.0e-4)).expect("pinch");
    let scale = session.view().mapping(canvas, &VIEWPORT).scale();
    assert!((scale - row_fit * MIN_ZOOM).abs() < 1e-6, "{scale}");
    session
        .dispatch(Intent::View(ViewIntent::Fit))
        .expect("fit");
    assert_eq!(
        session.view(),
        CanvasView::fitting(Some(ROW)),
        "fit keeps it"
    );
}

#[test]
fn a_view_glides_to_a_rectangle_centred_elsewhere() {
    let canvas = CanvasSize::new(SHEET[0], SHEET[1]).expect("size");
    let from = CanvasView::new([30.0, -20.0], 1.5, 0.3).expect("view");
    let from = from.mapping(canvas, &VIEWPORT);
    let to = CanvasView::fitting(Some(ROW)).mapping(canvas, &VIEWPORT);
    let points = [[0.0, 0.0], [1460.0, 800.0], [2900.0, 3100.0]];
    for (t, map) in [(0.0, &from), (1.0, &to)] {
        let glide = from.toward(&to, t);
        for point in points {
            let [a, b] = [glide.to_screen(point), map.to_screen(point)];
            assert!(near(a, b), "at {t}, {point:?}: {a:?} against {b:?}");
        }
    }
    // The row's centre moves in a line from where the sheet's view showed it.
    let centre = [1460.0, 800.0];
    let half = from.toward(&to, 0.5).to_screen(centre);
    let [a, b] = [from.to_screen(centre), to.to_screen(centre)];
    assert!(near(half, [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]));
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
