//! Transform records on the GPU (S31): content placed by mapping back to the earlier ops. At 1x
//! a record shows what the placed patch of S20 showed, within the difference between
//! re-evaluating and resampling once (stated per test); outside what moved nothing changes;
//! every drag frame evaluated incrementally equals the committed artwork, reloaded and
//! evaluated cold; masks stay and clipping layers follow their moved base.

use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, Color, LayerCommand, MaskCommand, Patch, Placement,
    RegionMask, StrokeStyle, StrokeTool,
};
use openpaint_render::{CanvasTarget, Core, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{
    SIZE, airbrush, blank, cold, core, diagonal, diagonal_with, graphite, plain, request, sample,
    wave, with_layer, with_stroke,
};
use crate::scale::difference;

pub fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

/// The wave, a black bar, the diagonal in pencil, an orange fill, then a smudge across.
pub fn busy() -> Artwork {
    let bar = (
        plain(12.0, 1.0, Color::BLACK),
        vec![sample(40.0, 40.0, 1.0, 0), sample(140.0, 40.0, 1.0, 8)],
    );
    let drawn = with_stroke(&with_stroke(&blank(), wave()), bar);
    let pencil = with_stroke(&drawn, diagonal_with(graphite()));
    let square = RegionMask::polygon(
        canvas(),
        &[[200.0, 30.0], [260.0, 42.0], [250.0, 90.0], [205.0, 80.0]],
    )
    .expect("a quad");
    let orange = Color::rgb(0.95, 0.55, 0.1).expect("orange");
    let filled = pencil.fill_region(square, orange).expect("fill").artwork;
    let smudge = StrokeStyle::new(airbrush(), 30.0, 0.8, Color::BLACK)
        .expect("style")
        .with_tool(StrokeTool::Smudge);
    let path = (0..24u16)
        .map(|i| {
            let t = f32::from(i) / 23.0;
            sample(
                90.0 + 150.0 * t,
                60.0 + 70.0 * t,
                0.5 + 0.5 * t,
                u32::from(i) * 8,
            )
        })
        .collect();
    with_stroke(&filled, (smudge, path))
}

/// The active layer's pixels (inside `clip`) as a canvas-sized patch, as S20 lifted them.
pub fn lift(core: &Core, artwork: &Artwork, clip: Option<&RegionMask>) -> Patch {
    let mut pixels = core
        .layer_pixels(artwork, artwork.active())
        .expect("sample")
        .into_pixels();
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let at = [index as u32 % SIZE[0], index as u32 / SIZE[0]];
        if clip.is_some_and(|clip| !clip.contains(at)) {
            pixel.fill(0);
        }
    }
    Patch::new(canvas(), [0, 0], SIZE, pixels).expect("patch")
}

/// `artwork` with its active layer (inside `clip`) moved by `placement`: as S20's patch.
pub fn by_patch(
    core: &Core,
    artwork: &Artwork,
    clip: Option<&RegionMask>,
    placement: Placement,
) -> Artwork {
    let patch = lift(core, artwork, clip);
    let applied = artwork.transform_region(clip.cloned(), patch, placement);
    applied.expect("transform").artwork
}

/// The same as a record.
pub fn by_record(artwork: &Artwork, clip: Option<&RegionMask>, placement: Placement) -> Artwork {
    let applied = artwork.place_region(clip.cloned(), placement);
    applied.expect("transform").artwork
}

/// Moved, scaled up and turned about the canvas's middle.
pub fn turned() -> Placement {
    Placement::around([160.0, 100.0], 1.37, 0.41, [false; 2])
        .and_then(|turn| turn.then(Placement::moved([12.5, -7.25])?))
        .expect("placement")
}

pub fn lasso() -> RegionMask {
    RegionMask::polygon(
        canvas(),
        &[[50.0, 20.0], [250.0, 60.0], [200.0, 180.0], [70.0, 150.0]],
    )
    .expect("a lasso")
}

/// The record and the patch at 1x: largest and mean channel difference.
fn against_patch(core: &Core, artwork: &Artwork, clip: Option<&RegionMask>) -> (u8, f64) {
    let record = cold(core, &by_record(artwork, clip, turned()));
    let patch = cold(core, &by_patch(core, artwork, clip, turned()));
    difference(record.pixels(), patch.pixels())
}

#[test]
fn a_layer_record_shows_what_the_patch_showed_at_1x() {
    let core = core();
    let (largest, mean) = against_patch(&core, &busy(), None);
    println!("whole layer, record against patch: largest {largest}, mean {mean:.3}");
    // Evaluated anew, edges are sharper than the patch's bilinear resampling of them (largest
    // 72 on 2026-10-04: a stroke's antialiased rim, blurred by the patch), the grain finer.
    assert!(mean < 1.0 && largest < 96, "largest {largest}, mean {mean}");
}

#[test]
fn a_selection_record_shows_what_the_patch_showed_at_1x() {
    let core = core();
    let (largest, mean) = against_patch(&core, &busy(), Some(&lasso()));
    println!("selection, record against patch: largest {largest}, mean {mean:.3}");
    // Where the lasso cuts through paint the record's cut is the mask's pixel edge, hard as
    // the lift's, where the patch's bilinear sampling faded it (largest 155 on 2026-10-04:
    // the fill cut by the lasso, enlarged 1.37 times).
    assert!(
        mean < 1.0 && largest < 192,
        "largest {largest}, mean {mean}"
    );
}

#[test]
fn outside_what_a_selection_record_moved_nothing_changes() {
    let core = core();
    let base = busy();
    let clip = lasso();
    let moved = by_record(&base, Some(&clip), turned());
    let (before, after) = (cold(&core, &base), cold(&core, &moved));
    // Where the moved content can land: the lasso's rectangle, three pixels wider, placed.
    let ([ox, oy], [w, h]) = (clip.origin(), clip.size());
    let placed = |x: u32, y: u32| {
        let inverse = turned().inverse_rows();
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        let sx = inverse[0] * px + inverse[1] * py + inverse[2];
        let sy = inverse[3] * px + inverse[4] * py + inverse[5];
        (ox as f32 - 3.0..(ox + w) as f32 + 3.0).contains(&sx)
            && (oy as f32 - 3.0..(oy + h) as f32 + 3.0).contains(&sy)
    };
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            if !clip.contains([x, y]) && !placed(x, y) {
                let at = ((y * SIZE[0] + x) * 4) as usize;
                assert_eq!(
                    before.pixels()[at..at + 4],
                    after.pixels()[at..at + 4],
                    "{x}, {y}"
                );
            }
        }
    }
}

/// Every placement of a drag evaluated on one target, then the last against the committed
/// artwork reloaded and evaluated cold.
fn drag_equals_commit(artwork: &Artwork, clip: Option<&RegionMask>) {
    let core = core();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("base");
    let steps = [
        Placement::moved([12.5, 7.25]).expect("move"),
        Placement::around([40.0, 30.0], 2.37, 0.0, [false; 2]).expect("scale"),
        turned(),
        Placement::around([150.0, 90.0], 0.6, -0.61, [true, false]).expect("flip"),
    ];
    let mut shown = artwork.clone();
    for placement in steps {
        let preview = by_record(artwork, clip, placement);
        let regions = changed_regions(&shown, &preview);
        evaluator
            .evaluate(&request(&preview), &regions)
            .expect("frame");
        shown = preview;
    }
    let later = with_stroke(&shown, diagonal());
    evaluator
        .evaluate(&request(&later), &changed_regions(&shown, &later))
        .expect("a stroke after");
    let last = target.read(&core).expect("the last frame");
    let committed = Artwork::from_storage(later.to_parts()).expect("reloaded");
    assert_eq!(last, cold(&core, &committed), "incremental equals cold");
}

/// The app's drag: the lifted pixels while the hand moves, the record once it lifts; the
/// settled frame, evaluated incrementally, equals the commit reloaded and evaluated cold.
#[test]
fn settling_a_patch_drag_into_its_record_equals_the_commit() {
    let core = core();
    let base = busy();
    for clip in [None, Some(lasso())] {
        let patch = lift(&core, &base, clip.as_ref());
        let mut target = CanvasTarget::new(&core, SIZE).expect("target");
        let mut evaluator = RegionEvaluator::new(&core, &mut target);
        evaluator
            .evaluate(&request(&base), &[PixelRect::full(SIZE)])
            .expect("base");
        let mut shown = base.clone();
        for step in 1..=3 {
            let placement = Placement::moved([7.5 * step as f32, 3.0]).expect("move");
            let applied = base.transform_region(clip.clone(), patch.clone(), placement);
            let preview = applied.expect("preview").artwork;
            evaluator
                .evaluate(&request(&preview), &changed_regions(&shown, &preview))
                .expect("drag frame");
            shown = preview;
        }
        let settled = by_record(&base, clip.as_ref(), turned());
        evaluator
            .evaluate(&request(&settled), &changed_regions(&shown, &settled))
            .expect("settled");
        let last = target.read(&core).expect("read");
        let committed = Artwork::from_storage(settled.to_parts()).expect("reloaded");
        assert_eq!(last, cold(&core, &committed), "settled equals committed");
    }
}

#[test]
fn dragging_a_layer_record_equals_its_commit_evaluated_cold() {
    drag_equals_commit(&busy(), None);
}

#[test]
fn dragging_a_selection_record_equals_its_commit_evaluated_cold() {
    drag_equals_commit(&busy(), Some(&lasso()));
}

#[test]
fn a_record_leaves_the_mask_in_place_and_moves_a_clipping_layers_base() {
    let core = core();
    let base = busy();
    let layer = base.active();
    let masked = base
        .apply(ArtworkCommand::Mask(MaskCommand::Add(layer)))
        .expect("mask")
        .artwork;
    let grey = plain(50.0, 1.0, Color::rgb(0.4, 0.4, 0.4).expect("grey"));
    let path = vec![sample(30.0, 160.0, 1.0, 0), sample(290.0, 170.0, 1.0, 8)];
    let painted = with_stroke(&masked, (grey, path));
    let select = LayerCommand::Select(layer);
    let paint = painted
        .apply(ArtworkCommand::Layer(select))
        .expect("paint")
        .artwork;
    let clipping = with_stroke(&with_layer(&paint, 1.0), diagonal());
    let top = clipping.active();
    let clipped = clipping
        .apply(ArtworkCommand::Layer(LayerCommand::SetClipping {
            layer: top,
            clipping: true,
        }))
        .expect("clip")
        .artwork;
    let on_base = clipped
        .apply(ArtworkCommand::Layer(LayerCommand::Select(layer)))
        .expect("base")
        .artwork;
    for clip in [None, Some(lasso())] {
        let (largest, mean) = against_patch(&core, &on_base, clip.as_ref());
        println!("mask and clipping layer: largest {largest}, mean {mean:.3}");
        assert!(
            mean < 1.0 && largest < 192,
            "largest {largest}, mean {mean}"
        );
    }
    drag_equals_commit(&on_base, Some(&lasso()));
}
