//! Insert image… (S26): choosing asks the platform for one image and remembers the canvas it
//! is for; a cancelled picker changes nothing; a failure is a notice; the picture read is taken
//! once. In the session the picture becomes a new layer above the active one (in its group),
//! framed by the Transform tool; the layer, the picture and its placement commit as one step,
//! even unmoved; undo at the first step takes both away; undo after the commit too.

use openpaint_app::{
    AppError, Availability, BrushLibrary, EditorAction, HistoryIntent, HostAnswer, HostRequest,
    ImageError, ImageSource, InsertIntent, Intent, LayerIntent, PortError, ReferenceNotice,
    Session, Toolbox, TransferJob, Transfers, TransformIntent, TransformRefusal,
};
use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, LayerCommand, MAX_LAYERS, Patch, Placement,
};

const SIZE: [u32; 2] = [40, 20];

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

struct Source;

impl ImageSource for Source {
    fn read(self: Box<Self>, _limit: u64) -> Result<Vec<u8>, PortError> {
        Ok(Vec::new())
    }
}

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(canvas(), brush)
}

/// A 6 x 4 picture centred on the canvas, as `open_picture` places it.
fn picture() -> Patch {
    Patch::new(canvas(), [17, 8], [6, 4], vec![128; 96]).expect("patch")
}

fn insert(session: &mut Session) {
    session
        .dispatch(Intent::Transform(TransformIntent::Insert(picture())))
        .expect("insert");
}

fn names(artwork: &Artwork) -> Vec<String> {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.name().as_str().to_owned())
        .collect()
}

fn history(session: &mut Session, step: HistoryIntent) {
    session.dispatch(Intent::History(step)).expect("history");
}

#[test]
fn choosing_asks_for_an_image_and_hands_its_canvas_to_the_worker() {
    let mut transfers = Transfers::default();
    let refused = transfers.insert(InsertIntent::Choose, None);
    assert!(refused.is_err(), "no artwork open");
    transfers
        .insert(InsertIntent::Choose, Some(canvas()))
        .expect("choose");
    assert_eq!(transfers.request(), Some(HostRequest::OpenImage));
    assert!(transfers.waiting());
    assert_eq!(transfers.projection().available, Availability::Running);
    assert_eq!(
        transfers.projection().insert_notice,
        ReferenceNotice::Opening
    );
    let job = transfers.answer(HostAnswer::Image(Box::new(Source)));
    let Some(TransferJob::Insert(job)) = job else {
        panic!("an insert job: {job:?}");
    };
    assert_eq!(job.canvas, canvas());
    assert_eq!(transfers.take_picture(), None, "nothing read yet");
    transfers.inserted(Ok(picture()));
    assert_eq!(transfers.take_picture(), Some(picture()));
    assert_eq!(transfers.take_picture(), None, "taken once");
    assert_eq!(transfers.projection().insert_notice, ReferenceNotice::None);
    assert_eq!(transfers.projection().available, Availability::Available);
}

#[test]
fn a_cancelled_picker_changes_nothing_and_a_failure_is_a_notice() {
    let mut transfers = Transfers::default();
    transfers
        .insert(InsertIntent::Choose, Some(canvas()))
        .expect("choose");
    assert!(transfers.answer(HostAnswer::Cancelled).is_none());
    assert_eq!(transfers.projection().insert_notice, ReferenceNotice::None);
    assert_eq!(
        transfers.projection().reference_notice,
        ReferenceNotice::None
    );
    transfers
        .insert(InsertIntent::Choose, Some(canvas()))
        .expect("choose again");
    assert!(
        transfers
            .answer(HostAnswer::Image(Box::new(Source)))
            .is_some()
    );
    transfers.inserted(Err(ImageError::Unsupported));
    assert_eq!(
        transfers.projection().insert_notice,
        ReferenceNotice::Failed(ImageError::Unsupported)
    );
    assert_eq!(transfers.take_picture(), None);
    transfers
        .insert(InsertIntent::Dismiss, None)
        .expect("dismiss");
    assert_eq!(transfers.projection().insert_notice, ReferenceNotice::None);
}

#[test]
fn the_picture_is_a_new_layer_framed_at_once_and_one_step_once_committed() {
    let mut session = session();
    insert(&mut session);
    assert!(session.transforming(), "the Transform tool is on");
    let frame = session.projection().transform().frame.expect("framed");
    assert_eq!((frame.origin, frame.size), ([17.0, 8.0], [6.0, 4.0]));
    assert_eq!(names(session.shown()), ["Layer 1", "Inserted image"]);
    assert_eq!(names(session.artwork()), ["Layer 1"], "not committed yet");
    assert!(session.transform_request().is_none(), "nothing to lift");
    // Committed unmoved: still one step, the picture in its place.
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(false)))
        .expect("commit");
    assert!(!session.transforming());
    assert_eq!(session.history().undo_steps(), 1);
    let layer = &session.artwork().layers()[1];
    assert_eq!(session.artwork().active(), layer.id());
    let edit = layer.edits()[0].transform().expect("a placed patch");
    assert_eq!((edit.lift(), edit.placement()), (None, Placement::IDENTITY));
    assert_eq!(edit.patch(), &picture());
    history(&mut session, HistoryIntent::Undo);
    assert_eq!(
        names(session.artwork()),
        ["Layer 1"],
        "one undo removes both"
    );
    history(&mut session, HistoryIntent::Redo);
    assert_eq!(names(session.artwork()), ["Layer 1", "Inserted image"]);
}

#[test]
fn moves_are_steps_of_the_tool_and_undo_at_the_first_takes_the_picture_away() {
    let mut session = session();
    insert(&mut session);
    let moved = Placement::moved([3.0, 1.0]).expect("move");
    session
        .dispatch(Intent::Transform(TransformIntent::Place(moved)))
        .expect("place");
    session
        .dispatch(Intent::Transform(TransformIntent::Settle))
        .expect("settle");
    history(&mut session, HistoryIntent::Undo);
    assert!(session.transforming(), "back to where it was inserted");
    assert_eq!(
        session.projection().availability(EditorAction::Undo),
        Availability::Available,
        "one more undo removes it"
    );
    history(&mut session, HistoryIntent::Undo);
    assert!(!session.transforming());
    assert_eq!(names(session.shown()), ["Layer 1"]);
    assert_eq!(session.history().undo_steps(), 0, "nothing recorded");
}

#[test]
fn the_layer_joins_the_active_group_and_another_edit_commits_it() {
    let mut session = session();
    session
        .dispatch(Intent::Layer(LayerIntent::NewGroup))
        .expect("group");
    let group = session.artwork().layers()[1].props().group();
    assert!(group.is_some());
    insert(&mut session);
    session
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("another layer commits the picture");
    let layers = session.artwork().layers();
    assert_eq!(layers[2].name().as_str(), "Inserted image");
    assert_eq!(
        layers[2].props().group(),
        group,
        "in the active layer's group"
    );
    // A second picture commits a transform in progress first.
    insert(&mut session);
    insert(&mut session);
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(false)))
        .expect("commit");
    assert_eq!(session.artwork().layers().len(), 6);
}

#[test]
fn no_room_for_another_layer_refuses_with_a_notice() {
    let mut artwork = Artwork::new(canvas());
    for _ in 1..MAX_LAYERS {
        let create = ArtworkCommand::Layer(LayerCommand::Create);
        artwork = artwork.apply(create).expect("create").artwork;
    }
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let mut session = Session::open(artwork, Toolbox::new(library).style().expect("style"));
    let refused = session.dispatch(Intent::Transform(TransformIntent::Insert(picture())));
    assert_eq!(
        refused,
        Err(AppError::TransformRefused(TransformRefusal::Full))
    );
    assert_eq!(
        session.projection().transform().refused,
        Some(TransformRefusal::Full)
    );
    assert!(!session.transforming());
}

#[test]
fn leaving_the_app_commits_the_picture_so_it_is_stored() {
    let mut session = session();
    insert(&mut session);
    let version = session.leave_request().expect("a version to store");
    assert_eq!(names(&version), ["Layer 1", "Inserted image"]);
    assert!(!session.transforming());
}
