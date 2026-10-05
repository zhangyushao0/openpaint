use openpaint_core::{
    Adjustment, BrushRecipe, CanvasPoint, CanvasSize, Color, Hsb, StrokeSample, StrokeStyle,
};

use crate::core::test_core;
use crate::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, changed_regions};

use super::Artwork;

const SIZE: [u32; 2] = [160, 96];

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

fn with_stroke(artwork: &Artwork, y: f32) -> Artwork {
    let recipe = std::sync::Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 6.0, 1.0, Color::BLACK).expect("style");
    let mut session = artwork
        .begin_stroke(style, sample(20.0, y, 0))
        .expect("begin");
    session
        .extend(&[sample(70.0, y + 12.0, 8), sample(130.0, y, 16)])
        .expect("extend");
    session.finish().artwork
}

fn request(artwork: &Artwork) -> FrameRequest<'_> {
    FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    }
}

/// Shows `next` after `shown` on `target`; returns the serial of the plan it kept.
fn show(target: &mut CanvasTarget, shown: &Artwork, next: &Artwork) -> u64 {
    let core = test_core();
    let regions = changed_regions(shown, next);
    RegionEvaluator::new(core, target)
        .evaluate(&request(next), &regions)
        .expect("evaluated");
    target.scene.as_ref().map_or(0, |cache| cache.plan_serial())
}

/// While one adjustment's values change, the scene stays and the plan is binned once (when the
/// layer first resumes from its snapshot); a change of structure (a stroke) bins anew.
#[test]
fn a_preview_keeps_its_plan_and_a_structure_change_bins_anew() {
    let core = test_core();
    let base = with_stroke(
        &Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size")),
        40.0,
    );
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    let mut shown = base.clone();
    let mut serials = Vec::new();
    for hue in [0.2, 0.4, 0.6, 0.8] {
        let preview = base
            .adjust_region(None, Adjustment::Hsb(Hsb::new(hue, 0.6, 0.5)))
            .expect("adjusted")
            .artwork;
        serials.push(show(&mut target, &shown, &preview));
        shown = preview;
    }
    let previewed = target
        .scene
        .as_ref()
        .and_then(|cache| cache.snapshot.as_ref());
    assert!(previewed.is_some(), "the layer resumes from a snapshot");
    assert_ne!(
        serials[0], serials[1],
        "the first value binned the scene, the second resumed"
    );
    assert_eq!(serials[1..], [serials[1]; 3], "the plan stays: {serials:?}");
    let drawn = with_stroke(&shown, 70.0);
    assert_ne!(
        show(&mut target, &shown, &drawn),
        serials[3],
        "a stroke bins anew"
    );
    assert!(
        target
            .scene
            .as_ref()
            .is_some_and(|cache| cache.snapshot.is_none())
    );
}
