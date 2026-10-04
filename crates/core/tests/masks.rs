//! Layer masks in core (S24): commands and their refusals, the target, strokes and fills in the
//! mask's grey (the eraser white, an inverted mask the opposite grey), and storage checks.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Change, Color, CoreError,
    LayerCommand, LayerId, MaskCommand, RegionMask, StrokeBlend, StrokeSample, StrokeStyle,
    StrokeTool, grey_of,
};

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(64, 48).expect("size"))
}

fn mask(artwork: &Artwork, command: MaskCommand) -> Result<Artwork, CoreError> {
    Ok(artwork.apply(ArtworkCommand::Mask(command))?.artwork)
}

fn draw(artwork: &Artwork, tool: StrokeTool, color: Color) -> Artwork {
    let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 6.0, 1.0, color)
        .expect("style")
        .with_tool(tool);
    let point = CanvasPoint::new(10.0, 10.0).expect("point");
    let first = StrokeSample::new(point, 1.0, 0.0, 0.0, 0).expect("sample");
    artwork
        .begin_stroke(style, first)
        .expect("begin")
        .finish()
        .artwork
}

const LAYER: LayerId = LayerId::new(1);

#[test]
fn adding_a_mask_makes_it_the_target_and_choosing_the_paint_is_no_edit() {
    let added = mask(&blank(), MaskCommand::Add(LAYER)).expect("add");
    assert!(added.on_mask());
    let white = added
        .layer(LAYER)
        .and_then(|layer| layer.mask())
        .expect("mask");
    assert!(white.enabled() && white.is_white());
    let again = added
        .apply(ArtworkCommand::Mask(MaskCommand::Add(LAYER)))
        .expect("again");
    assert_eq!(again.change, Change::None, "one mask a layer");
    let select = LayerCommand::Select(LAYER);
    let paint = added.apply(ArtworkCommand::Layer(select)).expect("paint");
    assert_eq!(paint.change, Change::ActiveLayer);
    assert!(!paint.artwork.on_mask());
    let back = paint
        .artwork
        .apply(ArtworkCommand::Mask(MaskCommand::Select(LAYER)));
    assert_eq!(back.expect("mask").change, Change::ActiveLayer);
    let create = ArtworkCommand::Layer(LayerCommand::Create);
    let other = added.apply(create).expect("create").artwork;
    assert!(!other.on_mask(), "a new layer's paint is the target");
}

#[test]
fn strokes_and_fills_on_a_mask_store_grey_the_eraser_white_inverted_the_opposite() {
    let added = mask(&blank(), MaskCommand::Add(LAYER)).expect("add");
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    let painted = draw(&added, StrokeTool::Paint, red);
    let erased = draw(&painted, StrokeTool::Erase, red);
    let inverted = mask(&erased, MaskCommand::Invert(LAYER)).expect("invert");
    let after = draw(&inverted, StrokeTool::Paint, Color::BLACK);
    let layer = after.layer(LAYER).expect("layer");
    assert!(layer.strokes().is_empty(), "the paint is untouched");
    let strokes: Vec<_> = layer.mask().expect("mask").strokes().iter().collect();
    let level = |index: usize| strokes[index].style().color().r();
    assert_eq!(level(0), grey_of(red).r());
    assert_eq!(level(0), 76.0 / 255.0, "red's luma, 0.299");
    assert_eq!(
        (level(1), strokes[1].blend()),
        (1.0, StrokeBlend::Over),
        "erasing reveals"
    );
    assert_eq!(level(2), 1.0, "black on an inverted mask stores white");
    let mut inside = vec![false; 64 * 48];
    inside[100] = true;
    let region = RegionMask::from_pixels(after.canvas(), &inside).expect("region");
    let filled = after
        .fill_region(region, Color::WHITE)
        .expect("fill")
        .artwork;
    let edit = &filled
        .layer(LAYER)
        .expect("layer")
        .mask()
        .expect("mask")
        .edits()[0];
    let fill = edit.fill().expect("a fill");
    assert_eq!(
        (fill.color().r(), fill.after()),
        (0.0, 3),
        "white inverted, after 3"
    );
    let smudged = draw(&filled, StrokeTool::Smudge, red);
    let last = smudged
        .layer(LAYER)
        .and_then(|l| l.mask())
        .and_then(|m| m.strokes().last());
    assert_eq!(last.expect("smudge").blend(), StrokeBlend::Smudge);
}

#[test]
fn an_off_mask_is_no_target_and_refuses_invert_clear_and_choosing() {
    let added = mask(&blank(), MaskCommand::Add(LAYER)).expect("add");
    let off = MaskCommand::SetEnabled {
        layer: LAYER,
        enabled: false,
    };
    let disabled = mask(&added, off).expect("off");
    assert!(!disabled.on_mask(), "its paint is the target again");
    assert_eq!(
        mask(&disabled, MaskCommand::Invert(LAYER)),
        Err(CoreError::MaskOff(LAYER))
    );
    assert_eq!(
        mask(&disabled, MaskCommand::Clear(LAYER)),
        Err(CoreError::MaskOff(LAYER))
    );
    assert_eq!(
        mask(&disabled, MaskCommand::Select(LAYER)),
        Err(CoreError::MaskOff(LAYER))
    );
    let deleted = mask(&disabled, MaskCommand::Delete(LAYER)).expect("delete");
    assert!(
        deleted
            .layer(LAYER)
            .is_some_and(|layer| layer.mask().is_none())
    );
    assert_eq!(
        mask(&deleted, MaskCommand::Delete(LAYER)),
        Err(CoreError::NoMask(LAYER))
    );
    let transform = openpaint_core::Patch::new(added.canvas(), [0, 0], [1, 1], vec![255; 4]);
    let placement = openpaint_core::Placement::moved([1.0, 0.0]).expect("placement");
    let refused = added.transform_region(None, transform.expect("patch"), placement);
    assert_eq!(refused.map(|_| ()), Err(CoreError::OnMask(LAYER)));
}

#[test]
fn clearing_goes_back_to_white_and_storage_checks_the_mask() {
    let added = mask(&blank(), MaskCommand::Add(LAYER)).expect("add");
    let painted = mask(
        &draw(&added, StrokeTool::Paint, Color::BLACK),
        MaskCommand::Invert(LAYER),
    );
    let cleared = mask(&painted.expect("invert"), MaskCommand::Clear(LAYER)).expect("clear");
    assert!(
        cleared
            .layer(LAYER)
            .and_then(|l| l.mask())
            .is_some_and(|m| m.is_white())
    );
    let drawn = draw(&added, StrokeTool::Paint, Color::BLACK);
    let parts = drawn.to_parts();
    assert_eq!(Artwork::from_storage(parts.clone()).expect("reload"), drawn);
    let mut coloured = parts.clone();
    if let Some(mask) = coloured.layers[0].mask.as_mut() {
        mask.strokes[0].color = [1.0, 0.0, 0.0];
    }
    assert!(
        Artwork::from_storage(coloured).is_err(),
        "mask strokes are grey"
    );
    let mut no_mask = parts;
    no_mask.layers[0].mask = None;
    assert_eq!(
        Artwork::from_storage(no_mask),
        Err(CoreError::NoMask(LAYER))
    );
}
