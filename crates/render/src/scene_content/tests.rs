//! Transform records in the scene: a whole layer's record lists its content once under a frame;
//! a selection's lists what stays, then the content again moved; nudging the same selection
//! again and again grows the scene by a few items each time, not twice over, and sets nothing
//! aside where nothing stayed; selections moved over content of their own nest a level per
//! record, up to the raster core's `MOVE_DEPTH` levels.

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, Placement, RegionMask, StrokeSample,
    StrokeStyle,
};

use crate::protocol::MOVE_DEPTH;
use crate::scene::{Item, Scene};

fn sample(x: f32, y: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, 1.0, 0.0, 0.0, time_ms).expect("sample")
}

/// `strokes` black lines from x 40 to 100, 6 px apart from y 40.
fn drawn(canvas: CanvasSize, strokes: usize) -> Artwork {
    let recipe = Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 6.0, 1.0, Color::BLACK).expect("style");
    let mut artwork = Artwork::new(canvas);
    for i in 0..strokes {
        let y = 40.0 + 6.0 * i as f32;
        let mut session = artwork
            .begin_stroke(style.clone(), sample(40.0, y, 0))
            .expect("begin");
        session.extend(&[sample(100.0, y, 8)]).expect("extend");
        artwork = session.finish().artwork;
    }
    artwork
}

fn codes(scene: &Scene) -> Vec<&'static str> {
    let name = |item: &Item| match item {
        Item::Segment { .. } => "segment",
        Item::EndStroke(_) => "stroke",
        Item::Fill { .. } => "fill",
        Item::Clear { .. } => "clear",
        Item::EndLayer { .. } => "layer",
        Item::Frame { .. } => "frame",
        Item::BeginMove(_) => "move",
        Item::EndMove { whole: false, .. } => "moved",
        Item::EndMove { whole: true, .. } => "cut",
        _ => "other",
    };
    scene.items.iter().map(name).collect()
}

fn canvas() -> CanvasSize {
    CanvasSize::new(200, 160).expect("size")
}

fn rectangle(x: f32, y: f32, w: f32, h: f32) -> Option<RegionMask> {
    RegionMask::polygon(canvas(), &[[x, y], [x + w, y], [x + w, y + h], [x, y + h]])
}

/// How many levels of moved content `artwork`'s scene nests at most: what the raster core sets
/// aside on its level targets at once.
fn deepest(artwork: &Artwork) -> u32 {
    let scene = Scene::build(artwork, None).expect("scene");
    let (mut depth, mut deepest) = (0u32, 0);
    for item in &scene.items {
        match item {
            Item::BeginMove(_) => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            Item::EndMove { whole: false, .. } | Item::EndNested(_) => depth -= 1,
            _ => {}
        }
    }
    deepest
}

#[test]
fn a_whole_layer_record_lists_its_content_once_under_a_frame() {
    let moved = Placement::moved([5.0, 3.0]).expect("move");
    let artwork = drawn(canvas(), 1).place_region(None, moved).expect("moved");
    let twice = artwork.artwork.place_region(None, moved).expect("again");
    let scene = Scene::build(&twice.artwork, None).expect("scene");
    assert_eq!(
        codes(&scene),
        [
            "frame", "segment", "stroke", "cut", "frame", "cut", "frame", "layer"
        ],
        "one frame per segment, the maps composed"
    );
}

#[test]
fn a_selection_record_lists_what_stays_then_the_content_moved() {
    let lift = RegionMask::polygon(canvas(), &[[30.0, 30.0], [80.0, 30.0], [80.0, 60.0]]);
    let moved = Placement::moved([40.0, 20.0]).expect("move");
    let artwork = drawn(canvas(), 1).place_region(lift, moved).expect("moved");
    let scene = Scene::build(&artwork.artwork, None).expect("scene");
    let expected = [
        "segment", "stroke", "clear", "move", "frame", "segment", "stroke", "moved", "frame",
        "layer",
    ];
    assert_eq!(codes(&scene), expected);
}

/// After `nudges` selection records, each a rectangle 6 px wider than the strokes where the
/// last one put them, moved 8 px right: the scene's items and its deepest nesting.
fn nudged(nudges: usize) -> (usize, u32) {
    let mut artwork = drawn(canvas(), 6);
    for nudge in 0..nudges {
        let x = 30.0 + 4.0 * nudge as f32;
        let moved = Placement::moved([4.0, 0.0]).expect("move");
        let lift = rectangle(x, 30.0, 80.0, 50.0);
        artwork = artwork.place_region(lift, moved).expect("nudge").artwork;
    }
    let items = Scene::build(&artwork, None).expect("scene").items.len();
    (items, deepest(&artwork))
}

#[test]
fn nudging_one_selection_again_and_again_grows_the_scene_by_a_few_items_each_time() {
    let counts: Vec<(usize, u32)> = (0..=24).map(nudged).collect();
    println!("scene items and deepest nesting after 0 to 24 nudges: {counts:?}");
    for pair in counts.windows(2).skip(1) {
        assert!(pair[1].0 <= pair[0].0 + 16, "{counts:?}");
    }
    // Each record's moved content holds the one before: a level more each time, which the raster
    // core draws up to `MOVE_DEPTH` (STATUS, Known limits).
    let depths: Vec<u32> = counts.iter().map(|(_, deepest)| *deepest).collect();
    assert_eq!(depths, (0..=24).collect::<Vec<u32>>());
    let plan = |nudges: usize| {
        let mut artwork = drawn(canvas(), 6);
        for nudge in 0..nudges {
            let lift = rectangle(30.0 + 4.0 * nudge as f32, 30.0, 80.0, 50.0);
            let moved = Placement::moved([4.0, 0.0]).expect("move");
            artwork = artwork.place_region(lift, moved).expect("nudge").artwork;
        }
        let scene = Scene::build(&artwork, None).expect("scene");
        let bases = crate::smudge::Bases::default();
        let region = (crate::output::Output::CANVAS, [200, 160]);
        crate::raster_plan::RasterPlan::build((&scene, &bases), region, [1024; 2]).map(drop)
    };
    assert!(plan(MOVE_DEPTH as usize).is_ok());
    assert!(
        plan(MOVE_DEPTH as usize + 1).is_err(),
        "refused past the levels"
    );
}

#[test]
fn selections_moved_over_their_own_content_set_aside_once_per_record() {
    // The worst case: every other column selected and moved a pixel right, so what stayed and
    // what moved interleave everywhere at every record: each one sets the layer aside, and
    // the scene doubles per record (a plan deeper than `MOVE_DEPTH` is refused).
    let mut inside = vec![false; 200 * 160];
    for (at, pixel) in inside.iter_mut().enumerate() {
        *pixel = at % 2 == 0;
    }
    let stripes = RegionMask::from_pixels(canvas(), &inside).expect("stripes");
    let mut artwork = drawn(canvas(), 1);
    let mut measured = Vec::new();
    for _ in 0..6 {
        let moved = Placement::moved([1.0, 0.0]).expect("move");
        artwork = artwork
            .place_region(Some(stripes.clone()), moved)
            .expect("move")
            .artwork;
        let items = Scene::build(&artwork, None).expect("scene").items.len();
        measured.push((items, deepest(&artwork)));
    }
    println!("striped records, scene items and deepest stack: {measured:?}");
    let depths: Vec<u32> = measured.iter().map(|(_, deepest)| *deepest).collect();
    assert_eq!(depths, [1, 2, 3, 4, 5, 6]);
    const { assert!(MOVE_DEPTH >= 16) };
}
