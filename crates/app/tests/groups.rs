//! Group, Clipping Mask and blend intents: each is one history step that undo takes back, one
//! group opacity touch is one step, the projection offers grouping and moves by the group
//! rules, and a layer in a hidden group refuses fills and transforms like a hidden layer.

use openpaint_app::{
    AppError, Availability, EditorAction, FillIntent, FillRefusal, HistoryIntent, Intent,
    LayerIntent, Session, TransformIntent, TransformRefusal,
};
use openpaint_app::{BrushLibrary, Toolbox};
use openpaint_core::{BlendMode, CanvasSize, Color, GroupId, LayerId};

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

/// Layers 1 and 2, then 2 grouped with 1 into group 3; layer 2 active.
fn grouped() -> Session {
    let mut session = session();
    layer(&mut session, LayerIntent::Add).expect("add");
    layer(&mut session, LayerIntent::Group(LayerId::new(2))).expect("group");
    session
}

#[test]
fn grouping_clipping_and_blending_are_steps_undo_takes_back() {
    let mut session = grouped();
    let list = session.projection().layers().cloned().expect("layers");
    assert_eq!(list.groups.len(), 1);
    assert!(
        list.rows
            .iter()
            .all(|row| row.group == Some(GroupId::new(3)))
    );
    let intents = [
        LayerIntent::SetClipping {
            layer: LayerId::new(2),
            clipping: true,
        },
        LayerIntent::SetBlend {
            layer: LayerId::new(2),
            blend: BlendMode::Screen,
        },
        LayerIntent::SetGroupVisible {
            group: GroupId::new(3),
            visible: false,
        },
        LayerIntent::Ungroup(GroupId::new(3)),
        LayerIntent::NewGroup,
    ];
    let mut versions = vec![session.artwork().clone()];
    for intent in intents {
        layer(&mut session, intent).expect("intent");
        versions.push(session.artwork().clone());
    }
    let list = session.projection().layers().cloned().expect("layers");
    let clipped = list.rows.iter().find(|row| row.id == LayerId::new(2));
    let clipped = clipped.expect("layer 2");
    assert!(clipped.clipping && clipped.clipped && clipped.blend == BlendMode::Screen);
    assert_eq!(list.active, LayerId::new(4), "the new group's layer");
    for version in versions.iter().rev().skip(1) {
        step(&mut session, HistoryIntent::Undo);
        assert_eq!(session.artwork(), version);
    }
    step(&mut session, HistoryIntent::Redo);
    assert_eq!(session.artwork(), &versions[1]);
}

#[test]
fn one_group_opacity_touch_is_one_step() {
    let mut session = grouped();
    let before = session.artwork().clone();
    for (opacity, settled) in [(0.8, false), (0.5, false), (0.3, true)] {
        let intent = LayerIntent::SetGroupOpacity {
            group: GroupId::new(3),
            opacity,
            settled,
        };
        layer(&mut session, intent).expect("fade");
    }
    let list = session.projection().layers().cloned().expect("layers");
    assert_eq!(list.group(GroupId::new(3)).map(|g| g.opacity), Some(0.3));
    step(&mut session, HistoryIntent::Undo);
    assert_eq!(session.artwork(), &before);
}

#[test]
fn grouping_and_moves_follow_the_group_rules() {
    let mut session = grouped();
    assert_eq!(
        offered(&session, EditorAction::GroupLayer),
        Availability::NothingLeft,
        "already in a group"
    );
    assert_eq!(
        offered(&session, EditorAction::RaiseLayer),
        Availability::Available,
        "the top layer can still leave its group"
    );
    layer(&mut session, LayerIntent::Raise(LayerId::new(2))).expect("leave");
    assert_eq!(
        offered(&session, EditorAction::RaiseLayer),
        Availability::NothingLeft
    );
    assert_eq!(
        offered(&session, EditorAction::GroupLayer),
        Availability::NothingLeft,
        "the layer under it lies in a group"
    );
    let refused = layer(&mut session, LayerIntent::Group(LayerId::new(2)));
    assert!(matches!(refused, Err(AppError::Unavailable { .. })));
}

#[test]
fn a_layer_in_a_hidden_group_refuses_fills_and_transforms() {
    let mut session = grouped();
    let hide = LayerIntent::SetGroupVisible {
        group: GroupId::new(3),
        visible: false,
    };
    layer(&mut session, hide).expect("hide");
    let fill = session.dispatch(Intent::Fill(FillIntent::Request {
        point: [10.0, 10.0],
        threshold: 0.5,
        color: Color::BLACK,
    }));
    assert_eq!(fill, Err(AppError::FillRefused(FillRefusal::Hidden)));
    let transform = session.dispatch(Intent::Transform(TransformIntent::Tool(true)));
    assert_eq!(
        transform,
        Err(AppError::TransformRefused(TransformRefusal::Hidden))
    );
}
