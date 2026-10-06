//! Copy and paste on the GPU (S40): a pasted copy is records (nested records, the pixels outside
//! a selection cleared), and at its place it shows exactly what was copied: the layer's paint
//! through its mask, cut to the selection bit for bit; what every visible layer showed (Copy
//! All) within one 8-bit level over the paper. A cut clears its selection exactly; a pasted
//! layer moved by a transform record equals a duplicate moved alike; incremental equals cold
//! through paste, move, cut and undo, and a reload.

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, GroupCommand, LayerCommand, LayerId, Placement,
    RegionMask,
};
use openpaint_render::{CanvasTarget, PixelRect, RegionEvaluator, RgbaImage, changed_regions};

use crate::fixture::{SIZE, cold, core, request, with_layer};
use crate::merge::{Look, PLAIN, alone};

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    artwork
        .apply(ArtworkCommand::Layer(command))
        .expect("command")
        .artwork
}

/// A rectangle and a triangle inside it cut off, as a lasso would leave a slanted edge.
fn selection() -> RegionMask {
    let inside: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| {
            let (x, y) = (at % SIZE[0], at / SIZE[0]);
            (60..250).contains(&x) && (30..170).contains(&y) && x + y > 150
        })
        .collect();
    let canvas = CanvasSize::new(SIZE[0], SIZE[1]).expect("size");
    RegionMask::from_pixels(canvas, &inside).expect("mask")
}

/// `artwork` with what it copies (`all`, inside `clip`) pasted on a new layer above the active
/// one, which becomes active.
fn pasted(artwork: &Artwork, all: bool, clip: Option<&RegionMask>) -> Artwork {
    let copied = artwork.copied(all, clip).expect("copy");
    with_layer(artwork, 1.0)
        .paste(&copied)
        .expect("paste")
        .artwork
}

/// `artwork` with every layer but `shown` hidden.
fn only(artwork: &Artwork, shown: LayerId) -> Artwork {
    let ids: Vec<LayerId> = artwork.layers().iter().map(|l| l.id()).collect();
    ids.into_iter().fold(artwork.clone(), |out, id| {
        let visible = id == shown;
        layer(&out, LayerCommand::SetVisible { layer: id, visible })
    })
}

/// The largest difference of any channel.
fn largest(a: &RgbaImage, b: &RgbaImage) -> u8 {
    let pairs = a.pixels().iter().zip(b.pixels());
    pairs.map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

#[test]
fn a_pasted_layer_shows_the_copied_layer_through_its_mask_at_full_opacity() {
    let core = core();
    let upper = Look {
        opacity: 0.6,
        blend: BlendMode::Multiply,
        clip: true,
        mask: true,
    };
    let artwork = alone(PLAIN, upper);
    let source = artwork.active();
    let after = pasted(&artwork, false, None);
    let paste = after.active();
    // The source alone at full opacity, by Normal, unclipped: what the copy holds.
    let mut reference = only(&after, source);
    for command in [
        LayerCommand::SetOpacity {
            layer: source,
            opacity: 1.0,
        },
        LayerCommand::SetBlend {
            layer: source,
            blend: BlendMode::Normal,
        },
        LayerCommand::SetClipping {
            layer: source,
            clipping: false,
        },
    ] {
        reference = layer(&reference, command);
    }
    assert!(
        cold(&core, &only(&after, paste)) == cold(&core, &reference),
        "masked copy"
    );
    let plain = alone(PLAIN, PLAIN);
    let copied = pasted(&plain, false, None);
    let original = core.layer_pixels(&plain, plain.active()).expect("pixels");
    let copy = core.layer_pixels(&copied, copied.active()).expect("pixels");
    assert!(copy == original, "the paint alone, bit for bit");
}

#[test]
fn a_copy_inside_a_selection_holds_exactly_its_pixels_and_a_cut_takes_exactly_them() {
    let core = core();
    let artwork = alone(PLAIN, PLAIN);
    let mask = selection();
    let original = core
        .layer_pixels(&artwork, artwork.active())
        .expect("pixels");
    let after = pasted(&artwork, false, Some(&mask));
    let copy = core.layer_pixels(&after, after.active()).expect("pixels");
    let cut = artwork.cut(Some(&mask)).expect("cut").artwork;
    let left = core.layer_pixels(&cut, cut.active()).expect("pixels");
    for (index, pixel) in original.pixels().chunks(4).enumerate() {
        let at = [index as u32 % SIZE[0], index as u32 / SIZE[0]];
        let (inside, outside) = if mask.contains(at) {
            (pixel, &[0u8; 4][..])
        } else {
            (&[0u8; 4][..], pixel)
        };
        assert_eq!(&copy.pixels()[index * 4..][..4], inside, "copy at {at:?}");
        assert_eq!(&left.pixels()[index * 4..][..4], outside, "cut at {at:?}");
    }
}

#[test]
fn copy_all_pasted_over_the_paper_shows_what_every_visible_layer_showed() {
    let core = core();
    let lower = Look {
        opacity: 0.6,
        mask: true,
        ..PLAIN
    };
    let upper = Look {
        opacity: 0.7,
        blend: BlendMode::Multiply,
        clip: true,
        mask: false,
    };
    let chain = alone(lower, upper);
    // The same two layers in a group faded to 0.5, and a hidden layer on top.
    let group = chain
        .apply(ArtworkCommand::Group(GroupCommand::Create {
            layer: chain.active(),
        }))
        .expect("group")
        .artwork;
    let id = group
        .layer(group.active())
        .and_then(|l| l.props().group())
        .expect("its group");
    let faded = group
        .apply(ArtworkCommand::Group(GroupCommand::SetOpacity {
            group: id,
            opacity: 0.5,
        }))
        .expect("fade")
        .artwork;
    let hidden = with_layer(&faded, 1.0);
    let hidden = crate::fixture::with_stroke(&hidden, crate::fixture::wave());
    let hidden = layer(
        &hidden,
        LayerCommand::SetVisible {
            layer: hidden.active(),
            visible: false,
        },
    );
    for (what, artwork) in [("chain", chain), ("faded group", hidden)] {
        // Pasted above the bottom layer, outside the group (a layer pasted into a faded group
        // would be faded once more, as any layer there).
        let bottom = artwork.layers()[0].id();
        let after = pasted(&layer(&artwork, LayerCommand::Select(bottom)), true, None);
        let shown = cold(&core, &only(&after, after.active()));
        let level = largest(&shown, &cold(&core, &artwork));
        assert!(level <= 1, "{what}: {level} levels off");
    }
}

#[test]
fn a_pasted_layer_moved_by_a_record_equals_a_duplicate_moved_alike() {
    let core = core();
    let artwork = alone(PLAIN, PLAIN);
    let placement = Placement::around([160.0, 100.0], 1.3, 0.4, [false, true]).expect("place");
    let copied = pasted(&artwork, false, None);
    let moved = copied.place_region(None, placement).expect("move").artwork;
    let twin = layer(&artwork, LayerCommand::Duplicate(artwork.active()));
    let twin = twin.place_region(None, placement).expect("move").artwork;
    let a = core.layer_pixels(&moved, moved.active()).expect("pixels");
    let b = core.layer_pixels(&twin, twin.active()).expect("pixels");
    assert!(a == b, "moved alike");
}

#[test]
fn paste_move_cut_and_undo_through_changed_regions_equal_cold() {
    let core = core();
    let base = alone(
        PLAIN,
        Look {
            mask: true,
            ..PLAIN
        },
    );
    let mask = selection();
    let pasted = pasted(&base, false, Some(&mask));
    let placement = Placement::moved([40.0, -15.0]).expect("move");
    let moved = pasted.place_region(None, placement).expect("move").artwork;
    let source = layer(&moved, LayerCommand::Select(base.active()));
    let cut = source.cut(Some(&mask)).expect("cut").artwork;
    let all = with_layer(&cut, 1.0).paste(&cut.copied(true, None).expect("copy all"));
    let all = all.expect("paste").artwork;
    let steps = [
        base.clone(),
        pasted,
        moved,
        cut.clone(),
        all.clone(),
        cut,
        base,
    ];
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&steps[0]), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    for pair in steps.windows(2) {
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&pair[1]), &changed_regions(&pair[0], &pair[1]))
            .expect("evaluate");
        assert!(
            target.read(&core).expect("readback") == cold(&core, &pair[1]),
            "a step"
        );
    }
    let reloaded = Artwork::from_storage(all.to_parts()).expect("reload");
    assert!(cold(&core, &reloaded) == cold(&core, &all), "reopened");
}
