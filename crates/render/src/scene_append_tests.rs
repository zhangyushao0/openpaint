//! A scene taking appended strokes in place equals the scene built anew (S64): its items, raster
//! units, masks, the place, its smudges and where its strokes' spans lie; for strokes on a layer
//! under others (a mask, smudges and a clipping chain above, a fill and smudges under), several
//! at once, on a mask, inside a group (S65); and it takes none of a smudge or a clipped stroke.

use std::sync::Arc;

use openpaint_core::{
    ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, GroupCommand, LayerCommand,
    MaskCommand, RegionMask, StrokeSample, StrokeStyle, StrokeTool,
};

use super::*;
use crate::scene::ActivePlace;

const SIZE: u32 = 96;

fn sample(x: f32, y: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, 0.8, 0.0, 0.0, time_ms).expect("sample")
}

fn style(tool: StrokeTool) -> StrokeStyle {
    let recipe = Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 8.0, 0.9, Color::BLACK).expect("style");
    style.with_tool(tool)
}

/// `artwork` with a stroke of `tool` from `at` (inside `clip`).
fn drawn(artwork: &Artwork, tool: StrokeTool, at: f32, clip: Option<&RegionMask>) -> Artwork {
    let mut session = artwork
        .begin_stroke(style(tool), sample(at, at, 0))
        .expect("begin");
    if let Some(clip) = clip {
        session = session.clipped(clip.clone()).expect("clipped");
    }
    let path = [
        sample(at + 20.0, at + 9.0, 8),
        sample(at + 40.0, at + 3.0, 16),
    ];
    session.extend(&path).expect("extend");
    session.finish().artwork
}

fn applied(artwork: &Artwork, command: ArtworkCommand) -> Artwork {
    artwork.apply(command).expect("command").artwork
}

fn square(from: u32) -> RegionMask {
    let canvas = CanvasSize::new(SIZE, SIZE).expect("size");
    let pixels: Vec<bool> = (0..SIZE * SIZE)
        .map(|at| {
            (from..from + 30).contains(&(at % SIZE)) && (from..from + 30).contains(&(at / SIZE))
        })
        .collect();
    RegionMask::from_pixels(canvas, &pixels).expect("mask")
}

/// Four layers: a fill and a smudge on the first; strokes ending in a smudge on the second (the
/// place); strokes, a smudge and a masked mask's strokes on the third; a fourth clipping to it.
fn busy() -> Artwork {
    let mut artwork = Artwork::new(CanvasSize::new(SIZE, SIZE).expect("size"));
    artwork = drawn(&artwork, StrokeTool::Paint, 5.0, None);
    artwork = artwork
        .fill_region(square(10), Color::BLACK)
        .expect("fill")
        .artwork;
    artwork = drawn(&artwork, StrokeTool::Smudge, 12.0, None);
    artwork = applied(&artwork, ArtworkCommand::Layer(LayerCommand::Create));
    let place = artwork.active();
    for at in [20.0, 30.0] {
        artwork = drawn(&artwork, StrokeTool::Paint, at, None);
    }
    artwork = drawn(&artwork, StrokeTool::Smudge, 25.0, None);
    artwork = applied(&artwork, ArtworkCommand::Layer(LayerCommand::Create));
    let third = artwork.active();
    artwork = drawn(&artwork, StrokeTool::Paint, 15.0, None);
    artwork = drawn(&artwork, StrokeTool::Smudge, 18.0, None);
    artwork = applied(&artwork, ArtworkCommand::Mask(MaskCommand::Add(third)));
    artwork = drawn(&artwork, StrokeTool::Paint, 35.0, None);
    artwork = applied(&artwork, ArtworkCommand::Layer(LayerCommand::Create));
    artwork = drawn(&artwork, StrokeTool::Paint, 40.0, None);
    let clipping = LayerCommand::SetClipping {
        layer: artwork.active(),
        clipping: true,
    };
    artwork = applied(&artwork, ArtworkCommand::Layer(clipping));
    applied(&artwork, ArtworkCommand::Layer(LayerCommand::Select(place)))
}

/// The scene of `before` with what `after` appends on its place taken in place, the items it
/// took, and the scene of `after` built.
fn both(before: &Artwork, after: &Artwork) -> (Scene, Option<Range<usize>>, Scene) {
    let place = ActivePlace::of_request(before, None);
    let mut kept = Scene::build(before, Some(&place)).expect("scene");
    let at = (place.layer, place.on_mask);
    assert!(appended_over(before, after, at));
    let took = kept.append(&appended(before, after, at));
    let built = Scene::build(after, Some(&place)).expect("scene");
    (kept, took, built)
}

/// The scene taking strokes in place is the scene built.
fn same(before: &Artwork, after: &Artwork) {
    let (kept, took, built) = both(before, after);
    assert!(took.is_some(), "taken in place");
    assert_eq!(kept.items, built.items);
    assert_eq!(kept.raster, built.raster);
    assert_eq!(kept.masks, built.masks);
    assert_eq!(
        (kept.active_before.clone(), kept.active_clip),
        (built.active_before.clone(), built.active_clip)
    );
    assert_eq!(kept.spans.ranges(), built.spans.ranges());
    let smudge = |scene: &Scene| -> Vec<_> {
        (scene.smudges.iter())
            .map(|s| {
                (
                    s.layer,
                    s.on_mask,
                    s.item,
                    s.before.clone(),
                    s.newest,
                    s.clip,
                    s.canvas,
                )
            })
            .collect()
    };
    assert_eq!(smudge(&kept), smudge(&built));
}

#[test]
fn strokes_appended_under_others_are_the_scene_built() {
    let before = busy();
    let one = drawn(&before, StrokeTool::Paint, 50.0, None);
    same(&before, &one);
    // Two at once (a stroke and its mirrored copy), an eraser among them.
    let two = drawn(&one, StrokeTool::Erase, 8.0, None);
    same(&before, &two);
    // Strokes on the third layer's mask, under the chain clipping to it.
    let third = before.layers()[2].id();
    let on_mask = applied(&before, ArtworkCommand::Mask(MaskCommand::Select(third)));
    same(&on_mask, &drawn(&on_mask, StrokeTool::Paint, 44.0, None));
}

/// S65: a place inside a group takes strokes as one on the canvas does.
#[test]
fn strokes_appended_inside_a_group_are_the_scene_built() {
    let before = busy();
    let grouped = applied(
        &before,
        ArtworkCommand::Group(GroupCommand::Create {
            layer: before.active(),
        }),
    );
    same(&grouped, &drawn(&grouped, StrokeTool::Paint, 50.0, None));
}

#[test]
fn a_smudge_or_a_clipped_stroke_is_not_taken() {
    let before = busy();
    let smudged = drawn(&before, StrokeTool::Smudge, 50.0, None);
    assert_eq!(both(&before, &smudged).1, None);
    let clipped = drawn(&before, StrokeTool::Paint, 50.0, Some(&square(40)));
    let (kept, took, _) = both(&before, &clipped);
    assert_eq!(took, None);
    let place = ActivePlace::of_request(&before, None);
    let unchanged = Scene::build(&before, Some(&place)).expect("scene");
    assert_eq!(kept.items, unchanged.items);
}
