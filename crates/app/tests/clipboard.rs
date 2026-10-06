//! Copy and paste in the session (S40): which commands the menu offers, Copy holding records
//! without a history step, Cut as one undo step, Paste framing a new layer with the Transform
//! tool that commits as one step (moved or not), Duplicate and Cut & Paste as one step each,
//! undo at the first transform step taking a paste back, and a paste of nothing refused.

use std::sync::Arc;

use openpaint_app::{
    AppError, Availability, BrushLibrary, Clipboard, ClipboardAction, EditorAction, HistoryIntent,
    Intent, Session, Toolbox, TransformIntent, TransformRefusal,
};
use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, LayerCommand, Patch,
    Placement, StrokeSample, StrokeStyle,
};

fn canvas() -> CanvasSize {
    CanvasSize::new(40, 20).expect("size")
}

/// A canvas with one stroke on Layer 1.
fn drawn() -> Artwork {
    let style =
        StrokeStyle::new(Arc::new(BrushRecipe::plain()), 4.0, 1.0, Color::BLACK).expect("style");
    let point = |x| CanvasPoint::new(x, 10.0).expect("point");
    let first = StrokeSample::new(point(4.0), 1.0, 0.0, 0.0, 0).expect("sample");
    let mut session = Artwork::new(canvas())
        .begin_stroke(style, first)
        .expect("begin");
    let last = StrokeSample::new(point(30.0), 1.0, 0.0, 0.0, 30).expect("sample");
    session.extend(&[last]).expect("extend");
    session.finish().artwork
}

fn session(artwork: Artwork) -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::open(artwork, brush)
}

fn offered(session: &Session, clipboard: &Clipboard, action: ClipboardAction) -> Availability {
    let projection = session.projection().with_clipboard(clipboard.canvas());
    projection.availability(EditorAction::Clipboard(action))
}

/// The runtime's part: the pasted layer's painted pixels lifted (here, a stand-in patch).
fn lift(session: &mut Session, painted: bool) {
    let job = session.transform_request().expect("a lift");
    assert_eq!(job.layer, job.artwork.active(), "the pasted layer");
    let patch = Patch::new(canvas(), [2, 6], [30, 8], vec![200; 30 * 8 * 4]).expect("patch");
    session.transform_lifted(Ok(painted.then_some(patch)));
}

fn step(session: &mut Session, step: HistoryIntent) {
    session.dispatch(Intent::History(step)).expect("history");
}

fn layer_names(artwork: &Artwork) -> Vec<&str> {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.name().as_str())
        .collect()
}

#[test]
fn the_menu_offers_what_there_is_something_for() {
    let mut clipboard = Clipboard::default();
    let mut session = session(drawn());
    for action in [
        ClipboardAction::Cut,
        ClipboardAction::Copy,
        ClipboardAction::CopyAll,
    ] {
        assert_eq!(
            offered(&session, &clipboard, action),
            Availability::Available
        );
    }
    assert_eq!(
        offered(&session, &clipboard, ClipboardAction::Paste),
        Availability::NothingLeft
    );
    session
        .clipboard(ClipboardAction::Copy, &mut clipboard)
        .expect("copy");
    assert_eq!(
        offered(&session, &clipboard, ClipboardAction::Paste),
        Availability::Available
    );
    let other = Session::open(Artwork::new(CanvasSize::new(41, 20).expect("size")), {
        Toolbox::new(BrushLibrary::built_in().expect("brushes"))
            .style()
            .expect("style")
    });
    assert_eq!(
        offered(&other, &clipboard, ClipboardAction::Paste),
        Availability::NothingLeft
    );
    assert_eq!(
        offered(&other, &clipboard, ClipboardAction::Copy),
        Availability::NothingLeft
    );
    // A hidden layer can be copied but not cut.
    let id = session.artwork().active();
    let hide = LayerCommand::SetVisible {
        layer: id,
        visible: false,
    };
    let hidden = session
        .artwork()
        .apply(ArtworkCommand::Layer(hide))
        .expect("hide")
        .artwork;
    let hidden = self::session(hidden);
    assert_eq!(
        offered(&hidden, &clipboard, ClipboardAction::Cut),
        Availability::NothingLeft
    );
    assert_eq!(
        offered(&hidden, &clipboard, ClipboardAction::Copy),
        Availability::Available
    );
    let refused = hidden
        .projection()
        .availability(EditorAction::Clipboard(ClipboardAction::CutPaste));
    assert_eq!(refused, Availability::NothingLeft);
}

#[test]
fn copy_holds_records_and_cut_is_one_undo_step() {
    let mut clipboard = Clipboard::default();
    let start = drawn();
    let mut session = session(start.clone());
    session
        .clipboard(ClipboardAction::Copy, &mut clipboard)
        .expect("copy");
    assert!(
        session.artwork().same_version(&start),
        "copying changes nothing"
    );
    assert_eq!(clipboard.held().map(|held| held.edits().len()), Some(1));
    session
        .clipboard(ClipboardAction::Cut, &mut clipboard)
        .expect("cut");
    let cut = session.artwork().clone();
    assert!(
        cut.layers()[0].strokes().is_empty(),
        "nothing selected: the layer emptied"
    );
    step(&mut session, HistoryIntent::Undo);
    assert!(session.artwork().same_version(&start));
    step(&mut session, HistoryIntent::Redo);
    assert!(session.artwork().same_version(&cut));
    let again = session.clipboard(ClipboardAction::Cut, &mut clipboard);
    assert!(
        matches!(again, Err(AppError::Unavailable { .. })),
        "nothing left to cut"
    );
}

#[test]
fn paste_frames_a_new_layer_and_commits_it_as_one_step() {
    let mut clipboard = Clipboard::default();
    let start = drawn();
    let mut session = session(start.clone());
    session
        .clipboard(ClipboardAction::Copy, &mut clipboard)
        .expect("copy");
    session
        .clipboard(ClipboardAction::Paste, &mut clipboard)
        .expect("paste");
    assert!(session.transforming(), "the Transform tool is on");
    lift(&mut session, true);
    assert_eq!(layer_names(session.shown()), ["Layer 1", "Pasted layer"]);
    assert!(
        session.artwork().same_version(&start),
        "nothing in history yet"
    );
    // Undo at the first step takes the paste back.
    step(&mut session, HistoryIntent::Undo);
    assert!(!session.transforming());
    assert_eq!(layer_names(session.shown()), ["Layer 1"]);
    // Pasted again, moved and committed: one step, the content a record placed by a record.
    session
        .clipboard(ClipboardAction::Paste, &mut clipboard)
        .expect("paste");
    lift(&mut session, true);
    let moved = Placement::moved([5.0, 2.0]).expect("move");
    session
        .dispatch(Intent::Transform(TransformIntent::Place(moved)))
        .expect("place");
    session
        .dispatch(Intent::Transform(TransformIntent::Settle))
        .expect("settle");
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(false)))
        .expect("commit");
    let placed = session.artwork().clone();
    let pasted = &placed.layers()[1];
    assert!(
        pasted.edits()[0].nested().is_some(),
        "the copy as a nested record"
    );
    let record = pasted.edits()[1].transform().expect("a transform");
    assert!(
        record.patch().is_none() && record.lift().is_none(),
        "a whole-layer record"
    );
    step(&mut session, HistoryIntent::Undo);
    assert!(session.artwork().same_version(&start), "one step");
    step(&mut session, HistoryIntent::Redo);
    assert!(session.artwork().same_version(&placed));
}

#[test]
fn an_unmoved_paste_commits_the_pasted_layer_alone_with_no_record() {
    let mut clipboard = Clipboard::default();
    let mut session = session(drawn());
    session
        .clipboard(ClipboardAction::Copy, &mut clipboard)
        .expect("copy");
    session
        .clipboard(ClipboardAction::Paste, &mut clipboard)
        .expect("paste");
    lift(&mut session, true);
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(false)))
        .expect("commit");
    let top = session
        .artwork()
        .layers()
        .last()
        .expect("layer")
        .edits()
        .len();
    assert_eq!(top, 1);
}

#[test]
fn duplicate_and_cut_and_paste_are_one_step_each() {
    let mut clipboard = Clipboard::default();
    let start = drawn();
    let mut session = session(start.clone());
    session
        .clipboard(ClipboardAction::Duplicate, &mut clipboard)
        .expect("duplicate");
    lift(&mut session, true);
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(false)))
        .expect("commit");
    assert_eq!(layer_names(session.artwork()), ["Layer 1", "Pasted layer"]);
    assert!(clipboard.held().is_some(), "Duplicate copies too");
    step(&mut session, HistoryIntent::Undo);
    assert!(session.artwork().same_version(&start));
    session
        .clipboard(ClipboardAction::CutPaste, &mut clipboard)
        .expect("cut and paste");
    lift(&mut session, true);
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(false)))
        .expect("commit");
    let after = session.artwork().clone();
    assert!(after.layers()[0].strokes().is_empty(), "cut");
    assert_eq!(after.layers().len(), 2, "and pasted");
    step(&mut session, HistoryIntent::Undo);
    assert!(session.artwork().same_version(&start), "both in one step");
}

#[test]
fn a_paste_of_nothing_is_refused_and_records_nothing() {
    let mut clipboard = Clipboard::default();
    let start = drawn();
    let mut session = session(start.clone());
    session
        .clipboard(ClipboardAction::Copy, &mut clipboard)
        .expect("copy");
    session
        .clipboard(ClipboardAction::Paste, &mut clipboard)
        .expect("paste");
    lift(&mut session, false);
    assert!(!session.transforming());
    let refused = session.projection().transform().refused;
    assert_eq!(refused, Some(TransformRefusal::NothingPasted));
    assert!(session.artwork().same_version(&start));
    assert_eq!(layer_names(session.shown()), ["Layer 1"]);
}
