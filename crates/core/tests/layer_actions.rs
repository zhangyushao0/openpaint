//! Duplicate and Move (S38): a copy keeps everything but its ids; a move of `by` steps is that
//! many `Raise` or `Lower` steps, and `layer_places` lists exactly where they land.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, BrushRecipe, CanvasPoint, CanvasSize, Change, Color,
    CoreError, GroupCommand, GroupId, LayerCommand, LayerId, MAX_LAYERS, MaskCommand, StrokeSample,
    StrokeStyle, layer_places,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

fn apply(artwork: &Artwork, command: ArtworkCommand) -> Artwork {
    let applied = artwork.apply(command).expect("command");
    assert_eq!(applied.change, Change::Content);
    applied.artwork
}

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    apply(artwork, ArtworkCommand::Layer(command))
}

fn draw(artwork: &Artwork, at: f32) -> Artwork {
    let style =
        StrokeStyle::new(Arc::new(BrushRecipe::plain()), 6.0, 1.0, Color::BLACK).expect("style");
    let point = |x| CanvasPoint::new(x, at).expect("point");
    let first = StrokeSample::new(point(4.0), 1.0, 0.0, 0.0, 0).expect("sample");
    let mut session = artwork.begin_stroke(style, first).expect("begin");
    let last = StrokeSample::new(point(40.0), 1.0, 0.0, 0.0, 30).expect("sample");
    session.extend(&[last]).expect("extend");
    session.finish().artwork
}

/// Per layer bottom to top: its id and its group's id (0 for none).
fn rows(artwork: &Artwork) -> Vec<(u64, u64)> {
    artwork
        .layers()
        .iter()
        .map(|l| (l.id().get(), l.props().group().map_or(0, GroupId::get)))
        .collect()
}

#[test]
fn a_copy_lies_above_with_everything_but_its_ids_and_becomes_active() {
    let one = LayerId::new(1);
    let painted = draw(&draw(&blank(), 10.0), 20.0);
    let masked = apply(&painted, ArtworkCommand::Mask(MaskCommand::Add(one)));
    let masked = draw(&masked, 30.0);
    let blended = layer(
        &masked,
        LayerCommand::SetBlend {
            layer: one,
            blend: BlendMode::Multiply,
        },
    );
    let copied = layer(&blended, LayerCommand::Duplicate(one));
    assert_eq!(rows(&copied), [(1, 0), (2, 0)]);
    let (original, copy) = (&copied.layers()[0], &copied.layers()[1]);
    assert_eq!(copied.active(), copy.id());
    assert!(!copied.on_mask(), "the copy's paint is the target");
    assert_eq!(copy.name(), original.name());
    assert_eq!(copy.props(), original.props());
    assert_eq!(copy.edits(), original.edits());
    let strokes = |l: &openpaint_core::Layer| l.strokes().iter().cloned().collect::<Vec<_>>();
    let (old, new) = (strokes(original), strokes(copy));
    assert_eq!(old.len(), 2);
    for (old, new) in old.iter().zip(&new) {
        assert_ne!(old.id(), new.id(), "a stroke's id is never shared");
        assert_eq!(old.samples(), new.samples());
        assert_eq!(old.style(), new.style());
    }
    let (old_mask, new_mask) = (original.mask().expect("mask"), copy.mask().expect("mask"));
    assert_eq!(new_mask.strokes().len(), 1);
    assert_ne!(
        old_mask.strokes().last().map(|s| s.id()),
        new_mask.strokes().last().map(|s| s.id())
    );
    assert!(
        !copy.same_paint(original),
        "a copy is drawn again, not shared"
    );
    let stored = Artwork::from_storage(copied.to_parts()).expect("stored ids are unique");
    assert_eq!(stored.to_parts(), copied.to_parts());
    assert!(
        blended.layers().len() == 1,
        "the version before is untouched"
    );
}

#[test]
fn a_copy_stays_in_its_group_and_refuses_past_the_limit() {
    let two = layer(&blank(), LayerCommand::Create);
    let grouped = apply(
        &two,
        ArtworkCommand::Group(GroupCommand::Create {
            layer: LayerId::new(2),
        }),
    );
    let copied = layer(&grouped, LayerCommand::Duplicate(LayerId::new(1)));
    assert_eq!(rows(&copied), [(1, 3), (4, 3), (2, 3)]);
    let full = (1..MAX_LAYERS).fold(blank(), |a, _| layer(&a, LayerCommand::Create));
    let refused = full.apply(ArtworkCommand::Layer(LayerCommand::Duplicate(
        LayerId::new(1),
    )));
    assert_eq!(refused, Err(CoreError::LayerLimit { max: MAX_LAYERS }));
    let missing = LayerCommand::Duplicate(LayerId::new(99));
    assert_eq!(
        blank().apply(ArtworkCommand::Layer(missing)),
        Err(CoreError::UnknownLayer(LayerId::new(99)))
    );
}

/// Layers 1 to 6; 2 and 3 in one group, 5 alone in another.
fn grouped_stack() -> Artwork {
    let six = (1..6).fold(blank(), |a, _| layer(&a, LayerCommand::Create));
    let lower_group = apply(
        &six,
        ArtworkCommand::Group(GroupCommand::Create {
            layer: LayerId::new(3),
        }),
    );
    let lone = lower_group
        .apply(ArtworkCommand::Layer(LayerCommand::Select(LayerId::new(5))))
        .expect("select")
        .artwork;
    // A group of 5 with 4 under it, then 4 steps down and out: 5 alone.
    let both = apply(
        &lone,
        ArtworkCommand::Group(GroupCommand::Create {
            layer: LayerId::new(5),
        }),
    );
    layer(&both, LayerCommand::Lower(LayerId::new(4)))
}

#[test]
fn a_move_is_its_steps_and_the_places_are_where_they_land() {
    let stack = grouped_stack();
    assert_eq!(
        rows(&stack),
        [(1, 0), (2, 7), (3, 7), (4, 0), (5, 8), (6, 0)]
    );
    let groups: Vec<_> = stack.layers().iter().map(|l| l.props().group()).collect();
    for (index, moving) in stack.layers().iter().enumerate() {
        let id = moving.id();
        let places = layer_places(&groups, index);
        assert!(places.windows(2).all(|pair| pair[1].by == pair[0].by + 1));
        assert!(
            places
                .iter()
                .any(|place| place.by == 0 && place.index == index)
        );
        for place in &places {
            let moved = stack
                .apply(ArtworkCommand::Layer(LayerCommand::Move {
                    layer: id,
                    by: place.by,
                }))
                .expect("a listed place is reachable")
                .artwork;
            let mut stepped = stack.clone();
            for _ in 0..place.by.unsigned_abs() {
                let step = if place.by > 0 {
                    LayerCommand::Raise(id)
                } else {
                    LayerCommand::Lower(id)
                };
                stepped = layer(&stepped, step);
            }
            assert_eq!(rows(&moved), rows(&stepped), "{id:?} by {}", place.by);
            let at = moved.layers().iter().position(|l| l.id() == id);
            assert_eq!(at, Some(place.index));
            assert_eq!(moved.layers()[place.index].props().group(), place.group);
        }
        let (first, last) = (places[0].by, places[places.len() - 1].by);
        for by in [first - 1, last + 1] {
            let beyond = stack.apply(ArtworkCommand::Layer(LayerCommand::Move { layer: id, by }));
            assert!(matches!(beyond, Err(CoreError::IndexOutOfRange { .. })));
        }
    }
    let still = stack.apply(ArtworkCommand::Layer(LayerCommand::Move {
        layer: LayerId::new(1),
        by: 0,
    }));
    assert_eq!(still.expect("nothing").change, Change::None);
}

#[test]
fn passing_a_lone_group_takes_it_away_and_brings_no_place_back() {
    let stack = grouped_stack();
    let groups: Vec<_> = stack.layers().iter().map(|l| l.props().group()).collect();
    // Layer 5 alone in group 8: one step up leaves it (the group goes), then past 6.
    let places = layer_places(&groups, 4);
    let up: Vec<_> = places
        .iter()
        .filter(|p| p.by > 0)
        .map(|p| (p.index, p.group))
        .collect();
    assert_eq!(up, [(4, None), (5, None)]);
    let left = layer(&stack, LayerCommand::Raise(LayerId::new(5)));
    assert!(left.groups().iter().all(|g| g.id() != GroupId::new(8)));
}
