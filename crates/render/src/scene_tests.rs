//! The scene's tests: drawing order, shared clips, hidden layers, fresh ops and smudges.

use std::sync::Arc;

use super::*;
use crate::stroke::stroke_region;
use openpaint_core::{
    ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, GroupCommand, LayerCommand,
};

fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
}

fn style() -> StrokeStyle {
    let recipe = Arc::new(BrushRecipe::plain());
    StrokeStyle::new(recipe, 10.0, 1.0, Color::BLACK).expect("style")
}

fn codes(scene: &Scene) -> Vec<&'static str> {
    let name = |item: &Item| match item {
        Item::Segment { .. } => "segment",
        Item::EndStroke(_) => "stroke",
        Item::Fill { .. } => "fill",
        Item::EndLayer { chain: false, .. } => "layer",
        Item::EndLayer { chain: true, .. } => "chained",
        Item::EndChain { .. } => "chain",
        Item::EndGroup(_) => "group",
        Item::Active => "active",
        Item::Reach(_) => "reach",
        Item::Smudge(_) => "smudge",
    };
    scene.items.iter().map(name).collect()
}

#[test]
fn active_place_follows_its_layers_finished_strokes() {
    let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
    let mut session = blank
        .begin_stroke(style(), sample(1.0, 1.0, 1.0, 0))
        .expect("begin");
    session.extend(&[sample(9.0, 9.0, 1.0, 5)]).expect("extend");
    let one = session.finish().artwork;
    let scene = Scene::build(&one, Some(&place(one.active(), None))).expect("scene");
    assert_eq!(codes(&scene), ["segment", "stroke", "active", "layer"]);
    assert!(scene.has_active());
    assert_eq!(
        scene.active_before,
        Some(0..2),
        "the layer's strokes before it"
    );
    let empty = Scene::build(&blank, Some(&place(blank.active(), None))).expect("scene");
    assert_eq!(codes(&empty), ["active", "layer"]);
}

fn place(layer: LayerId, clip: Option<RegionMask>) -> ActivePlace {
    ActivePlace { layer, clip }
}

#[test]
fn a_smudge_is_its_reach_then_its_op_with_a_base_word_of_its_own() {
    let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
    let mut artwork = blank.clone();
    for tool in [StrokeTool::Paint, StrokeTool::Smudge, StrokeTool::Smudge] {
        let mut session = artwork
            .begin_stroke(style().with_tool(tool), sample(4.0, 4.0, 1.0, 0))
            .expect("begin");
        session
            .extend(&[sample(20.0, 9.0, 1.0, 5)])
            .expect("extend");
        artwork = session.finish().artwork;
    }
    let scene = Scene::build(&artwork, None).expect("scene");
    let expected = [
        "segment", "stroke", "reach", "smudge", "reach", "smudge", "layer",
    ];
    assert_eq!(codes(&scene), expected);
    assert_eq!(
        (scene.bases_at, scene.masks.len()),
        (0, 2),
        "two base words first"
    );
    let ranges: Vec<_> = scene.smudges.iter().map(|s| s.before.clone()).collect();
    assert_eq!(ranges, [0..2, 0..4]);
    let newest: Vec<_> = scene.smudges.iter().map(|s| s.newest).collect();
    assert_eq!(newest, [false, true]);
}

#[test]
fn strokes_inside_one_selection_share_its_mask_with_the_stroke_being_drawn() {
    let canvas = CanvasSize::new(64, 64).expect("size");
    let clip =
        RegionMask::polygon(canvas, &[[4.0, 4.0], [40.0, 4.0], [40.0, 40.0]]).expect("a triangle");
    let mut artwork = Artwork::new(canvas);
    for x in [5.0, 9.0] {
        let session = artwork
            .begin_stroke(style(), sample(x, 10.0, 1.0, 0))
            .expect("begin");
        artwork = session
            .clipped(clip.clone())
            .expect("clip")
            .finish()
            .artwork;
    }
    let words = MASK_HEADER as usize + clip.words().len();
    let place = place(artwork.active(), Some(clip.clone()));
    let scene = Scene::build(&artwork, Some(&place)).expect("scene");
    assert_eq!(scene.masks.len(), words, "one copy");
    assert_eq!(scene.active_clip, 1);
    let alone = Scene::build(&artwork, None).expect("scene");
    assert_eq!((alone.masks.len(), alone.active_clip), (words, 0));
}

#[test]
fn hidden_and_empty_layers_add_nothing() {
    let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
    let session = blank
        .begin_stroke(style(), sample(1.0, 1.0, 1.0, 0))
        .expect("begin");
    let drawn = session.finish().artwork;
    let layer = drawn.active();
    let hide = LayerCommand::SetVisible {
        layer,
        visible: false,
    };
    let hidden = drawn
        .apply(ArtworkCommand::Layer(hide))
        .expect("hide")
        .artwork;
    let build = |artwork: &Artwork, active| Scene::build(artwork, active).expect("scene");
    assert_eq!(codes(&build(&blank, None)), [""; 0]);
    assert_eq!(codes(&build(&hidden, None)), [""; 0]);
    let place = place(layer, None);
    let hidden_place = Scene::build(&hidden, Some(&place)).expect("scene");
    assert!(!hidden_place.has_active());
    let shown = codes(&build(&drawn, None));
    assert_eq!(shown, ["segment", "stroke", "layer"]);
}

/// Layers 1 to 3 with one stroke each, 3 clipping to 2, and 2 and 3 in group 4.
fn chained_in_a_group() -> Artwork {
    let mut artwork = Artwork::new(CanvasSize::new(64, 64).expect("size"));
    for at in 0..3 {
        if at > 0 {
            let created = artwork.apply(ArtworkCommand::Layer(LayerCommand::Create));
            artwork = created.expect("create").artwork;
        }
        let session = artwork
            .begin_stroke(style(), sample(4.0, 4.0, 1.0, 0))
            .expect("begin");
        artwork = session.finish().artwork;
    }
    let clip = LayerCommand::SetClipping {
        layer: LayerId::new(3),
        clipping: true,
    };
    let group = GroupCommand::Create {
        layer: LayerId::new(3),
    };
    let artwork = artwork.apply(ArtworkCommand::Layer(clip)).expect("clip");
    let artwork = artwork.artwork.apply(ArtworkCommand::Group(group));
    artwork.expect("group").artwork
}

#[test]
fn a_chain_ends_after_its_last_clipping_layer_and_a_group_after_its_top_layer() {
    let artwork = chained_in_a_group();
    let scene = Scene::build(&artwork, None).expect("scene");
    let expected = [
        "segment", "stroke", "layer", "segment", "stroke", "chained", "segment", "stroke",
        "chained", "chain", "group",
    ];
    assert_eq!(codes(&scene), expected);
    let set = |artwork: &Artwork, command| {
        let applied = artwork.apply(command).expect("command");
        Scene::build(&applied.artwork, None).expect("scene")
    };
    let hide_base = LayerCommand::SetVisible {
        layer: LayerId::new(2),
        visible: false,
    };
    let base_hidden = set(&artwork, ArtworkCommand::Layer(hide_base));
    assert_eq!(
        codes(&base_hidden),
        ["segment", "stroke", "layer", "group"],
        "a hidden base takes its chain along"
    );
    let hide_group = GroupCommand::SetVisible {
        group: openpaint_core::GroupId::new(4),
        visible: false,
    };
    let group_hidden = set(&artwork, ArtworkCommand::Group(hide_group));
    assert_eq!(codes(&group_hidden), ["segment", "stroke", "layer"]);
}

#[test]
fn fresh_ops_start_with_the_paint_op() {
    let samples = [sample(10.0, 20.0, 1.0, 0), sample(30.0, 20.0, 0.2, 4)];
    let blank = Artwork::new(CanvasSize::new(64, 64).expect("size"));
    let mut session = blank.begin_stroke(style(), samples[0]).expect("begin");
    let dot = Fresh::new(session.active(), 0, 0);
    assert_eq!(dot.bounds.len(), 1);
    session.extend(&samples[1..]).expect("extend");
    let fresh = Fresh::new(session.active(), 0, 7);
    assert_eq!(fresh.ops.len(), 2);
    let clipped = Paint {
        clip: 7,
        ..paint(&style(), openpaint_core::StrokeBlend::Over)
    };
    assert_eq!(fresh.ops[0], Op::end_active(&clipped), "with the clip");
    assert_eq!(fresh.bounds.len(), 1);
    assert_eq!(
        Some(fresh.bounds[0].pixels()),
        stroke_region(&style(), &samples, 0)
    );
    assert_eq!(Fresh::new(session.active(), 1, 0).bounds.len(), 0);
    let smudging = blank
        .begin_stroke(style().with_tool(StrokeTool::Smudge), samples[0])
        .expect("begin");
    let smudge = Fresh::new(smudging.active(), 0, 3);
    assert_eq!(
        smudge.ops,
        [Op::smudge(None, 3, false)],
        "its residual, everywhere"
    );
}
