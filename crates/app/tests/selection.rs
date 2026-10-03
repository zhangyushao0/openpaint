//! The Selection tool: the pen draws a rectangle or a freehand path (dragged or tapped, closed
//! on its first point or by leaving the tool), shapes add up, Invert and Deselect; undo walks
//! the selection's steps while the tool is on and the artwork's history otherwise; strokes and
//! fills made while a selection exists keep inside it; nothing of the selection enters the
//! artwork or its history.

use openpaint_app::{
    AppError, Availability, BrushLibrary, Contact, ContactPhase, Device, EditorAction, FillIntent,
    FillRefusal, HistoryIntent, InputSample, Intent, SelectionIntent, SelectionShape, Session,
    StrokeIntent, Toolbox,
};
use openpaint_core::{CanvasSize, Color, RegionMask};

const SIZE: [u32; 2] = [100, 60];
/// How near the first point a freehand path closes, in canvas pixels.
const CLOSE: f32 = 6.0;

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"), brush)
}

fn contact(phase: ContactPhase, points: &[[f32; 2]]) -> Contact {
    let samples = points
        .iter()
        .enumerate()
        .map(|(i, &position)| InputSample {
            position,
            pressure: 0.5,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms: i as i64 * 8,
        })
        .collect();
    Contact {
        id: 3,
        device: Device::Pen,
        phase,
        samples,
    }
}

fn select(session: &mut Session, intent: SelectionIntent) {
    session
        .dispatch(Intent::Selection(intent))
        .expect("selection intent");
}

/// The pen touching down at the first point, moving through the rest and lifting at the last.
fn drag(session: &mut Session, points: &[[f32; 2]]) {
    let (first, rest) = points.split_first().expect("points");
    let (last, middle) = rest.split_last().expect("two points");
    let mut phases = vec![(ContactPhase::Down, std::slice::from_ref(first))];
    if !middle.is_empty() {
        phases.push((ContactPhase::Move, middle));
    }
    phases.push((ContactPhase::Up, std::slice::from_ref(last)));
    for (phase, at) in phases {
        let contact = contact(phase, at);
        select(
            session,
            SelectionIntent::Contact {
                contact,
                close: CLOSE,
            },
        );
    }
}

fn tap(session: &mut Session, at: [f32; 2]) {
    drag(session, &[at, at]);
}

fn mask(session: &Session) -> Option<RegionMask> {
    session.projection().selection().mask.clone()
}

fn undo(session: &mut Session) {
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
}

#[test]
fn a_dragged_rectangle_selects_and_never_enters_the_artwork_or_its_history() {
    let mut session = session();
    let blank = session.artwork().clone();
    select(&mut session, SelectionIntent::Tool(true));
    select(
        &mut session,
        SelectionIntent::Shape(SelectionShape::Rectangle),
    );
    drag(&mut session, &[[10.0, 10.0], [30.0, 20.0], [40.0, 30.0]]);
    let selected = mask(&session).expect("a rectangle");
    assert_eq!((selected.origin(), selected.size()), ([10, 10], [30, 20]));
    assert!(session.artwork().same_version(&blank), "no artwork change");
    assert_eq!(session.history().undo_steps(), 0, "no history step");
    // Off the tool, the selection stays and undo is the artwork's again: nothing to undo.
    select(&mut session, SelectionIntent::Tool(false));
    assert_eq!(mask(&session), Some(selected));
    assert_eq!(
        session.projection().availability(EditorAction::Undo),
        Availability::NothingLeft
    );
}

#[test]
fn a_freehand_path_of_taps_closes_on_its_first_point_and_shapes_add_up() {
    let mut session = session();
    select(&mut session, SelectionIntent::Tool(true));
    for at in [[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]] {
        tap(&mut session, at);
    }
    assert!(session.projection().selection().closable);
    assert_eq!(session.projection().selection().path.len(), 4);
    assert_eq!(mask(&session), None, "still open");
    tap(&mut session, [12.0, 11.0]);
    let square = mask(&session).expect("closed on its first point");
    assert_eq!((square.origin(), square.size()), ([10, 10], [40, 40]));
    assert!(session.projection().selection().path.is_empty());
    // A dragged lasso to the right, lifted on its start, adds to it.
    drag(
        &mut session,
        &[
            [70.0, 5.0],
            [90.0, 5.0],
            [90.0, 25.0],
            [70.0, 25.0],
            [71.0, 6.0],
        ],
    );
    let both = mask(&session).expect("both");
    assert!(both.contains([20, 20]) && both.contains([80, 15]) && !both.contains([60, 15]));
    // Undo takes the lasso back, then the square; redo brings the square again.
    undo(&mut session);
    assert_eq!(mask(&session), Some(square.clone()));
    undo(&mut session);
    assert_eq!(mask(&session), None);
    session
        .dispatch(Intent::History(HistoryIntent::Redo))
        .expect("redo");
    assert_eq!(mask(&session), Some(square));
}

#[test]
fn leaving_the_tool_closes_an_open_path_and_undo_drops_an_open_one() {
    let mut session = session();
    select(&mut session, SelectionIntent::Tool(true));
    drag(&mut session, &[[10.0, 10.0], [40.0, 10.0], [40.0, 40.0]]);
    assert_eq!(mask(&session), None);
    undo(&mut session);
    assert!(session.projection().selection().path.is_empty(), "dropped");
    drag(&mut session, &[[10.0, 10.0], [40.0, 10.0], [40.0, 40.0]]);
    select(&mut session, SelectionIntent::Tool(false));
    let triangle = mask(&session).expect("closed by leaving the tool");
    assert!(triangle.contains([35, 15]) && !triangle.contains([15, 35]));
}

#[test]
fn invert_and_deselect() {
    let mut session = session();
    select(&mut session, SelectionIntent::Tool(true));
    select(&mut session, SelectionIntent::Invert);
    let all = mask(&session).expect("everything");
    assert_eq!((all.origin(), all.size()), ([0, 0], SIZE));
    select(
        &mut session,
        SelectionIntent::Shape(SelectionShape::Rectangle),
    );
    select(&mut session, SelectionIntent::Deselect);
    drag(&mut session, &[[0.0, 0.0], [50.0, 60.0]]);
    select(&mut session, SelectionIntent::Invert);
    let right = mask(&session).expect("the right half");
    assert_eq!((right.origin(), right.size()), ([50, 0], [50, 60]));
    select(&mut session, SelectionIntent::Deselect);
    assert_eq!(mask(&session), None);
}

fn stroke(session: &mut Session, from: [f32; 2], to: [f32; 2]) {
    for (phase, at) in [
        (ContactPhase::Down, from),
        (ContactPhase::Move, to),
        (ContactPhase::Up, to),
    ] {
        let contact = contact(phase, &[at]);
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("stroke");
    }
}

#[test]
fn strokes_and_fills_keep_inside_the_selection_and_keep_their_clip_after_deselect() {
    let mut session = session();
    select(&mut session, SelectionIntent::Tool(true));
    select(
        &mut session,
        SelectionIntent::Shape(SelectionShape::Rectangle),
    );
    drag(&mut session, &[[20.0, 10.0], [60.0, 50.0]]);
    let selected = mask(&session).expect("a rectangle");
    select(&mut session, SelectionIntent::Tool(false));
    stroke(&mut session, [5.0, 30.0], [95.0, 30.0]);
    let layer = &session.artwork().layers()[0];
    let drawn = layer.strokes().last().expect("a stroke");
    assert!(drawn.clip().expect("clipped").shares(&selected));
    // A drop outside is refused; one inside fills the flood inside the selection.
    let request = |point| {
        Intent::Fill(FillIntent::Request {
            point,
            threshold: 0.5,
            color: Color::BLACK,
        })
    };
    assert_eq!(
        session.dispatch(request([5.0, 5.0])),
        Err(AppError::FillRefused(FillRefusal::OutsideSelection))
    );
    session.dispatch(request([30.0, 20.0])).expect("inside");
    let job = session.fill_request().expect("a job");
    session.fill_finished(
        job.sequence,
        Ok(RegionMask::whole(session.artwork().canvas())),
    );
    session
        .dispatch(Intent::Fill(FillIntent::Commit))
        .expect("commit");
    let edit = &session.artwork().layers()[0].edits()[0];
    let fill = edit.fill().expect("a fill").mask().clone();
    assert_eq!(fill, selected, "the whole flood, cut to the selection");
    select(&mut session, SelectionIntent::Deselect);
    stroke(&mut session, [5.0, 30.0], [95.0, 30.0]);
    let strokes: Vec<_> = session.artwork().layers()[0]
        .strokes()
        .iter()
        .cloned()
        .collect();
    assert!(strokes[0].clip().is_some(), "the first keeps its clip");
    assert!(strokes[1].clip().is_none(), "after deselect, no clip");
    // Undo outside the tool walks the artwork: the stroke, the fill; the selection stays gone.
    undo(&mut session);
    undo(&mut session);
    assert_eq!(session.artwork().layers()[0].edits().len(), 0);
    assert_eq!(mask(&session), None);
}
