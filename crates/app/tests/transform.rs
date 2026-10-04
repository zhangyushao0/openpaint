//! Transform intents: turning the tool on asks for the layer's pixels once; every placement is a
//! preview (never a history step) walked by undo and redo while the tool is on; turning it off
//! or another edit commits one step; an unchanged placement commits nothing; hidden, locked and
//! empty layers refuse with a notice; a selection's content is lifted inside it and leaves
//! nothing selected once committed.

use openpaint_app::{
    AppError, Availability, BrushLibrary, EditorAction, HistoryIntent, Intent, LayerIntent,
    LiftFailed, SelectionIntent, Session, Toolbox, TransformIntent, TransformRefusal,
};
use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, LayerCommand, Patch, Placement, RegionMask,
};

const SIZE: [u32; 2] = [40, 20];

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(canvas(), brush)
}

/// What the runtime would lift: 4 x 2 opaque pixels at (2, 3).
fn patch() -> Patch {
    Patch::new(canvas(), [2, 3], [4, 2], vec![255; 32]).expect("patch")
}

fn transform(session: &mut Session, intent: TransformIntent) {
    session
        .dispatch(Intent::Transform(intent))
        .expect("transform intent");
}

/// Turns the tool on and hands back the patch for the job it asks for.
fn begin(session: &mut Session) {
    transform(session, TransformIntent::Tool(true));
    assert!(session.transform_request().is_some(), "a job");
    assert!(session.transform_request().is_none(), "handed out once");
    session.transform_lifted(Ok(Some(patch())));
}

fn edits(artwork: &Artwork) -> usize {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.edits().len())
        .sum()
}

fn moved(x: f32) -> Placement {
    Placement::moved([x, 0.0]).expect("move")
}

fn drag(session: &mut Session, placement: Placement) {
    transform(session, TransformIntent::Place(placement));
    transform(session, TransformIntent::Settle);
}

fn available(session: &Session, action: EditorAction) -> bool {
    session.projection().availability(action) == Availability::Available
}

#[test]
fn placements_are_previews_walked_by_undo_and_turning_off_commits_one_step() {
    let mut session = session();
    begin(&mut session);
    let frame = session.projection().transform().frame.expect("framed");
    assert_eq!((frame.origin, frame.size), ([2.0, 3.0], [4.0, 2.0]));
    assert_eq!(edits(session.shown()), 1, "the preview shows");
    assert_eq!(edits(session.artwork()), 0, "but is not the artwork");
    assert!(
        !available(&session, EditorAction::Undo),
        "nothing placed yet"
    );
    let shown_patch = |s: &Session| {
        let edit = s.shown().layers()[0].edits()[0].transform().cloned();
        edit.expect("a transform").patch().is_some()
    };
    transform(&mut session, TransformIntent::Place(moved(5.0)));
    assert!(shown_patch(&session), "dragging shows the lifted pixels");
    transform(&mut session, TransformIntent::Settle);
    assert!(!shown_patch(&session), "settled, the record that commits");
    drag(&mut session, moved(9.0));
    let placement = |s: &Session| s.projection().transform().frame.expect("frame").placement;
    assert_eq!(placement(&session), moved(9.0));
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(
        placement(&session),
        moved(5.0),
        "undo walks the tool's steps"
    );
    session
        .dispatch(Intent::History(HistoryIntent::Redo))
        .expect("redo");
    assert_eq!(placement(&session), moved(9.0));
    assert_eq!(
        session.history().undo_steps(),
        0,
        "no history step while it is on"
    );
    transform(&mut session, TransformIntent::Tool(false));
    assert!(!session.transforming());
    assert_eq!(edits(session.artwork()), 1, "committed");
    let committed = session.artwork().layers()[0].edits()[0]
        .transform()
        .cloned();
    let committed = committed.expect("a transform");
    assert_eq!(committed.placement(), moved(9.0));
    assert!(committed.patch().is_none(), "a record, no pixels (S31)");
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(edits(session.artwork()), 0, "one undo takes it back");
}

#[test]
fn an_unchanged_or_reset_placement_commits_nothing() {
    let mut session = session();
    begin(&mut session);
    transform(&mut session, TransformIntent::Tool(false));
    assert_eq!(session.history().undo_steps(), 0);
    begin(&mut session);
    drag(&mut session, moved(4.0));
    assert!(session.projection().transform().placed);
    transform(&mut session, TransformIntent::Reset);
    assert!(!session.projection().transform().placed);
    transform(&mut session, TransformIntent::Tool(false));
    assert_eq!(edits(session.artwork()), 0);
}

#[test]
fn taps_turn_and_mirror_about_the_frames_centre() {
    let mut session = session();
    begin(&mut session);
    transform(&mut session, TransformIntent::Flip { horizontal: true });
    let frame = session.projection().transform().frame.expect("frame");
    assert_eq!(frame.centre(), [4.0, 4.0], "the centre stays");
    assert_eq!(frame.corners()[0], [6.0, 3.0], "left and right swap");
    transform(&mut session, TransformIntent::Rotate);
    let turned = session.projection().transform().frame.expect("frame");
    let [x, y] = turned.centre();
    assert!((x - 4.0).abs() < 1e-5 && (y - 4.0).abs() < 1e-5);
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(session.projection().transform().frame, Some(frame));
}

#[test]
fn another_edit_or_tool_commits_the_transform_first() {
    let mut session = session();
    begin(&mut session);
    drag(&mut session, moved(3.0));
    session
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("add a layer");
    assert!(!session.transforming());
    assert_eq!(
        edits(session.artwork()),
        1,
        "on the layer it was lifted from"
    );
    assert_eq!(session.artwork().layers().len(), 2);
    begin(&mut session);
    session
        .dispatch(Intent::Selection(SelectionIntent::Tool(true)))
        .expect("selection");
    assert!(!session.transforming(), "nothing placed: nothing committed");
}

#[test]
fn hidden_locked_and_empty_layers_refuse_with_a_notice() {
    let mut session = session();
    let refusal = |s: &Session| s.projection().transform().refused;
    transform(&mut session, TransformIntent::Tool(true));
    session.transform_lifted(Ok(None));
    assert!(!session.transforming());
    assert_eq!(refusal(&session), Some(TransformRefusal::Empty));
    transform(&mut session, TransformIntent::Tool(false));
    assert_eq!(refusal(&session), None, "OK closes the notice");
    transform(&mut session, TransformIntent::Tool(true));
    session.transform_lifted(Err(LiftFailed));
    assert_eq!(refusal(&session), Some(TransformRefusal::Failed));
    let layer = session.artwork().active();
    let hide = LayerIntent::SetVisible {
        layer,
        visible: false,
    };
    session.dispatch(Intent::Layer(hide)).expect("hide");
    let refused = session.dispatch(Intent::Transform(TransformIntent::Tool(true)));
    assert_eq!(
        refused,
        Err(AppError::TransformRefused(TransformRefusal::Hidden))
    );
    assert!(session.transform_request().is_none());
    let show = ArtworkCommand::Layer(LayerCommand::SetVisible {
        layer,
        visible: true,
    });
    let lock = ArtworkCommand::Layer(LayerCommand::SetLocked {
        layer,
        locked: true,
    });
    let shown = session.artwork().apply(show).expect("show").artwork;
    let locked = shown.apply(lock).expect("lock").artwork;
    let mut session = Session::open(locked, session_brush());
    let refused = session.dispatch(Intent::Transform(TransformIntent::Tool(true)));
    assert_eq!(
        refused,
        Err(AppError::TransformRefused(TransformRefusal::Locked))
    );
}

fn session_brush() -> openpaint_core::StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    Toolbox::new(library).style().expect("style")
}

#[test]
fn a_selections_content_is_lifted_inside_it_and_leaves_nothing_selected() {
    let mut session = session();
    let corners = [[1.0, 1.0], [12.0, 1.0], [12.0, 9.0], [1.0, 9.0]];
    let clip = RegionMask::polygon(canvas(), &corners).expect("a rectangle");
    for intent in [
        SelectionIntent::Tool(true),
        SelectionIntent::Shape(openpaint_app::SelectionShape::Rectangle),
    ] {
        session
            .dispatch(Intent::Selection(intent))
            .expect("selection");
    }
    draw_rectangle(&mut session, [1.0, 1.0], [12.0, 9.0]);
    assert_eq!(session.selection(), Some(&clip));
    transform(&mut session, TransformIntent::Tool(true));
    assert!(
        !session.projection().selection().tool,
        "the Selection tool goes"
    );
    let job = session.transform_request().expect("a job");
    assert_eq!(job.clip.as_ref(), Some(&clip));
    session.transform_lifted(Ok(Some(patch())));
    assert!(
        session.selection().is_none(),
        "lifted: not marked while it moves"
    );
    drag(&mut session, moved(6.0));
    transform(&mut session, TransformIntent::Tool(false));
    let edit = session.artwork().layers()[0].edits()[0]
        .transform()
        .cloned();
    assert_eq!(edit.expect("a transform").lift(), Some(&clip));
    assert!(session.selection().is_none(), "nothing selected after");
}

/// Drags the rectangle selection from `a` to `b` with the pen.
fn draw_rectangle(session: &mut Session, a: [f32; 2], b: [f32; 2]) {
    use openpaint_app::{Contact, ContactPhase, Device, InputSample};
    let contact = |phase, at: [f32; 2], time_ms| Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![InputSample {
            position: at,
            pressure: 0.5,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms,
        }],
    };
    for (phase, at, time) in [
        (ContactPhase::Down, a, 0),
        (ContactPhase::Move, b, 8),
        (ContactPhase::Up, b, 16),
    ] {
        let intent = SelectionIntent::Contact {
            contact: contact(phase, at, time),
            close: 2.0,
        };
        session
            .dispatch(Intent::Selection(intent))
            .expect("selection contact");
    }
}
