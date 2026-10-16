//! The routing's tests: everything the author's artworks hold, smudges included (S61), goes to the
//! raster core; each kind it does not draw yet sends the scene to the evaluator, named.

use std::sync::Arc;

use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, BlendMode, Blur, BrushRecipe, CanvasPoint, CanvasSize,
    Color, GroupCommand, Hsb, LayerCommand, MaskCommand, Patch, Placement, RegionMask,
    StrokeSample, StrokeStyle, StrokeTool,
};

use super::*;

const SIZE: u32 = 64;

fn sample(x: f32, y: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, 1.0, 0.0, 0.0, time_ms).expect("sample")
}

fn drawn(artwork: &Artwork, tool: StrokeTool) -> Artwork {
    let recipe = Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 6.0, 1.0, Color::BLACK)
        .expect("style")
        .with_tool(tool);
    let mut session = artwork
        .begin_stroke(style, sample(4.0, 4.0, 0))
        .expect("begin");
    session.extend(&[sample(40.0, 30.0, 8)]).expect("extend");
    session.finish().artwork
}

fn apply(artwork: &Artwork, command: ArtworkCommand) -> Artwork {
    artwork.apply(command).expect("command").artwork
}

fn square() -> RegionMask {
    let canvas = CanvasSize::new(SIZE, SIZE).expect("size");
    let pixels: Vec<bool> = (0..SIZE * SIZE)
        .map(|at| (10..30).contains(&(at % SIZE)) && (10..30).contains(&(at / SIZE)))
        .collect();
    RegionMask::from_pixels(canvas, &pixels).expect("mask")
}

/// Strokes (one erasing), a fill, a cut, a picture, a masked Screen layer and a layer clipping
/// to it by Overlay: all the raster core's.
fn plain() -> Artwork {
    let canvas = CanvasSize::new(SIZE, SIZE).expect("size");
    let mut artwork = drawn(&Artwork::new(canvas), StrokeTool::Paint);
    artwork = drawn(&artwork, StrokeTool::Erase);
    artwork = artwork
        .fill_region(square(), Color::WHITE)
        .expect("fill")
        .artwork;
    artwork = artwork.cut(Some(&square())).expect("cut").artwork;
    // A picture on a layer of its own (its patch replaces what its layer held).
    artwork = apply(&artwork, ArtworkCommand::Layer(LayerCommand::Create));
    let patch = Patch::new(canvas, [2, 2], [4, 4], vec![200; 64]).expect("patch");
    let moved = Placement::moved([3.0, 1.0]).expect("move");
    artwork = (artwork.transform_region(None, patch, moved))
        .expect("picture")
        .artwork;
    artwork = apply(&artwork, ArtworkCommand::Layer(LayerCommand::Create));
    let base = artwork.active();
    artwork = apply(&artwork, ArtworkCommand::Mask(MaskCommand::Add(base)));
    artwork = drawn(&artwork, StrokeTool::Paint);
    let screen = LayerCommand::SetBlend {
        layer: base,
        blend: BlendMode::Screen,
    };
    artwork = apply(&artwork, ArtworkCommand::Layer(screen));
    artwork = apply(&artwork, ArtworkCommand::Layer(LayerCommand::Select(base)));
    artwork = drawn(&artwork, StrokeTool::Paint);
    artwork = apply(&artwork, ArtworkCommand::Layer(LayerCommand::Create));
    let top = artwork.active();
    let clip = LayerCommand::SetClipping {
        layer: top,
        clipping: true,
    };
    let overlay = LayerCommand::SetBlend {
        layer: top,
        blend: BlendMode::Overlay,
    };
    artwork = apply(&artwork, ArtworkCommand::Layer(clip));
    artwork = apply(&artwork, ArtworkCommand::Layer(overlay));
    drawn(&artwork, StrokeTool::Paint)
}

fn routed(artwork: &Artwork) -> Result<RasterScene, Routed> {
    RasterScene::of(&Scene::build(artwork, None).expect("scene"))
}

#[test]
fn what_the_authors_artworks_hold_goes_to_the_raster_core() {
    let scene = routed(&plain()).expect("rastered");
    let ends = (scene.units.iter())
        .filter(|unit| matches!(unit, Unit::End(_)))
        .count();
    let masks = (scene.units.iter())
        .filter(|unit| matches!(unit, Unit::BeginMask))
        .count();
    assert_eq!((ends, masks), (5, 1), "four layers and a chain, a mask");
    let edits = (scene.units.iter())
        .filter(|unit| matches!(unit, Unit::Edit { .. }))
        .count();
    assert_eq!(edits, 3, "the fill, the cut, the picture");
}

/// S61: a smudge on the paint and one on the mask are units of their own, in scene order, each
/// naming its op, its number among the scene's smudges and the canvas pixels it can change.
#[test]
fn smudges_on_paint_and_on_a_mask_are_units_in_scene_order() {
    let base = plain();
    let layer = base.active();
    let on_paint = drawn(&base, StrokeTool::Smudge);
    let on_mask = apply(&on_paint, ArtworkCommand::Mask(MaskCommand::Add(layer)));
    let both = drawn(&on_mask, StrokeTool::Smudge);
    let built = Scene::build(&both, None).expect("scene");
    let routed = RasterScene::of(&built).expect("rastered");
    let smudges: Vec<(usize, usize, PixelRect)> = (routed.units.iter())
        .filter_map(|unit| match unit {
            Unit::Smudge {
                item,
                smudge,
                reach,
            } => Some((*item, *smudge, *reach)),
            _ => None,
        })
        .collect();
    assert_eq!(smudges.len(), 2);
    for (number, (item, smudge, reach)) in smudges.iter().enumerate() {
        assert_eq!(*smudge, number);
        assert_eq!(built.smudges[number].item, *item);
        assert!(matches!(built.items[*item], Item::Smudge(_)));
        assert_eq!(*reach, built.smudges[number].canvas);
        assert!(!reach.is_empty() && reach.min[0] <= 4 && reach.max[0] >= 40);
    }
    assert!(!built.smudges[0].on_mask && built.smudges[1].on_mask);
    let mask_at = (routed.units.iter())
        .position(|unit| matches!(unit, Unit::BeginMask))
        .expect("the mask");
    let second = (routed.units.iter())
        .position(|unit| matches!(unit, Unit::Smudge { smudge: 1, .. }))
        .expect("the mask's smudge");
    assert!(second > mask_at, "the mask's smudge after its start");
}

#[test]
fn each_kind_the_raster_core_lacks_sends_the_scene_to_the_evaluator_named() {
    let base = plain();
    let layer = base.active();
    let blur = Adjustment::Blur(Blur::new(3.0));
    let hsb = Adjustment::Hsb(Hsb::new(0.7, 0.5, 0.5));
    let moved = Placement::moved([5.0, 2.0]).expect("move");
    let canvas = CanvasSize::new(SIZE, SIZE).expect("size");
    let one = drawn(&Artwork::new(canvas), StrokeTool::Paint);
    let below = apply(&one, ArtworkCommand::Layer(LayerCommand::Create));
    let cases: [(Reason, Artwork); 6] = [
        (
            Reason::Adjustment,
            base.adjust_region(None, hsb).expect("adjust").artwork,
        ),
        (
            Reason::Filter,
            base.adjust_region(None, blur.clone())
                .expect("blur")
                .artwork,
        ),
        (
            Reason::Group,
            apply(&base, ArtworkCommand::Group(GroupCommand::Create { layer })),
        ),
        (
            Reason::Transform,
            base.place_region(None, moved).expect("record").artwork,
        ),
        (
            Reason::MovedSelection,
            (base.place_region(Some(square()), moved))
                .expect("moved")
                .artwork,
        ),
        (
            Reason::Nested,
            apply(
                &drawn(&below, StrokeTool::Paint),
                ArtworkCommand::Layer(LayerCommand::MergeDown(below.active())),
            ),
        ),
    ];
    for (reason, artwork) in cases {
        let routed = routed(&artwork).expect_err("evaluated");
        assert_eq!(routed.reasons(), [reason], "{routed}");
    }
    let grouped = apply(&base, ArtworkCommand::Group(GroupCommand::Create { layer }));
    let blurred = (grouped.adjust_region(None, blur)).expect("blur").artwork;
    let routed = routed(&drawn(&blurred, StrokeTool::Smudge)).expect_err("evaluated");
    assert_eq!(
        routed.to_string(),
        "filters, groups",
        "a smudge is no reason"
    );
}
