//! What a mode (D112) sets and does on the session it owns, which holds no mode: bounds that a
//! stroke stays inside and a fill is found within, strokes kept raw, a version amended (no step)
//! or recorded (one), the history cut, the view moved, and what an intent ends made way for
//! once however often it is asked.

use std::sync::Arc;

use openpaint_app::{
    AdjustIntent, BrushLibrary, CanvasView, Contact, ContactPhase, Device, FillIntent,
    HistoryIntent, InputSample, Intent, LayerIntent, Session, StrokeIntent, Toolbox,
};
use openpaint_core::{AdjustmentKind, Artwork, CanvasSize, Color, RegionMask, StrokeStyle};

fn brush() -> StrokeStyle {
    Toolbox::new(BrushLibrary::built_in().expect("brushes"))
        .style()
        .expect("style")
}

/// A session of 200 x 100 kept to its left half, as a mode keeps it, drawing with a pen of 12.
fn framed() -> Session {
    let canvas = CanvasSize::new(200, 100).expect("size");
    let left = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let brush = brush();
    let pen = StrokeStyle::new(Arc::clone(brush.recipe()), 12.0, 1.0, Color::BLACK).expect("pen");
    let mut session = Session::open(Artwork::new(canvas), brush);
    session.keep_raw();
    session.set_bounds(RegionMask::polygon(canvas, &left));
    session.set_brush(pen);
    session
}

fn contact(phase: ContactPhase, at: [f32; 2], time_ms: i64) -> Intent {
    let sample = InputSample {
        position: at,
        pressure: 0.7,
        tilt: 0.0,
        azimuth: 0.0,
        time_ms,
    };
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![sample],
    }))
}

fn stroke(session: &mut Session, x: f32) {
    for (phase, dx, time_ms) in [
        (ContactPhase::Down, 0.0, 0),
        (ContactPhase::Move, 20.0, 8),
        (ContactPhase::Up, 40.0, 16),
    ] {
        let intent = contact(phase, [x + dx, 50.0], time_ms);
        session.dispatch(intent).expect("dispatch");
    }
}

#[test]
fn a_stroke_stays_inside_the_bounds_with_the_pen_set_and_its_own_samples() {
    let mut session = framed();
    stroke(&mut session, 20.0);
    let drawn = (session.artwork().layers().iter()).find_map(|l| l.strokes().last());
    let drawn = drawn.expect("the stroke");
    assert_eq!(drawn.style().size(), 12.0, "the pen set, not the brush");
    assert!(drawn.clip().is_some(), "kept inside the bounds");
    let xs: Vec<f32> = (drawn.samples().iter()).map(|s| s.position().x()).collect();
    assert_eq!(xs, [20.0, 40.0, 60.0], "the pen's own samples");
    assert_eq!(
        session.projection().pen(),
        None,
        "a mode fills it in, not the session"
    );
    let pen = session.projection().with_pen(Some(12.0)).pen();
    assert_eq!(pen, Some(12.0), "its pen's outline");
}

#[test]
fn a_fill_is_found_within_the_bounds_box_and_cut_to_them() {
    let mut session = framed();
    let drop = |point| {
        Intent::Fill(FillIntent::Request {
            point,
            threshold: 0.5,
            color: Color::BLACK,
        })
    };
    session.dispatch(drop([10.0, 20.0])).expect("inside");
    let job = session.fill_request().expect("a job");
    // The worker floods only this box, so a line across the frame keeps the flood on its side.
    assert_eq!(job.within, Some(([0, 0], [100, 100])));
    let canvas = session.artwork().canvas();
    session.fill_finished(job.sequence, Ok(RegionMask::whole(canvas)));
    session
        .dispatch(Intent::Fill(FillIntent::Commit))
        .expect("commit");
    let layer = (session.artwork().layers().iter()).find(|layer| !layer.edits().is_empty());
    let edit = &layer.expect("the filled layer").edits()[0];
    let fill = edit.fill().expect("a fill").mask();
    assert_eq!((fill.origin(), fill.size()), ([0, 0], [100, 100]), "cut");
    session.set_bounds(None);
    session.dispatch(drop([150.0, 20.0])).expect("no bounds");
    let job = session.fill_request().expect("a job");
    assert_eq!(job.within, None, "without bounds the canvas is flooded");
}

#[test]
fn a_version_amended_is_no_step_one_recorded_is_and_the_history_cut_reaches_none() {
    let mut session = framed();
    let mut layered = Session::open(session.artwork().clone(), brush());
    layered
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("add");
    session.amend(layered.artwork().clone());
    assert_eq!(session.artwork().layers().len(), 2, "amended");
    assert!(!session.history().can_undo(), "no step");
    stroke(&mut session, 20.0);
    let mut another = Session::open(session.artwork().clone(), brush());
    another
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("add");
    session.record(another.artwork().clone());
    assert_eq!(
        session.history().undo_steps(),
        2,
        "the stroke and the version recorded"
    );
    let moved = CanvasView::new([12.0, 0.0], 2.0, 0.0).expect("a view");
    *session.view_mut() = moved;
    assert_eq!(session.view(), moved, "the view moved, no step");
    session.keep_only_current();
    assert!(
        !session.history().can_undo(),
        "nothing before the version kept"
    );
    let undo = session.dispatch(Intent::History(HistoryIntent::Undo));
    assert!(undo.is_err());
    let layers = session.artwork().layers();
    let strokes: usize = layers.iter().map(|layer| layer.strokes().len()).sum();
    assert_eq!((layers.len(), strokes), (3, 1), "the version kept");
}

#[test]
fn nothing_is_amended_while_a_stroke_is_drawn() {
    let mut session = framed();
    let down = contact(ContactPhase::Down, [20.0, 50.0], 0);
    session.dispatch(down).expect("down");
    let mut layered = Session::open(session.artwork().clone(), brush());
    layered
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("add");
    session.amend(layered.artwork().clone());
    assert_eq!(
        session.artwork().layers().len(),
        1,
        "kept as the stroke began"
    );
}

#[test]
fn what_an_intent_ends_is_made_way_for_once_however_often_it_is_asked() {
    let mut session = framed();
    let curves = Intent::Adjust(AdjustIntent::Begin(AdjustmentKind::Curves));
    session.dispatch(curves).expect("an adjustment under way");
    assert!(session.adjusting(), "under way");
    let landing = contact(ContactPhase::Down, [20.0, 50.0], 0);
    session.make_way(&landing);
    assert!(!session.adjusting(), "kept as it shows before the stroke");
    let steps = session.history().undo_steps();
    session.make_way(&landing);
    session.make_way_for_step();
    assert_eq!(session.history().undo_steps(), steps, "nothing more ended");
    session.dispatch(landing).expect("the stroke begins");
    assert!(session.active().is_some());
}
