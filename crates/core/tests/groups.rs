//! Groups, Clipping Masks and blend modes as artwork semantics: grouping keeps clipping chains
//! whole, moves never split a group, a group goes with its last layer, chains stop at group
//! edges, and storage rejects broken membership.

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, Change, CoreError, GroupCommand, GroupId,
    LayerCommand, LayerId,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).unwrap())
}

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    let applied = artwork.apply(ArtworkCommand::Layer(command)).unwrap();
    assert_eq!(applied.change, Change::Content);
    applied.artwork
}

fn group(artwork: &Artwork, command: GroupCommand) -> Result<Artwork, CoreError> {
    artwork
        .apply(ArtworkCommand::Group(command))
        .map(|applied| applied.artwork)
}

/// Layers 1 to `count` from the bottom, the top one active.
fn stack(count: usize) -> Artwork {
    (1..count).fold(blank(), |artwork, _| layer(&artwork, LayerCommand::Create))
}

/// Per layer bottom to top: its id and its group's id (0 for none).
fn rows(artwork: &Artwork) -> Vec<(u64, u64)> {
    artwork
        .layers()
        .iter()
        .map(|l| (l.id().get(), l.props().group().map_or(0, GroupId::get)))
        .collect()
}

fn clip(artwork: &Artwork, id: u64) -> Artwork {
    layer(
        artwork,
        LayerCommand::SetClipping {
            layer: LayerId::new(id),
            clipping: true,
        },
    )
}

#[test]
fn a_layer_groups_with_the_one_below_and_a_chain_groups_whole() {
    let artwork = stack(3);
    let grouped = group(
        &artwork,
        GroupCommand::Create {
            layer: LayerId::new(3),
        },
    )
    .unwrap();
    assert_eq!(
        rows(&grouped),
        [(1, 0), (2, 4), (3, 4)],
        "ids from the layer counter"
    );
    let made = &grouped.groups()[0];
    assert_eq!(
        (made.name().as_str(), made.visible(), made.opacity()),
        ("Group 4", true, 1.0)
    );
    assert_eq!(
        group(
            &grouped,
            GroupCommand::Create {
                layer: LayerId::new(3)
            }
        ),
        Err(CoreError::GroupLayered(LayerId::new(3)))
    );
    assert_eq!(
        group(
            &grouped,
            GroupCommand::Create {
                layer: LayerId::new(1)
            }
        )
        .map(|a| rows(&a)),
        Ok(vec![(1, 5), (2, 4), (3, 4)]),
        "at the bottom it groups alone"
    );
    // 2 clips to 1: grouping either takes the chain, and only the chain.
    let chained = clip(&stack(3), 2);
    for id in [1, 2] {
        let grouped = group(
            &chained,
            GroupCommand::Create {
                layer: LayerId::new(id),
            },
        )
        .unwrap();
        assert_eq!(rows(&grouped), [(1, 4), (2, 4), (3, 0)]);
        assert!(grouped.clips_at(1), "the chain stays whole");
    }
}

#[test]
fn moves_enter_and_leave_groups_at_their_edges_and_never_split_one() {
    let artwork = stack(4);
    let artwork = group(
        &artwork,
        GroupCommand::Create {
            layer: LayerId::new(3),
        },
    )
    .unwrap();
    assert_eq!(rows(&artwork), [(1, 0), (2, 5), (3, 5), (4, 0)]);
    let raised = layer(&artwork, LayerCommand::Raise(LayerId::new(1)));
    assert_eq!(
        rows(&raised),
        [(1, 5), (2, 5), (3, 5), (4, 0)],
        "joins from below"
    );
    let raised = layer(&raised, LayerCommand::Raise(LayerId::new(1)));
    assert_eq!(
        rows(&raised),
        [(2, 5), (1, 5), (3, 5), (4, 0)],
        "moves inside"
    );
    let lowered = layer(&artwork, LayerCommand::Lower(LayerId::new(4)));
    assert_eq!(
        rows(&lowered),
        [(1, 0), (2, 5), (3, 5), (4, 5)],
        "joins from above"
    );
    let left = layer(&artwork, LayerCommand::Raise(LayerId::new(3)));
    assert_eq!(
        rows(&left),
        [(1, 0), (2, 5), (3, 0), (4, 0)],
        "the top leaves in place"
    );
    let gone = layer(&left, LayerCommand::Lower(LayerId::new(2)));
    assert_eq!(rows(&gone), [(1, 0), (2, 0), (3, 0), (4, 0)]);
    assert!(
        gone.groups().is_empty(),
        "the group went with its last layer"
    );
    assert_eq!(
        artwork.apply(ArtworkCommand::Layer(LayerCommand::Raise(LayerId::new(4)))),
        Err(CoreError::IndexOutOfRange { index: 4, len: 4 })
    );
    // Reorder lands in the group only between two of its layers, and never splits it.
    let into = layer(
        &artwork,
        LayerCommand::Reorder {
            layer: LayerId::new(4),
            index: 2,
        },
    );
    assert_eq!(rows(&into), [(1, 0), (2, 5), (4, 5), (3, 5)]);
    let past = layer(
        &artwork,
        LayerCommand::Reorder {
            layer: LayerId::new(1),
            index: 3,
        },
    );
    assert_eq!(rows(&past), [(2, 5), (3, 5), (4, 0), (1, 0)]);
}

#[test]
fn clipping_chains_stop_at_group_edges_and_a_bottom_clip_is_a_base() {
    let artwork = clip(&clip(&stack(3), 2), 3);
    assert!(!artwork.clips_at(0) && artwork.clips_at(1) && artwork.clips_at(2));
    let bottom = clip(&artwork, 1);
    assert!(!bottom.clips_at(0), "nothing under it: it is the base");
    let grouped = group(
        &stack(3),
        GroupCommand::Create {
            layer: LayerId::new(3),
        },
    )
    .unwrap();
    let grouped = clip(&grouped, 2);
    assert!(
        !grouped.clips_at(1),
        "the first layer of a group has no base under it"
    );
    let blended = layer(
        &artwork,
        LayerCommand::SetBlend {
            layer: LayerId::new(2),
            blend: BlendMode::Multiply,
        },
    );
    assert_eq!(blended.layers()[1].props().blend(), BlendMode::Multiply);
}

#[test]
fn hiding_and_ungrouping_follow_the_group() {
    let artwork = group(
        &stack(3),
        GroupCommand::Create {
            layer: LayerId::new(3),
        },
    )
    .unwrap();
    let id = GroupId::new(4);
    let hidden = group(
        &artwork,
        GroupCommand::SetVisible {
            group: id,
            visible: false,
        },
    )
    .unwrap();
    assert!(hidden.shown(LayerId::new(1)) && !hidden.shown(LayerId::new(2)));
    assert!(
        hidden.layers()[1].props().visible(),
        "the layer's own eye stays open"
    );
    let faded = group(
        &hidden,
        GroupCommand::SetOpacity {
            group: id,
            opacity: 0.4,
        },
    )
    .unwrap();
    assert_eq!(faded.group(id).map(|g| g.opacity()), Some(0.4));
    let ungrouped = group(&hidden, GroupCommand::Ungroup(id)).unwrap();
    assert_eq!(rows(&ungrouped), [(1, 0), (2, 0), (3, 0)]);
    assert!(
        !ungrouped.shown(LayerId::new(2)),
        "a hidden group leaves its layers hidden"
    );
    assert_eq!(
        group(&artwork, GroupCommand::Ungroup(GroupId::new(9))),
        Err(CoreError::UnknownGroup(GroupId::new(9)))
    );
}

#[test]
fn new_groups_new_layers_and_deleting_follow_the_group() {
    let artwork = group(
        &stack(3),
        GroupCommand::Create {
            layer: LayerId::new(3),
        },
    )
    .unwrap();
    let selected = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::Select(LayerId::new(1))))
        .unwrap()
        .artwork;
    let fresh = group(&selected, GroupCommand::New).unwrap();
    assert_eq!(rows(&fresh), [(1, 0), (5, 6), (2, 4), (3, 4)]);
    assert_eq!(fresh.active(), LayerId::new(5));
    let inside = group(&artwork, GroupCommand::New).unwrap();
    assert_eq!(
        rows(&inside),
        [(1, 0), (2, 4), (3, 4), (5, 6)],
        "above the active group"
    );
    let added = layer(&artwork, LayerCommand::Create);
    assert_eq!(
        rows(&added).last(),
        Some(&(5, 4)),
        "a new layer joins the active one's group"
    );
    let one = layer(&artwork, LayerCommand::Delete(LayerId::new(3)));
    let none = layer(&one, LayerCommand::Delete(LayerId::new(2)));
    assert!(none.groups().is_empty());
}

#[test]
fn storage_keeps_groups_and_rejects_broken_membership() {
    let artwork = group(
        &stack(3),
        GroupCommand::Create {
            layer: LayerId::new(3),
        },
    )
    .unwrap();
    let artwork = clip(&artwork, 3);
    assert_eq!(Artwork::from_storage(artwork.to_parts()).unwrap(), artwork);
    let mut split = artwork.to_parts();
    split.layers.swap(0, 1);
    assert!(
        Artwork::from_storage(split).is_err(),
        "a group split by a layer"
    );
    let mut dangling = artwork.to_parts();
    dangling.layers[0].group = Some(7);
    assert!(
        Artwork::from_storage(dangling).is_err(),
        "a group that does not exist"
    );
    let mut empty = artwork.to_parts();
    empty.layers[1].group = None;
    empty.layers[2].group = None;
    assert!(
        Artwork::from_storage(empty).is_err(),
        "a group without layers"
    );
    let mut clash = artwork.to_parts();
    clash.groups[0].id = 2;
    assert!(
        Artwork::from_storage(clash).is_err(),
        "a group sharing a layer's id"
    );
}
