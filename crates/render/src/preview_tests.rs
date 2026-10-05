use super::*;
use openpaint_core::{
    Adjustment, BrushRecipe, CanvasPoint, CanvasSize, Color, Hsb, Sharpen, StrokeSample,
    StrokeStyle,
};

fn sample(x: f32, y: f32, time_ms: u32) -> StrokeSample {
    StrokeSample::new(
        CanvasPoint::new(x, y).expect("point"),
        1.0,
        0.0,
        0.0,
        time_ms,
    )
    .expect("sample")
}

/// A canvas with one stroke across its middle.
fn stroked() -> Artwork {
    let artwork = Artwork::new(CanvasSize::new(160, 96).expect("size"));
    let recipe = std::sync::Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 6.0, 1.0, Color::BLACK).expect("style");
    let mut session = artwork
        .begin_stroke(style, sample(20.0, 40.0, 0))
        .expect("begin");
    session
        .extend(&[sample(70.0, 52.0, 8), sample(130.0, 44.0, 16)])
        .expect("extend");
    session.finish().artwork
}

fn adjusted(artwork: &Artwork, adjustment: Adjustment) -> Artwork {
    artwork
        .adjust_region(None, adjustment)
        .expect("adjusted")
        .artwork
}

fn hsb(hue: f32) -> Adjustment {
    Adjustment::Hsb(Hsb::new(hue, 0.5, 0.5))
}

#[test]
fn only_a_change_of_one_adjustments_values_is_a_retune() {
    let base = stroked();
    let first = adjusted(&base, hsb(0.2));
    let second = adjusted(&base, hsb(0.7));
    let retune = retuned(&first, &second).expect("a retune");
    assert_eq!(retune.place.2, 0, "the layer's first edit");
    assert_eq!(retune.edit.adjustment(), &hsb(0.7));
    assert_eq!(retune.spread, 0, "a point adjustment spreads nowhere");
    assert!(retuned(&first, &first).is_none(), "the same version");
    assert!(retuned(&base, &first).is_none(), "an edit added");
    let sharpened = adjusted(&base, Adjustment::Sharpen(Sharpen::new(0.4)));
    assert!(retuned(&first, &sharpened).is_none(), "another kind");
    let blurred = |radius| adjusted(&base, Adjustment::Blur(openpaint_core::Blur::new(radius)));
    let wide = retuned(&blurred(2.0), &blurred(32.0)).expect("a blur's radius");
    assert_eq!(
        wide.spread, 7,
        "a reach of 96 and cells of 4 pixels: 104 pixels"
    );
}

/// The scene a retune leaves equals the scene of the new version built anew: the same items
/// and mask words, the same filter results named.
#[test]
fn a_retuned_scene_equals_the_new_versions_scene() {
    let base = stroked();
    let blur = |radius| Adjustment::Blur(openpaint_core::Blur::new(radius));
    let lifted = openpaint_core::ToneCurve::new(&[[0.0, 0.2], [1.0, 0.8]]).expect("curve");
    let curves = |curve| {
        let channel = openpaint_core::ToneChannel::Composite;
        Adjustment::Curves(openpaint_core::Curves::default().with(channel, curve))
    };
    let flat = openpaint_core::ToneCurve::new(&[[0.0, 0.0], [1.0, 0.5]]).expect("curve");
    for (from, to) in [
        (hsb(0.1), hsb(0.9)),
        (curves(lifted), curves(flat)),
        (blur(3.0), blur(25.0)),
    ] {
        let (old, new) = (adjusted(&base, from), adjusted(&base, to));
        let mut scene = Scene::build(&old, None).expect("old");
        scene
            .retune(&retuned(&old, &new).expect("retune"))
            .expect("patched");
        let built = Scene::build(&new, None).expect("new");
        assert_eq!(scene.items, built.items);
        assert_eq!(scene.masks, built.masks);
        let edits = |scene: &Scene| -> Vec<_> {
            scene
                .filters
                .iter()
                .map(|item| item.key.edit().cloned())
                .collect()
        };
        assert_eq!(edits(&scene), edits(&built));
    }
}

/// An adjustment over nothing reaches no pixel; over a stroke, the stroke's bounds.
#[test]
fn an_unclipped_adjustment_reaches_only_the_content_before_it() {
    let empty = Artwork::new(CanvasSize::new(160, 96).expect("size"));
    let scene = Scene::build(&adjusted(&empty, hsb(0.3)), None).expect("scene");
    assert!(
        scene
            .items
            .iter()
            .all(|item| item.bounds().is_none_or(|b| b.is_empty()))
    );
    let scene = Scene::build(&adjusted(&stroked(), hsb(0.3)), None).expect("scene");
    let adjust = scene.adjusts[0].item;
    let reach = scene.items[adjust].bounds().expect("bounds").pixels();
    assert!(reach.min[0] >= 10 && reach.max[0] <= 140, "{reach:?}");
    assert!(reach.min[1] >= 30 && reach.max[1] <= 62, "{reach:?}");
}
