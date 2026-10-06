//! Copy and paste (S40): a copy is records, never pixels: the active layer's content as a nested
//! record at full opacity by Normal (its enabled mask kept), or for Copy All one record per
//! clipping chain of the visible layers, a faded group a record of its own; inside a selection
//! the pixels outside it cleared. Paste renumbers everything; a cut clears the selection or
//! empties the layer. Refusals, storage and history bytes.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError,
    GroupCommand, LayerCommand, MaskCommand, NestedLayer, RegionEdit, RegionMask, StrokeSample,
    StrokeStyle,
};

fn canvas() -> CanvasSize {
    CanvasSize::new(64, 48).expect("size")
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

/// Layer 1 with a stroke, layer 2 above it at 0.5 by Multiply with a stroke and a mask, active.
fn two() -> Artwork {
    let lower = draw(&Artwork::new(canvas()), 10.0);
    let upper = draw(&layer(&lower, LayerCommand::Create), 30.0);
    let id = upper.active();
    let upper = layer(
        &upper,
        LayerCommand::SetOpacity {
            layer: id,
            opacity: 0.5,
        },
    );
    let upper = layer(
        &upper,
        LayerCommand::SetBlend {
            layer: id,
            blend: BlendMode::Multiply,
        },
    );
    let masked = apply(&upper, ArtworkCommand::Mask(MaskCommand::Add(id)));
    layer(&masked, LayerCommand::Select(id))
}

fn nested(edit: &RegionEdit) -> &NestedLayer {
    edit.nested().expect("a nested record").layer()
}

fn rectangle() -> RegionMask {
    let inside: Vec<bool> = (0..64 * 48)
        .map(|at| (8..30).contains(&(at % 64)))
        .collect();
    RegionMask::from_pixels(canvas(), &inside).expect("mask")
}

#[test]
fn a_copy_is_the_layers_content_at_full_opacity_and_a_selection_clears_the_rest() {
    let artwork = two();
    let source = artwork.layer(artwork.active()).expect("layer");
    let whole = artwork.copied(false, None).expect("copy");
    assert_eq!(whole.canvas(), canvas());
    let [part] = whole.edits() else {
        panic!("one record: {:?}", whole.edits().len());
    };
    let part = nested(part);
    assert_eq!(
        (part.opacity(), part.blend(), part.clipped()),
        (1.0, BlendMode::Normal, false)
    );
    assert!(
        part.strokes().same(source.strokes()),
        "the very strokes, shared"
    );
    assert_eq!(part.mask(), source.mask(), "its mask applies");
    let inside = artwork.copied(false, Some(&rectangle())).expect("copy");
    let [_, cleared] = inside.edits() else {
        panic!("a record and a clear");
    };
    let outside = cleared.clear().expect("cleared pixels").mask();
    assert_eq!(*outside, rectangle().inverted(canvas()).expect("outside"));
}

#[test]
fn pasting_puts_the_copy_on_the_active_layer_under_new_ids_any_number_of_times() {
    let artwork = two();
    let copied = artwork.copied(false, Some(&rectangle())).expect("copy");
    let created = layer(&artwork, LayerCommand::Create);
    let once = created.paste(&copied).expect("paste").artwork;
    let twice = layer(&once, LayerCommand::Create)
        .paste(&copied)
        .expect("paste")
        .artwork;
    let ids = |artwork: &Artwork| {
        let pasted = artwork.layer(artwork.active()).expect("layer");
        let record = nested(&pasted.edits()[0]);
        (record.id(), record.strokes().get(0).expect("stroke").id())
    };
    assert_ne!(ids(&once), ids(&twice), "new ids each time");
    let source = artwork.layer(artwork.active()).expect("source");
    assert_ne!(ids(&once).1, source.strokes().get(0).expect("stroke").id());
    // Stored and reloaded, the ids are valid and the artwork equal.
    let reloaded = Artwork::from_storage(twice.to_parts()).expect("valid");
    assert_eq!(reloaded.to_parts(), twice.to_parts());
    assert!(twice.bytes_since(&once) < 4096, "records, not pixels");
    let other = Artwork::new(CanvasSize::new(65, 48).expect("size"));
    assert!(matches!(
        other.paste(&copied),
        Err(CoreError::InvalidValue { .. })
    ));
    let locked = layer(
        &created,
        LayerCommand::SetLocked {
            layer: created.active(),
            locked: true,
        },
    );
    assert!(matches!(
        locked.paste(&copied),
        Err(CoreError::LayerLocked(_))
    ));
}

#[test]
fn copy_all_takes_each_visible_chain_and_a_faded_group_as_records() {
    let artwork = two();
    let lower = artwork.layers()[0].id();
    let clipped = layer(
        &artwork,
        LayerCommand::SetClipping {
            layer: artwork.active(),
            clipping: true,
        },
    );
    let all = clipped.copied(true, None).expect("copy all");
    let [chain] = all.edits() else {
        panic!("one chain");
    };
    let chain = nested(chain);
    assert_eq!(
        (chain.opacity(), chain.blend()),
        (1.0, BlendMode::Normal),
        "the base's look"
    );
    let [base, clip] = chain.edits() else {
        panic!("the base and its clip");
    };
    assert!(!nested(base).clipped() && nested(clip).clipped());
    assert_eq!(
        (nested(clip).opacity(), nested(clip).blend()),
        (0.5, BlendMode::Multiply)
    );
    // Two loose layers are two records; a hidden one is none.
    assert_eq!(
        artwork.copied(true, None).expect("copy all").edits().len(),
        2
    );
    let hidden = layer(
        &artwork,
        LayerCommand::SetVisible {
            layer: lower,
            visible: false,
        },
    );
    assert_eq!(
        hidden.copied(true, None).expect("copy all").edits().len(),
        1
    );
}

#[test]
fn copy_all_takes_a_faded_group_as_one_record_and_a_full_one_as_its_layers() {
    let artwork = two();
    // A faded group is a record by the group's opacity holding its layers'.
    let grouped = apply(
        &artwork,
        ArtworkCommand::Group(GroupCommand::Create {
            layer: artwork.active(),
        }),
    );
    let group = grouped.groups()[0].id();
    let faded = apply(
        &grouped,
        ArtworkCommand::Group(GroupCommand::SetOpacity {
            group,
            opacity: 0.25,
        }),
    );
    let all = faded.copied(true, None).expect("copy all");
    let [group] = all.edits() else {
        panic!("one group");
    };
    assert_eq!(nested(group).opacity(), 0.25);
    assert_eq!(nested(group).edits().len(), 2);
    let full = grouped.copied(true, None).expect("copy all");
    assert_eq!(
        full.edits().len(),
        2,
        "a group at full opacity is its layers"
    );
}

#[test]
fn a_cut_clears_the_selection_or_empties_the_layer_and_nothing_refuses() {
    let artwork = two();
    let id = artwork.active();
    let cut = artwork.cut(Some(&rectangle())).expect("cut").artwork;
    let layer_now = cut.layer(id).expect("layer");
    let last = layer_now.edits().last().expect("an edit");
    assert_eq!(last.clear().expect("a clear").mask(), &rectangle());
    assert_eq!(last.after(), layer_now.strokes().len());
    assert!(layer_now.mask().is_some(), "the mask stays");
    let emptied = artwork.cut(None).expect("cut").artwork;
    let empty = emptied.layer(id).expect("layer");
    assert!(empty.strokes().is_empty() && empty.edits().is_empty());
    assert_eq!(emptied.cut(None).map(|_| ()), Err(CoreError::NothingToCopy));
    assert_eq!(
        emptied.copied(false, None).map(|_| ()),
        Err(CoreError::NothingToCopy)
    );
    assert!(!emptied.copyable(false) && emptied.copyable(true));
    let blank = Artwork::new(canvas());
    assert!(!blank.copyable(true));
    let locked = layer(
        &artwork,
        LayerCommand::SetLocked {
            layer: id,
            locked: true,
        },
    );
    assert_eq!(
        locked.cut(None).map(|_| ()),
        Err(CoreError::LayerLocked(id))
    );
    let reloaded = Artwork::from_storage(cut.to_parts()).expect("valid");
    assert_eq!(
        reloaded.to_parts(),
        cut.to_parts(),
        "a clear stores and reloads"
    );
}

#[test]
fn records_nested_to_the_limit_are_not_copied_deeper() {
    // Each merge over a faded lower layer nests its content once more: eight deep after eight.
    let mut artwork = draw(&Artwork::new(canvas()), 10.0);
    for row in 0..8 {
        let lower = artwork.active();
        let faded = layer(
            &artwork,
            LayerCommand::SetOpacity {
                layer: lower,
                opacity: 0.9,
            },
        );
        artwork = draw(&layer(&faded, LayerCommand::Create), 12.0 + row as f32);
        artwork = layer(&artwork, LayerCommand::MergeDown(artwork.active()));
    }
    let records = artwork.layers()[0]
        .edits()
        .iter()
        .filter_map(RegionEdit::nested);
    let depth = records.map(|n| n.layer().depth()).max().unwrap_or(0);
    assert_eq!(depth, 8);
    assert_eq!(
        artwork.copied(false, None).map(|_| ()),
        Err(CoreError::TooDeep)
    );
    assert!(!artwork.copyable(false) && !artwork.copyable(true));
}
