//! Every `LayerCommand`: its success case, its error cases, and that the input version never changes.

use openpaint_core::{
    Applied, Artwork, ArtworkCommand, CanvasSize, Change, CoreError, LayerCommand, LayerId,
    LayerName, MAX_LAYERS,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).unwrap())
}

fn run(artwork: &Artwork, command: LayerCommand) -> Result<Applied, CoreError> {
    artwork.apply(ArtworkCommand::Layer(command))
}

fn ok(artwork: &Artwork, command: LayerCommand) -> Artwork {
    let applied = run(artwork, command).unwrap();
    assert_eq!(applied.change, Change::Content);
    applied.artwork
}

/// An artwork with layers 1, 2, 3 from the bottom; layer 3 is active.
fn three() -> Artwork {
    let one = ok(&blank(), LayerCommand::Create);
    ok(&one, LayerCommand::Create)
}

fn ids(artwork: &Artwork) -> Vec<u64> {
    artwork
        .layers()
        .iter()
        .map(|layer| layer.id().get())
        .collect()
}

const MISSING: LayerId = LayerId::new(99);

#[test]
fn new_artwork_has_one_active_layer() {
    let artwork = blank();
    assert_eq!(ids(&artwork), [1]);
    assert_eq!(artwork.active(), LayerId::new(1));
    let layer = artwork.layer(LayerId::new(1)).unwrap();
    assert_eq!(layer.name().as_str(), "Layer 1");
    assert!(layer.props().visible() && !layer.props().locked());
    assert_eq!(layer.props().opacity(), 1.0);
    assert!(layer.strokes().is_empty());
}

#[test]
fn create_inserts_above_active_and_selects_it() {
    let base = three();
    let below = run(&base, LayerCommand::Select(LayerId::new(1)))
        .unwrap()
        .artwork;
    let next = ok(&below, LayerCommand::Create);
    assert_eq!(ids(&next), [1, 4, 2, 3]);
    assert_eq!(next.active(), LayerId::new(4));
    assert_eq!(
        next.layer(LayerId::new(4)).unwrap().name().as_str(),
        "Layer 4"
    );
    assert_eq!(ids(&below), [1, 2, 3], "the input version is unchanged");
}

#[test]
fn create_stops_at_the_layer_limit() {
    let mut artwork = blank();
    for _ in 1..MAX_LAYERS {
        artwork = ok(&artwork, LayerCommand::Create);
    }
    assert_eq!(artwork.layers().len(), MAX_LAYERS);
    let result = run(&artwork, LayerCommand::Create);
    assert_eq!(result, Err(CoreError::LayerLimit { max: MAX_LAYERS }));
}

#[test]
fn delete_removes_and_moves_the_active_layer_down() {
    let base = three();
    let next = ok(&base, LayerCommand::Delete(LayerId::new(3)));
    assert_eq!(ids(&next), [1, 2]);
    assert_eq!(next.active(), LayerId::new(2));
    let bottom = run(&base, LayerCommand::Select(LayerId::new(1)))
        .unwrap()
        .artwork;
    let next = ok(&bottom, LayerCommand::Delete(LayerId::new(1)));
    assert_eq!(
        next.active(),
        LayerId::new(2),
        "deleting the bottom selects the new bottom"
    );
    let next = ok(&base, LayerCommand::Delete(LayerId::new(1)));
    assert_eq!(
        next.active(),
        LayerId::new(3),
        "an inactive delete keeps the active layer"
    );
}

#[test]
fn delete_errors() {
    assert_eq!(
        run(&three(), LayerCommand::Delete(MISSING)),
        Err(CoreError::UnknownLayer(MISSING))
    );
    let only = blank();
    assert_eq!(
        run(&only, LayerCommand::Delete(only.active())),
        Err(CoreError::LastLayer)
    );
}

#[test]
fn deleted_ids_are_never_reused() {
    let next = ok(&three(), LayerCommand::Delete(LayerId::new(3)));
    let next = ok(&next, LayerCommand::Create);
    assert_eq!(ids(&next), [1, 2, 4]);
}

#[test]
fn reorder_moves_a_layer() {
    let base = three();
    let next = ok(
        &base,
        LayerCommand::Reorder {
            layer: LayerId::new(3),
            index: 0,
        },
    );
    assert_eq!(ids(&next), [3, 1, 2]);
    let next = ok(
        &base,
        LayerCommand::Reorder {
            layer: LayerId::new(1),
            index: 2,
        },
    );
    assert_eq!(ids(&next), [2, 3, 1]);
    let same = run(
        &base,
        LayerCommand::Reorder {
            layer: LayerId::new(2),
            index: 1,
        },
    )
    .unwrap();
    assert_eq!(same.change, Change::None);
    assert!(same.artwork.same_version(&base));
}

#[test]
fn reorder_errors() {
    let base = three();
    assert_eq!(
        run(
            &base,
            LayerCommand::Reorder {
                layer: MISSING,
                index: 0
            }
        ),
        Err(CoreError::UnknownLayer(MISSING))
    );
    assert_eq!(
        run(
            &base,
            LayerCommand::Reorder {
                layer: LayerId::new(1),
                index: 3
            }
        ),
        Err(CoreError::IndexOutOfRange { index: 3, len: 3 })
    );
}

#[test]
fn select_changes_only_the_active_layer() {
    let base = three();
    let applied = run(&base, LayerCommand::Select(LayerId::new(1))).unwrap();
    assert_eq!(applied.change, Change::ActiveLayer);
    assert_eq!(applied.artwork.active(), LayerId::new(1));
    assert_eq!(applied.artwork.layers(), base.layers());
    let again = run(&base, LayerCommand::Select(base.active())).unwrap();
    assert_eq!(again.change, Change::None);
    assert_eq!(
        run(&base, LayerCommand::Select(MISSING)),
        Err(CoreError::UnknownLayer(MISSING))
    );
}

#[test]
fn rename() {
    let base = three();
    let name = LayerName::new("Inks").unwrap();
    let next = ok(
        &base,
        LayerCommand::Rename {
            layer: LayerId::new(2),
            name: name.clone(),
        },
    );
    assert_eq!(next.layer(LayerId::new(2)).unwrap().name(), &name);
    let same = run(
        &next,
        LayerCommand::Rename {
            layer: LayerId::new(2),
            name: name.clone(),
        },
    );
    assert_eq!(same.unwrap().change, Change::None);
    assert_eq!(
        run(
            &base,
            LayerCommand::Rename {
                layer: MISSING,
                name
            }
        ),
        Err(CoreError::UnknownLayer(MISSING))
    );
}

#[test]
fn set_visible_and_locked() {
    let base = three();
    let id = LayerId::new(2);
    let hidden = ok(
        &base,
        LayerCommand::SetVisible {
            layer: id,
            visible: false,
        },
    );
    assert!(!hidden.layer(id).unwrap().props().visible());
    let locked = ok(
        &hidden,
        LayerCommand::SetLocked {
            layer: id,
            locked: true,
        },
    );
    assert!(locked.layer(id).unwrap().props().locked());
    let same = run(
        &locked,
        LayerCommand::SetLocked {
            layer: id,
            locked: true,
        },
    )
    .unwrap();
    assert_eq!(same.change, Change::None);
    assert_eq!(
        run(
            &base,
            LayerCommand::SetVisible {
                layer: MISSING,
                visible: false
            }
        ),
        Err(CoreError::UnknownLayer(MISSING))
    );
    assert_eq!(
        run(
            &base,
            LayerCommand::SetLocked {
                layer: MISSING,
                locked: true
            }
        ),
        Err(CoreError::UnknownLayer(MISSING))
    );
}

#[test]
fn set_opacity() {
    let base = three();
    let id = LayerId::new(1);
    let next = ok(
        &base,
        LayerCommand::SetOpacity {
            layer: id,
            opacity: 0.25,
        },
    );
    assert_eq!(next.layer(id).unwrap().props().opacity(), 0.25);
    for opacity in [-0.1, 1.01, f32::NAN] {
        assert_eq!(
            run(&base, LayerCommand::SetOpacity { layer: id, opacity }),
            Err(CoreError::InvalidValue {
                what: "layer opacity"
            })
        );
    }
    assert_eq!(
        run(
            &base,
            LayerCommand::SetOpacity {
                layer: MISSING,
                opacity: 0.5
            }
        ),
        Err(CoreError::UnknownLayer(MISSING))
    );
}

#[test]
fn errors_display_a_sentence() {
    assert_eq!(
        CoreError::UnknownLayer(MISSING).to_string(),
        "no layer with id 99"
    );
    assert_eq!(
        CoreError::IndexOutOfRange { index: 3, len: 3 }.to_string(),
        "layer position 3 is outside 0..3"
    );
}
