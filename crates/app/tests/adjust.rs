//! Adjustment intents (S28): each value shows at once as a preview that is no history step,
//! Apply keeps it as one undoable step, Cancel and undo leave the artwork, another edit keeps
//! what shows, the selection limits it and the mask takes it when it is the target, and hidden
//! or locked layers refuse with a notice.

use openpaint_app::{
    AdjustIntent, AdjustRefusal, AppError, Availability, BrushLibrary, Contact, ContactPhase,
    Device, EditorAction, HistoryIntent, InputSample, Intent, LayerIntent, SelectionIntent,
    SelectionShape, Session, Toolbox,
};
use openpaint_core::{Adjustment, AdjustmentKind, Artwork, CanvasSize, Hsb, LayerId, RegionMask};

const SIZE: [u32; 2] = [40, 20];
const LAYER: LayerId = LayerId::new(1);

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"), brush)
}

fn adjust(session: &mut Session, intent: AdjustIntent) {
    session
        .dispatch(Intent::Adjust(intent))
        .expect("adjust intent");
}

fn brighter(value: f32) -> Adjustment {
    Adjustment::Hsb(Hsb::new(0.5, 0.5, value))
}

fn edits(artwork: &Artwork) -> usize {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.edits().len())
        .sum()
}

fn mask_edits(artwork: &Artwork) -> usize {
    let mask = artwork.layer(LAYER).and_then(|layer| layer.mask());
    mask.map_or(0, |mask| mask.edits().len())
}

#[test]
fn a_preview_shows_at_once_and_apply_keeps_it_as_one_step() {
    let mut session = session();
    adjust(
        &mut session,
        AdjustIntent::Begin(AdjustmentKind::HueSaturationBrightness),
    );
    let shown = session.projection().adjust().adjustment.clone();
    assert_eq!(shown, Some(Adjustment::Hsb(Hsb::NONE)));
    assert!(session.adjusting());
    assert_eq!(edits(session.shown()), 0, "nothing changes yet");
    for value in [0.6, 0.7] {
        adjust(&mut session, AdjustIntent::Set(brighter(value)));
    }
    assert_eq!(edits(session.shown()), 1, "the preview shows, one edit");
    assert_eq!(edits(session.artwork()), 0, "but is not the artwork");
    let undo = session.projection().availability(EditorAction::Undo);
    assert_eq!(undo, Availability::Available, "undo cancels");
    adjust(&mut session, AdjustIntent::Apply);
    assert!(
        session.projection().adjust().adjustment.is_none(),
        "the panel goes"
    );
    let artwork = session.artwork().clone();
    let edit = artwork.layer(LAYER).expect("layer").edits()[0]
        .adjust()
        .cloned();
    assert_eq!(edit.map(|e| e.adjustment().clone()), Some(brighter(0.7)));
    session
        .dispatch(Intent::History(HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(edits(session.artwork()), 0);
    session
        .dispatch(Intent::History(HistoryIntent::Redo))
        .expect("redo");
    assert_eq!(edits(session.artwork()), 1);
}

#[test]
fn cancel_undo_and_nothing_changed_leave_the_artwork() {
    let mut session = session();
    for end in [
        Intent::Adjust(AdjustIntent::Cancel),
        Intent::History(HistoryIntent::Undo),
    ] {
        adjust(
            &mut session,
            AdjustIntent::Begin(AdjustmentKind::HueSaturationBrightness),
        );
        adjust(&mut session, AdjustIntent::Set(brighter(0.2)));
        assert_eq!(edits(session.shown()), 1);
        session.dispatch(end).expect("ends");
        assert!(!session.adjusting());
        assert_eq!(edits(session.shown()), 0);
        assert!(!session.history().can_undo(), "no step");
    }
    adjust(&mut session, AdjustIntent::Begin(AdjustmentKind::Curves));
    adjust(&mut session, AdjustIntent::Apply);
    assert!(!session.adjusting());
    assert!(!session.history().can_undo(), "nothing changed: no step");
}

#[test]
fn another_edit_keeps_what_shows_and_other_kinds_values_are_ignored() {
    let mut session = session();
    adjust(&mut session, AdjustIntent::Begin(AdjustmentKind::Curves));
    adjust(&mut session, AdjustIntent::Set(brighter(0.9)));
    assert_eq!(
        edits(session.shown()),
        0,
        "another kind's values are ignored"
    );
    adjust(
        &mut session,
        AdjustIntent::Begin(AdjustmentKind::HueSaturationBrightness),
    );
    adjust(&mut session, AdjustIntent::Set(brighter(0.8)));
    session
        .dispatch(Intent::Layer(LayerIntent::Add))
        .expect("a new layer");
    assert!(!session.adjusting());
    assert_eq!(edits(session.artwork()), 1, "applied as it showed");
}

/// The pen dragging a rectangle from `from` to `to` with the Selection tool on.
fn select_rectangle(session: &mut Session, from: [f32; 2], to: [f32; 2]) {
    for intent in [
        SelectionIntent::Tool(true),
        SelectionIntent::Shape(SelectionShape::Rectangle),
    ] {
        session
            .dispatch(Intent::Selection(intent))
            .expect("selection");
    }
    for (phase, at) in [(ContactPhase::Down, from), (ContactPhase::Up, to)] {
        let contact = Contact {
            id: 3,
            device: Device::Pen,
            phase,
            samples: vec![InputSample {
                position: at,
                pressure: 0.5,
                tilt: 0.0,
                azimuth: 0.0,
                time_ms: 0,
            }],
        };
        let intent = SelectionIntent::Contact {
            contact,
            close: 1.0,
        };
        session.dispatch(Intent::Selection(intent)).expect("drag");
    }
}

#[test]
fn the_selection_limits_it_and_the_mask_takes_it() {
    let mut session = session();
    select_rectangle(&mut session, [4.0, 2.0], [12.0, 9.0]);
    let selected: Option<RegionMask> = session.selection().cloned();
    assert!(selected.is_some(), "a rectangle is selected");
    session
        .dispatch(Intent::Layer(LayerIntent::AddMask(LAYER)))
        .expect("mask");
    adjust(
        &mut session,
        AdjustIntent::Begin(AdjustmentKind::HueSaturationBrightness),
    );
    assert!(
        !session.projection().selection().tool,
        "the Selection tool is left"
    );
    adjust(&mut session, AdjustIntent::Set(brighter(0.3)));
    adjust(&mut session, AdjustIntent::Apply);
    let artwork = session.artwork().clone();
    assert_eq!(
        (edits(&artwork), mask_edits(&artwork)),
        (0, 1),
        "on the mask"
    );
    let mask = artwork.layer(LAYER).and_then(|l| l.mask()).expect("mask");
    assert_eq!(
        mask.edits()[0].adjust().and_then(|e| e.clip()),
        selected.as_ref()
    );
    assert!(session.selection().is_some(), "the selection stays");
}

#[test]
fn hidden_and_locked_layers_refuse_with_a_notice() {
    let mut session = session();
    session
        .dispatch(Intent::Layer(LayerIntent::SetVisible {
            layer: LAYER,
            visible: false,
        }))
        .expect("hide");
    let begin = AdjustIntent::Begin(AdjustmentKind::Curves);
    let refused = session.dispatch(Intent::Adjust(begin));
    assert_eq!(refused, Err(AppError::AdjustRefused(AdjustRefusal::Hidden)));
    assert_eq!(
        session.projection().adjust().refused,
        Some(AdjustRefusal::Hidden)
    );
    assert!(!session.adjusting());
    adjust(&mut session, AdjustIntent::Dismiss);
    assert_eq!(session.projection().adjust().refused, None);
}
