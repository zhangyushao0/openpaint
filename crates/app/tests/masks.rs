//! Mask intents (S24): adding, turning off and on, inverting, clearing and deleting a mask are
//! one history step each that undo takes back; choosing the mask or the paint is no step of its
//! own; a stroke drawn while the mask is chosen lands on the mask in grey; the projection offers
//! each mask action by the mask rules; a fill on the mask asks for the mask's pixels and a
//! transform is refused there.

use openpaint_app::{
    AppError, Availability, Contact, ContactPhase, Device, EditorAction, FillIntent, HistoryIntent,
    InputSample, Intent, LayerIntent, MaskRow, Session, StrokeIntent, TransformIntent,
    TransformRefusal,
};
use openpaint_app::{BrushLibrary, Toolbox};
use openpaint_core::{CanvasSize, Color, LayerId};

const LAYER: LayerId = LayerId::new(1);

fn session() -> Session {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = Toolbox::new(library).style().expect("style");
    Session::new(CanvasSize::new(200, 100).expect("size"), brush)
}

fn layer(session: &mut Session, intent: LayerIntent) -> Result<(), AppError> {
    session.dispatch(Intent::Layer(intent))
}

fn step(session: &mut Session, step: HistoryIntent) {
    session.dispatch(Intent::History(step)).expect("history");
}

fn offered(session: &Session, action: EditorAction) -> Availability {
    session.projection().availability(action)
}

fn pen(phase: ContactPhase, x: f32, time_ms: i64) -> Intent {
    Intent::Stroke(StrokeIntent::Contact(Contact {
        id: 1,
        device: Device::Pen,
        phase,
        samples: vec![InputSample {
            position: [x, 50.0],
            pressure: 0.8,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms,
        }],
    }))
}

fn stroke(session: &mut Session) {
    for (phase, x, time) in [
        (ContactPhase::Down, 20.0, 0),
        (ContactPhase::Move, 60.0, 8),
        (ContactPhase::Up, 100.0, 16),
    ] {
        session.dispatch(pen(phase, x, time)).expect("pen");
    }
}

fn mask_row(session: &Session) -> Option<MaskRow> {
    let list = session.projection().layers().cloned().expect("layers");
    list.rows
        .iter()
        .find(|row| row.id == LAYER)
        .and_then(|row| row.mask)
}

#[test]
fn each_mask_action_is_one_step_and_choosing_the_target_is_none() {
    let mut session = session();
    assert_eq!(
        offered(&session, EditorAction::AddMask),
        Availability::Available
    );
    assert_eq!(
        offered(&session, EditorAction::EditMask),
        Availability::NothingLeft
    );
    layer(&mut session, LayerIntent::AddMask(LAYER)).expect("add");
    assert!(session.artwork().on_mask(), "the new mask is the target");
    assert_eq!(
        offered(&session, EditorAction::AddMask),
        Availability::NothingLeft
    );
    let white = MaskRow {
        enabled: true,
        inverted: false,
    };
    assert_eq!(mask_row(&session), Some(white));
    let steps = session.history().undo_steps();
    layer(&mut session, LayerIntent::Select(LAYER)).expect("paint");
    assert!(!session.artwork().on_mask());
    layer(&mut session, LayerIntent::SelectMask(LAYER)).expect("mask");
    assert_eq!(session.history().undo_steps(), steps, "choosing is no step");
    layer(&mut session, LayerIntent::InvertMask(LAYER)).expect("invert");
    layer(&mut session, LayerIntent::ClearMask(LAYER)).expect("clear");
    let off = LayerIntent::SetMaskEnabled {
        layer: LAYER,
        enabled: false,
    };
    layer(&mut session, off).expect("off");
    assert!(!session.artwork().on_mask(), "an off mask is no target");
    assert_eq!(
        offered(&session, EditorAction::ChangeMask),
        Availability::NothingLeft
    );
    let refused = layer(&mut session, LayerIntent::SelectMask(LAYER));
    assert!(matches!(refused, Err(AppError::Unavailable { .. })));
    layer(&mut session, LayerIntent::DeleteMask(LAYER)).expect("delete");
    assert_eq!(mask_row(&session), None);
    assert_eq!(session.history().undo_steps(), steps + 4);
    for _ in 0..4 {
        step(&mut session, HistoryIntent::Undo);
    }
    assert_eq!(mask_row(&session), Some(white));
    assert!(session.artwork().on_mask(), "back on the mask, as it was");
    step(&mut session, HistoryIntent::Undo);
    assert_eq!(mask_row(&session), None, "the add undone");
}

#[test]
fn a_stroke_on_the_mask_lands_there_in_grey_and_the_paint_stays() {
    let mut session = session();
    stroke(&mut session);
    layer(&mut session, LayerIntent::AddMask(LAYER)).expect("add");
    stroke(&mut session);
    let artwork = session.artwork().clone();
    let on = artwork.layer(LAYER).expect("layer");
    assert_eq!(on.strokes().len(), 1, "the paint keeps its one stroke");
    let mask = on.mask().expect("mask");
    let color = mask
        .strokes()
        .last()
        .expect("a mask stroke")
        .style()
        .color();
    assert_eq!(
        (color.r(), color.g(), color.b()),
        (0.0, 0.0, 0.0),
        "black's grey"
    );
    step(&mut session, HistoryIntent::Undo);
    let undone = session.artwork().layer(LAYER).expect("layer");
    assert!(undone.mask().is_some_and(|mask| mask.strokes().is_empty()));
}

#[test]
fn a_fill_on_the_mask_samples_the_mask_and_a_transform_is_refused() {
    let mut session = session();
    layer(&mut session, LayerIntent::AddMask(LAYER)).expect("add");
    let request = FillIntent::Request {
        point: [50.0, 50.0],
        threshold: 0.5,
        color: Color::BLACK,
    };
    session.dispatch(Intent::Fill(request)).expect("fill");
    let job = session.fill_request().expect("a job");
    assert!(job.mask && job.layer == LAYER, "the mask's pixels");
    session
        .dispatch(Intent::Fill(FillIntent::Cancel))
        .expect("cancel");
    let tool = session.dispatch(Intent::Transform(TransformIntent::Tool(true)));
    assert_eq!(
        tool,
        Err(AppError::TransformRefused(TransformRefusal::Mask))
    );
    layer(&mut session, LayerIntent::Select(LAYER)).expect("paint");
    session
        .dispatch(Intent::Transform(TransformIntent::Tool(true)))
        .expect("the paint transforms");
}
