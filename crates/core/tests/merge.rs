//! Merge Down (S39): the upper layer becomes a nested record at the end of the lower one's
//! content with its own content and how it composited; the lower layer's mask and (merged
//! over) opacity move into a nested record of their own where they would otherwise apply to
//! both; where the look would change it is refused with the reason; copies, storage and later
//! strokes treat nested records as content.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError,
    EditParts, GroupCommand, LayerCommand, LayerId, MAX_NESTING, MaskCommand, NestedLayer,
    StrokeSample, StrokeStyle, Unmergeable,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

fn apply(artwork: &Artwork, command: ArtworkCommand) -> Artwork {
    artwork.apply(command).expect("command").artwork
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

fn id(raw: u64) -> LayerId {
    LayerId::new(raw)
}

/// Layer 1 with two strokes, layer 2 above it with one, active.
fn two() -> Artwork {
    let lower = draw(&draw(&blank(), 10.0), 20.0);
    let created = layer(&lower, LayerCommand::Create);
    draw(&created, 30.0)
}

fn merge(artwork: &Artwork) -> Artwork {
    layer(artwork, LayerCommand::MergeDown(artwork.active()))
}

fn nested(artwork: &Artwork, index: usize, edit: usize) -> &NestedLayer {
    let edit = &artwork.layers()[index].edits()[edit];
    edit.nested().expect("a nested record").layer()
}

fn clip(artwork: &Artwork, raw: u64) -> Artwork {
    layer(
        artwork,
        LayerCommand::SetClipping {
            layer: id(raw),
            clipping: true,
        },
    )
}

#[test]
fn the_upper_layer_goes_after_the_lower_ones_strokes_with_how_it_composited() {
    let artwork = layer(
        &two(),
        LayerCommand::SetOpacity {
            layer: id(2),
            opacity: 0.4,
        },
    );
    let merged = merge(&artwork);
    assert_eq!(merged.layers().len(), 1);
    assert_eq!((merged.active(), merged.on_mask()), (id(1), false));
    let lower = &merged.layers()[0];
    assert_eq!(lower.strokes().len(), 2, "its own strokes stay");
    assert_eq!(lower.edits()[0].after(), 2, "after them");
    let upper = nested(&merged, 0, 0);
    assert_eq!(upper.id(), id(2), "the merged layer's id");
    assert_eq!(
        (upper.opacity(), upper.blend(), upper.clipped()),
        (0.4, BlendMode::Normal, false)
    );
    assert!(
        upper.strokes().same(artwork.layers()[1].strokes()),
        "its strokes, shared"
    );
    assert_eq!(upper.depth(), 1);
    // Strokes drawn later lie after the nested record.
    let later = draw(&merged, 40.0);
    assert_eq!(later.layers()[0].strokes().len(), 3);
    assert_eq!(later.layers()[0].edits()[0].after(), 2);
}

#[test]
fn a_clipped_layer_goes_atop_and_the_lower_mask_moves_into_a_record_of_its_own() {
    let clipped = clip(&two(), 2);
    let masked = apply(&clipped, ArtworkCommand::Mask(MaskCommand::Add(id(1))));
    let masked = draw(&masked, 5.0);
    let faded = layer(
        &masked,
        LayerCommand::SetOpacity {
            layer: id(1),
            opacity: 0.5,
        },
    );
    let artwork = layer(&faded, LayerCommand::Select(id(2)));
    let merged = merge(&artwork);
    let lower = &merged.layers()[0];
    assert!(
        lower.mask().is_none(),
        "the mask went with the lower layer's own content"
    );
    assert_eq!(
        lower.props().opacity(),
        0.5,
        "atop: the opacity stays the chain's"
    );
    assert_eq!(lower.strokes().len(), 0);
    let own = nested(&merged, 0, 0);
    assert_eq!(
        (own.id(), own.opacity(), own.clipped()),
        (id(3), 1.0, false)
    );
    assert_eq!(own.strokes().len(), 2);
    assert_eq!(own.mask().map(|mask| mask.strokes().len()), Some(1));
    let upper = nested(&merged, 0, 1);
    assert!(upper.clipped());
    assert_eq!(merged.next_layer_id(), artwork.next_layer_id() + 1);
}

#[test]
fn merged_over_the_lower_opacity_moves_into_its_own_record_and_the_layer_shows_full() {
    let faded = layer(
        &two(),
        LayerCommand::SetOpacity {
            layer: id(1),
            opacity: 0.3,
        },
    );
    let merged = merge(&faded);
    assert_eq!(merged.layers()[0].props().opacity(), 1.0);
    assert_eq!(nested(&merged, 0, 0).opacity(), 0.3);
    assert_eq!(nested(&merged, 0, 1).id(), id(2));
    // Nothing to move at full opacity without a mask.
    assert_eq!(merge(&two()).layers()[0].edits().len(), 1);
}

#[test]
fn where_the_look_would_change_merging_is_refused_with_why() {
    let why = |artwork: &Artwork| artwork.mergeable(artwork.active()).err();
    let bottom = layer(&two(), LayerCommand::Select(id(1)));
    assert_eq!(why(&bottom), Some(Unmergeable::NothingBelow));
    let hidden = layer(
        &two(),
        LayerCommand::SetVisible {
            layer: id(1),
            visible: false,
        },
    );
    assert_eq!(why(&hidden), Some(Unmergeable::Hidden));
    let locked = layer(
        &two(),
        LayerCommand::SetLocked {
            layer: id(2),
            locked: true,
        },
    );
    assert_eq!(why(&locked), Some(Unmergeable::Locked));
    let multiply = layer(
        &two(),
        LayerCommand::SetBlend {
            layer: id(2),
            blend: BlendMode::Multiply,
        },
    );
    assert_eq!(why(&multiply), Some(Unmergeable::Modes));
    assert_eq!(why(&clip(&multiply, 2)), None, "clipped: any mode");
    let both = layer(
        &multiply,
        LayerCommand::SetBlend {
            layer: id(1),
            blend: BlendMode::Multiply,
        },
    );
    assert_eq!(why(&both), None, "one composing mode");
    let overlay = |raw| LayerCommand::SetBlend {
        layer: id(raw),
        blend: BlendMode::Overlay,
    };
    assert_eq!(
        why(&layer(&layer(&two(), overlay(1)), overlay(2))),
        Some(Unmergeable::Modes)
    );
}

#[test]
fn clipping_that_would_change_and_the_bottom_layer_are_refused_with_why() {
    let why = |artwork: &Artwork| artwork.mergeable(artwork.active()).err();
    // A third layer: the middle one clips to the bottom, the top one does not clip.
    let three = draw(&layer(&clip(&two(), 2), LayerCommand::Create), 35.0);
    assert_eq!(why(&three), Some(Unmergeable::BelowClips));
    // Layers clip to the upper one, which does not clip.
    let based = clip(&three, 3);
    let middle = layer(
        &based,
        LayerCommand::SetClipping {
            layer: id(2),
            clipping: false,
        },
    );
    assert_eq!(
        why(&layer(&middle, LayerCommand::Select(id(2)))),
        Some(Unmergeable::ClippedTo)
    );
    let refused = two().apply(ArtworkCommand::Layer(LayerCommand::MergeDown(id(1))));
    assert_eq!(
        refused.map(|applied| applied.change),
        Err(CoreError::Unmergeable {
            layer: id(1),
            why: Unmergeable::NothingBelow
        })
    );
}

#[test]
fn a_group_merges_within_and_its_bottom_layer_has_nothing_below() {
    let three = draw(&layer(&two(), LayerCommand::Create), 35.0);
    let grouped = apply(
        &three,
        ArtworkCommand::Group(GroupCommand::Create { layer: id(3) }),
    );
    let group = grouped.layers()[2].props().group();
    assert!(group.is_some());
    assert_eq!(grouped.layers()[1].props().group(), group);
    let merged = merge(&grouped);
    assert_eq!(merged.layers().len(), 2);
    assert_eq!(
        merged.layers()[1].props().group(),
        group,
        "still in the group"
    );
    assert_eq!(merged.groups().len(), 1);
    assert_eq!(
        merged.mergeable(merged.active()),
        Err(Unmergeable::NothingBelow)
    );
}

#[test]
fn records_nest_up_to_the_limit_and_a_copy_renumbers_them() {
    let mut artwork = two();
    let mut refused = false;
    for _ in 0..=MAX_NESTING {
        let faded = layer(
            &artwork,
            LayerCommand::SetOpacity {
                layer: id(1),
                opacity: 0.5,
            },
        );
        let upper = draw(
            &layer(
                &layer(&faded, LayerCommand::Select(id(1))),
                LayerCommand::Create,
            ),
            12.0,
        );
        match upper.mergeable(upper.active()) {
            Ok(()) => artwork = merge(&upper),
            Err(why) => {
                assert_eq!(why, Unmergeable::TooDeep);
                refused = true;
                break;
            }
        }
    }
    let depth = nested(&artwork, 0, 0).depth();
    assert!(refused, "one more than the limit refuses");
    assert_eq!(depth, MAX_NESTING, "the lower content's record");
    let copied = layer(&artwork, LayerCommand::Duplicate(id(1)));
    let (original, copy) = (nested(&copied, 0, 0), nested(&copied, 1, 0));
    assert_ne!(original.id(), copy.id());
    assert_eq!(copy.depth(), original.depth());
    // Every stroke and nested id unique across the artwork, or storage refuses it.
    let parts = copied.to_parts();
    assert_eq!(Artwork::from_storage(parts).expect("valid"), copied);
}

#[test]
fn stored_nested_records_round_trip_and_bad_ones_are_refused() {
    let masked = apply(&two(), ArtworkCommand::Mask(MaskCommand::Add(id(2))));
    let masked = draw(&masked, 8.0);
    let merged = merge(&layer(&clip(&masked, 2), LayerCommand::Select(id(2))));
    let parts = merged.to_parts();
    assert_eq!(Artwork::from_storage(parts.clone()).expect("valid"), merged);
    let with = |change: &dyn Fn(&mut openpaint_core::NestedParts)| {
        let mut broken = parts.clone();
        let EditParts::Nested(nested) = &mut broken.layers[0].edits[0] else {
            panic!("a nested record");
        };
        change(nested);
        Artwork::from_storage(broken)
    };
    assert!(
        matches!(with(&|n| n.id = 1), Err(CoreError::DuplicateId { .. })),
        "a layer's id"
    );
    assert!(with(&|n| n.opacity = 1.5).is_err(), "opacity");
    assert!(with(&|n| n.after = 9).is_err(), "past the strokes");
    let off = |n: &mut openpaint_core::NestedParts| {
        if let Some(mask) = n.mask.as_mut() {
            mask.enabled = false;
        }
    };
    assert!(with(&off).is_err(), "a disabled mask is never kept");
    // A mask holds no nested record.
    let mut broken = parts.clone();
    let record = broken.layers[0].edits[0].clone();
    broken.layers[0].mask = Some(openpaint_core::LayerMaskParts {
        enabled: true,
        inverted: false,
        strokes: Vec::new(),
        edits: vec![record],
    });
    assert!(Artwork::from_storage(broken).is_err());
}
