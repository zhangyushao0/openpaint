//! Adjustments on the GPU (S28, D54): an applied adjustment is an operation the one evaluator
//! runs on the layer's content so far. Each shows the values worked out by hand, on paint and on
//! a mask (inverted too), only inside its selection and never on what was painted after it;
//! preview frames evaluated incrementally on one target end equal to the committed artwork
//! reloaded and evaluated cold; after an adjustment, a stroke and their undo, incremental
//! equals cold. Gaussian Blur and Sharpen (S33) take the same preview path.

mod gpu;

use std::sync::Arc;

use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, Blur, BrushRecipe, CanvasPoint, CanvasSize, Color, Curves,
    Hsb, LayerId, MaskCommand, RegionMask, Sharpen, StrokeSample, StrokeStyle, ToneChannel,
    ToneCurve,
};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage, changed_regions,
};

const SIZE: [u32; 2] = [96, 64];
/// The banding measurement's canvas: a soft ramp needs room.
const TALL: [u32; 2] = [64, 600];
const LAYER: LayerId = LayerId::new(1);

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

fn rectangle(from: [f32; 2], to: [f32; 2]) -> RegionMask {
    let corners = [from, [to[0], from[1]], to, [from[0], to[1]]];
    RegionMask::polygon(canvas(), &corners).expect("a rectangle")
}

/// Opaque red filling (20, 16) to (60, 44) on the one layer.
fn red() -> Artwork {
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    let mask = rectangle([20.0, 16.0], [60.0, 44.0]);
    Artwork::new(canvas())
        .fill_region(mask, red)
        .expect("fill")
        .artwork
}

fn with_mask(artwork: &Artwork, inverted: bool) -> Artwork {
    let mut masked = artwork
        .apply(ArtworkCommand::Mask(MaskCommand::Add(LAYER)))
        .expect("mask")
        .artwork;
    if inverted {
        let invert = ArtworkCommand::Mask(MaskCommand::Invert(LAYER));
        masked = masked.apply(invert).expect("invert").artwork;
    }
    masked
}

/// A soft blue diagonal stroke on the target (the paint, or the mask's grey).
fn stroked(artwork: &Artwork) -> Artwork {
    let color = Color::rgb(0.1, 0.3, 0.8).expect("blue");
    let style = StrokeStyle::new(Arc::new(BrushRecipe::plain()), 14.0, 0.6, color).expect("style");
    let sample = |x: f32, y: f32, t: u32| {
        let at = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(at, 0.8, 0.0, 0.0, t).expect("sample")
    };
    let mut session = artwork
        .begin_stroke(style, sample(10.0, 10.0, 0))
        .expect("begin");
    session
        .extend(&[sample(50.0, 30.0, 8), sample(86.0, 54.0, 16)])
        .expect("extend");
    session.finish().artwork
}

fn adjusted(artwork: &Artwork, clip: Option<&RegionMask>, adjustment: Adjustment) -> Artwork {
    let applied = artwork.adjust_region(clip.cloned(), adjustment);
    applied.expect("adjusted").artwork
}

fn request(artwork: &Artwork) -> FrameRequest<'_> {
    FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    }
}

fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    target.read(core).expect("readback")
}

fn pixel(image: &RgbaImage, [x, y]: [u32; 2]) -> [u8; 3] {
    let at = ((y * SIZE[0] + x) * 4) as usize;
    [0, 1, 2].map(|i| image.pixels()[at + i])
}

fn near(got: [u8; 3], expected: [u8; 3]) -> bool {
    got.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1)
}

fn hsb(hue: f32, saturation: f32, brightness: f32) -> Adjustment {
    Adjustment::Hsb(Hsb::new(hue, saturation, brightness))
}

fn red_curve_halved() -> Adjustment {
    let half = ToneCurve::new(&[[0.0, 0.0], [1.0, 0.5]]).expect("curve");
    Adjustment::Curves(Curves::default().with(ToneChannel::Red, half))
}

#[test]
fn each_adjustment_shows_the_values_worked_out_by_hand() {
    let core = gpu::core();
    let cases: [(Adjustment, [u8; 3]); 6] = [
        // A third of a turn: red to green; half a turn: cyan.
        (hsb(0.5 + 1.0 / 3.0, 0.5, 0.5), [0, 255, 0]),
        (hsb(0.0, 0.5, 0.5), [0, 255, 255]),
        // No saturation: red's luma, 0.299 x 255 = 76.2.
        (hsb(0.5, 0.0, 0.5), [76, 76, 76]),
        // A quarter brightness: half as bright (127.5); three quarters: halfway to white.
        (hsb(0.5, 0.5, 0.25), [128, 0, 0]),
        (hsb(0.5, 0.5, 0.75), [255, 128, 128]),
        // The red curve halved: 255 to round(127.5).
        (red_curve_halved(), [128, 0, 0]),
    ];
    for (adjustment, expected) in cases {
        let shown = cold(&core, &adjusted(&red(), None, adjustment.clone()));
        let got = pixel(&shown, [40, 30]);
        assert!(near(got, expected), "{adjustment:?}: {got:?}");
        assert_eq!(pixel(&shown, [4, 4]), [255; 3], "the paper stays");
    }
}

#[test]
fn only_the_selection_and_the_content_so_far_change() {
    let core = gpu::core();
    let clip = rectangle([20.0, 16.0], [40.0, 44.0]);
    let turned = adjusted(&red(), Some(&clip), hsb(0.5 + 1.0 / 3.0, 0.5, 0.5));
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    let later = turned
        .fill_region(rectangle([60.0, 16.0], [80.0, 44.0]), red)
        .expect("a later fill")
        .artwork;
    let shown = cold(&core, &later);
    assert!(near(pixel(&shown, [30, 30]), [0, 255, 0]), "inside: green");
    assert_eq!(pixel(&shown, [50, 30]), [255, 0, 0], "outside: red");
    assert_eq!(pixel(&shown, [70, 30]), [255, 0, 0], "painted after: red");
}

#[test]
fn a_mask_is_adjusted_in_the_grey_it_shows() {
    let core = gpu::core();
    // The white mask half as bright: grey 0.5 shows the red at half over white.
    let shown = cold(
        &core,
        &adjusted(&with_mask(&red(), false), None, hsb(0.5, 0.5, 0.25)),
    );
    assert!(
        near(pixel(&shown, [40, 30]), [255, 128, 128]),
        "{:?}",
        pixel(&shown, [40, 30])
    );
    assert_eq!(pixel(&shown, [4, 4]), [255; 3]);
    // Inverted it shows black (the red hidden); halfway to white shows half the red.
    let inverted = with_mask(&red(), true);
    assert_eq!(pixel(&cold(&core, &inverted), [40, 30]), [255; 3], "hidden");
    let lifted = cold(&core, &adjusted(&inverted, None, hsb(0.5, 0.5, 0.75)));
    assert!(
        near(pixel(&lifted, [40, 30]), [255, 128, 128]),
        "{:?}",
        pixel(&lifted, [40, 30])
    );
}

/// Preview frames of `steps` evaluated incrementally on one target from `base`, as the canvas
/// shows them; the last one equals the committed artwork reloaded and evaluated cold.
fn previews_equal_the_commit(base: &Artwork, clip: Option<&RegionMask>, steps: &[Adjustment]) {
    let core = gpu::core();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(base), &[PixelRect::full(SIZE)])
        .expect("base");
    let mut shown = base.clone();
    for adjustment in steps {
        let preview = adjusted(base, clip, adjustment.clone());
        let regions = changed_regions(&shown, &preview);
        evaluator
            .evaluate(&request(&preview), &regions)
            .expect("frame");
        shown = preview;
    }
    let last = target.read(&core).expect("the last preview frame");
    let committed = Artwork::from_storage(shown.to_parts()).expect("reloaded");
    assert_eq!(last, cold(&core, &committed), "preview and committed match");
}

fn steps() -> Vec<Adjustment> {
    let lifted = ToneCurve::new(&[[0.0, 0.1], [0.4, 0.7], [1.0, 0.9]]).expect("curve");
    vec![
        hsb(0.2, 0.8, 0.6),
        hsb(0.7, 0.3, 0.45),
        red_curve_halved(),
        Adjustment::Curves(Curves::default().with(ToneChannel::Composite, lifted)),
        hsb(0.55, 0.9, 0.35),
        Adjustment::Blur(Blur::new(2.5)),
        Adjustment::Sharpen(Sharpen::new(0.6)),
        Adjustment::Blur(Blur::new(7.0)),
    ]
}

#[test]
fn the_last_preview_equals_the_committed_adjustment() {
    let painted = stroked(&red());
    previews_equal_the_commit(&painted, None, &steps());
    let clip = rectangle([30.0, 10.0], [80.0, 40.0]);
    previews_equal_the_commit(&painted, Some(&clip), &steps());
    for inverted in [false, true] {
        let masked = stroked(&with_mask(&red(), inverted));
        previews_equal_the_commit(&masked, None, &steps());
        previews_equal_the_commit(&masked, Some(&clip), &steps());
    }
}

/// The banding measurement (STATUS): a soft black stroke across a white layer is a smooth
/// ramp; a strong S curve (slope 4 in the middle) through 8-bit tables makes neighbouring
/// pixels several levels apart where the ramp's were one or two apart. Returns the largest step
/// between neighbours down a column across the stroke, and the levels used, before and after.
fn steps_across(core: &Core, artwork: &Artwork) -> (u8, usize) {
    let mut target = CanvasTarget::new(core, TALL).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(TALL)])
        .expect("evaluate");
    let shown = target.read(core).expect("readback");
    let at = |y: u32| shown.pixels()[((y * TALL[0] + TALL[0] / 2) * 4) as usize];
    let column: Vec<u8> = (60..300).map(at).collect();
    let largest = column
        .windows(2)
        .map(|w| w[0].abs_diff(w[1]))
        .max()
        .unwrap_or(0);
    let mut levels = column.clone();
    levels.sort_unstable();
    levels.dedup();
    (largest, levels.len())
}

#[test]
fn a_strong_curve_on_a_soft_gradient_shows_steps_of_several_levels() {
    let core = gpu::core();
    let white = Color::rgb(1.0, 1.0, 1.0).expect("white");
    let tall = CanvasSize::new(TALL[0], TALL[1]).expect("size");
    let everything = [[0.0, 0.0], [64.0, 0.0], [64.0, 600.0], [0.0, 600.0]];
    let paper = Artwork::new(tall)
        .fill_region(RegionMask::polygon(tall, &everything).expect("all"), white)
        .expect("white layer")
        .artwork;
    let curve = |points: &[[f32; 2]]| ToneCurve::new(points).expect("curve");
    let soft = openpaint_core::Dynamics::new(
        openpaint_core::Curve::new(&[[0.0, 1.0]]).expect("size"),
        openpaint_core::Curve::new(&[[0.0, 1.0]]).expect("speed"),
    )
    .expect("dynamics");
    let recipe = BrushRecipe::new(1.0, soft.clone(), soft, openpaint_core::Grain::NONE);
    // Radius 250 across the column: about one level a pixel.
    let style = StrokeStyle::new(Arc::new(recipe.expect("soft")), 500.0, 1.0, Color::BLACK);
    let sample = |x: f32| {
        let at = CanvasPoint::new(x, 300.0).expect("point");
        StrokeSample::new(at, 1.0, 0.0, 0.0, 0).expect("sample")
    };
    let mut session = paper
        .begin_stroke(style.expect("style"), sample(0.0))
        .expect("begin");
    session.extend(&[sample(64.0)]).expect("extend");
    let ramp = session.finish().artwork;
    let s_curve = curve(&[[0.0, 0.0], [0.4, 0.1], [0.6, 0.9], [1.0, 1.0]]);
    let strong = Adjustment::Curves(Curves::default().with(ToneChannel::Composite, s_curve));
    let before = steps_across(&core, &ramp);
    let after = steps_across(&core, &adjusted(&ramp, None, strong));
    let brighter = steps_across(&core, &adjusted(&ramp, None, hsb(0.5, 0.5, 0.8)));
    eprintln!("ramp {before:?}, strong curve {after:?}, brightness 80 % {brighter:?}");
    assert!(before.0 <= 2, "the ramp is smooth: {before:?}");
    // Measured 2026-10-04: the ramp (2, 204), the curve (11, 84), brightness (1, 102).
    assert!(
        after.0 > 2 * before.0 && after.0 <= 12,
        "steps of levels: {after:?}"
    );
    assert!(
        brighter.0 <= before.0,
        "brightness works in floats: {brighter:?}"
    );
}

#[test]
fn after_an_adjustment_a_stroke_and_undo_incremental_equals_cold() {
    let core = gpu::core();
    let base = red();
    let turned = adjusted(&base, None, hsb(0.3, 0.7, 0.6));
    let reopened = Artwork::from_storage(turned.to_parts()).expect("reopened");
    let later = stroked(&reopened);
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    for (from, to) in [(&base, &reopened), (&reopened, &later)] {
        evaluator
            .evaluate(&request(to), &changed_regions(from, to))
            .expect("step");
    }
    assert_eq!(target.read(&core).expect("read"), cold(&core, &later));
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &changed_regions(&later, &base))
        .expect("undo both");
    assert_eq!(target.read(&core).expect("read"), cold(&core, &base));
}
