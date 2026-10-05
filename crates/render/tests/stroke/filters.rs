//! Gaussian Blur and Sharpen (S33, D59) on the GPU: a filter's layer pixels equal a CPU
//! Gaussian (or unsharp mask) of the layer's pixels before it, premultiplied, so red on
//! transparent never darkens and the canvas's edge pixel continues past it; only the selection
//! changes, with a hard edge; on a mask the grey it shows is filtered; a filter in a clipping
//! layer shows only on its base, one in a full group as ungrouped; through filters, smudges,
//! later strokes, a transform record and undo, incremental equals cold; banded exports at 2x and
//! 4x equal one piece.

use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, Blur, Color, GroupCommand, LayerCommand, LayerId,
    MaskCommand, Placement, RegionMask, Sharpen, StrokeStyle, StrokeTool,
};
use openpaint_render::changed_regions;
use openpaint_render::{Banding, CanvasTarget, PixelRect, RegionEvaluator};

use crate::filter_reference::{largest, mean, reference};
use crate::fixture::{
    SIZE, airbrush, blank, cold, core, diagonal, pixel, plain, request, sample, wave, with_layer,
    with_stroke,
};
use crate::scale::{export, whole};

const LAYER: LayerId = LayerId::new(1);

fn region(inside: impl Fn(u32, u32) -> bool) -> RegionMask {
    let pixels: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| inside(at % SIZE[0], at / SIZE[0]))
        .collect();
    let canvas = blank().canvas();
    RegionMask::from_pixels(canvas, &pixels).expect("mask")
}

fn color(r: f32, g: f32, b: f32) -> Color {
    Color::rgb(r, g, b).expect("colour")
}

fn filled(artwork: &Artwork, inside: impl Fn(u32, u32) -> bool, colour: Color) -> Artwork {
    artwork
        .fill_region(region(inside), colour)
        .expect("fill")
        .artwork
}

fn adjusted(artwork: &Artwork, clip: Option<RegionMask>, adjustment: Adjustment) -> Artwork {
    let applied = artwork.adjust_region(clip, adjustment);
    applied.expect("adjusted").artwork
}

fn blur(radius: f32) -> Adjustment {
    Adjustment::Blur(Blur::new(radius))
}

fn sharpen(amount: f32) -> Adjustment {
    Adjustment::Sharpen(Sharpen::new(amount))
}

/// Opaque red on the left, touching the canvas's edge, and a translucent blue wave.
fn painted() -> Artwork {
    let red = filled(
        &blank(),
        |x, y| x < 120 && (40..160).contains(&y),
        color(1.0, 0.0, 0.0),
    );
    with_stroke(&red, wave())
}

#[test]
fn a_blur_is_the_cpu_gaussian_of_the_layer_before_it() {
    let core = core();
    let before = painted();
    let source = core.layer_pixels(&before, LAYER).expect("before");
    // From a deviation of 4 pixels on cells of 2 x 2, from 8 on cells of 4 x 4 (S35).
    let radii = [
        (0.4, 1),
        (2.5, 1),
        (3.9, 1),
        (4.0, 2),
        (6.0, 2),
        (9.0, 3),
        (16.0, 3),
        (32.0, 3),
    ];
    for (radius, most) in radii {
        let after = adjusted(&before, None, blur(radius));
        let shown = core.layer_pixels(&after, LAYER).expect("after");
        let exact = reference(&source, radius, 0.0);
        let (worst, mean) = (
            largest(shown.pixels(), &exact),
            mean(shown.pixels(), &exact),
        );
        println!("radius {radius}: largest {worst}, mean {mean:.4}");
        assert!(
            worst <= most && mean < 0.1,
            "radius {radius}: {worst}, {mean}"
        );
    }
}

#[test]
fn premultiplied_red_never_darkens_and_the_edge_continues() {
    let core = core();
    let red = filled(
        &blank(),
        |x, y| x < 120 && (40..160).contains(&y),
        color(1.0, 0.0, 0.0),
    );
    let blurred = adjusted(&red, None, blur(6.0));
    let canvas = cold(&core, &blurred);
    // Red over white keeps its red channel at 255 wherever the blur reaches: no dark rim.
    let reds = canvas.pixels().chunks(4).map(|p| p[0]);
    assert!(reds.into_iter().all(|r| r >= 254), "a dark rim");
    let layer = core.layer_pixels(&blurred, LAYER).expect("layer");
    // The fill reaches the canvas's left edge: the edge pixel continues, so it stays opaque.
    assert_eq!(pixel(&layer, [0, 100]), [255, 0, 0, 255]);
    // Across its right edge the blur goes half way: transparent beyond, opaque inside.
    let [_, _, _, edge] = pixel(&layer, [120, 100]);
    assert!((100..=155).contains(&edge), "{edge}");
    assert_eq!(pixel(&layer, [160, 100]), [0; 4]);
    // Straight colour stays pure red where there is any alpha.
    for x in 110..150 {
        let [r, g, b, a] = pixel(&layer, [x, 100]);
        assert!(g == 0 && b == 0 && r.abs_diff(a) <= 1, "{x}: {r} {a}");
    }
}

#[test]
fn sharpening_is_the_cpu_unsharp_mask_and_leaves_flat_colour() {
    let core = core();
    let grey = filled(&blank(), |x, y| x < 200 && y < 150, color(0.5, 0.5, 0.5));
    let before = filled(
        &grey,
        |x, y| (60..140).contains(&x) && (50..100).contains(&y),
        color(0.2, 0.3, 0.6),
    );
    let source = core.layer_pixels(&before, LAYER).expect("before");
    let after = adjusted(&before, None, sharpen(0.5));
    let shown = core.layer_pixels(&after, LAYER).expect("after");
    let strength = 0.5 * Sharpen::STRENGTH;
    let worst = largest(
        shown.pixels(),
        &reference(&source, Sharpen::RADIUS, strength),
    );
    assert!(worst <= 1, "{worst}");
    assert_eq!(
        pixel(&shown, [20, 20]),
        pixel(&source, [20, 20]),
        "flat colour stays"
    );
    // Each side of an edge moves away from the other.
    let [inside, ..] = pixel(&shown, [61, 75]);
    let [outside, ..] = pixel(&shown, [58, 75]);
    assert!(inside < pixel(&source, [61, 75])[0] && outside > pixel(&source, [58, 75])[0]);
}

#[test]
fn a_selection_limits_the_filter_with_a_hard_edge() {
    let core = core();
    let before = painted();
    let source = core.layer_pixels(&before, LAYER).expect("before");
    let clip = region(|x, _| x < 90);
    let after = adjusted(&before, Some(clip), blur(5.0));
    let shown = core.layer_pixels(&after, LAYER).expect("after");
    let all = reference(&source, 5.0, 0.0);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let i = ((y * SIZE[0] + x) * 4) as usize;
            let got = &shown.pixels()[i..i + 4];
            if x < 90 {
                assert!(largest(got, &all[i..i + 4]) <= 1, "inside at {x}, {y}");
            } else {
                assert_eq!(got, &source.pixels()[i..i + 4], "outside at {x}, {y}");
            }
        }
    }
}

#[test]
fn a_blur_on_a_mask_blurs_the_grey_it_shows() {
    let core = core();
    let blue = filled(&blank(), |_, _| true, color(0.1, 0.2, 0.9));
    let masked = blue
        .apply(ArtworkCommand::Mask(MaskCommand::Add(LAYER)))
        .expect("mask")
        .artwork;
    let hidden = filled(&masked, |x, y| x < 160 && y > 30, Color::BLACK);
    let grey = with_stroke(&hidden, diagonal());
    for inverted in [false, true] {
        let before = if inverted {
            grey.apply(ArtworkCommand::Mask(MaskCommand::Invert(LAYER)))
                .expect("invert")
                .artwork
        } else {
            grey.clone()
        };
        let source = core.mask_pixels(&before, LAYER).expect("before");
        let after = adjusted(&before, None, blur(4.0));
        let shown = core.mask_pixels(&after, LAYER).expect("after");
        let worst = largest(shown.pixels(), &reference(&source, 4.0, 0.0));
        assert!(worst <= 2, "inverted {inverted}: {worst}");
    }
}

#[test]
fn a_filter_in_a_clipping_layer_shows_on_its_base_and_in_a_group_as_ungrouped() {
    let core = core();
    let base = filled(
        &blank(),
        |x, y| (80..240).contains(&x) && (50..150).contains(&y),
        color(0.9, 0.9, 0.8),
    );
    let top = with_layer(&base, 1.0);
    let layer = top.active();
    let clip = LayerCommand::SetClipping {
        layer,
        clipping: true,
    };
    let clipped = top
        .apply(ArtworkCommand::Layer(clip))
        .expect("clip")
        .artwork;
    let before = with_stroke(&clipped, diagonal());
    let source = core.layer_pixels(&before, layer).expect("before");
    let after = adjusted(&before, None, blur(6.0));
    let shown = core.layer_pixels(&after, layer).expect("after");
    assert!(largest(shown.pixels(), &reference(&source, 6.0, 0.0)) <= 1);
    let canvas = cold(&core, &after);
    for at in [[40, 100], [260, 100], [160, 20], [160, 180]] {
        assert_eq!(pixel(&canvas, at), [255; 4], "off the base at {at:?}");
    }
    // The blurred stroke reaches past where it was drawn, on the base.
    assert_ne!(
        pixel(&canvas, [90, 60]),
        pixel(&cold(&core, &base), [90, 60])
    );
    let grouped = after
        .apply(ArtworkCommand::Group(GroupCommand::Create { layer }))
        .expect("group")
        .artwork;
    assert_eq!(
        cold(&core, &grouped),
        canvas,
        "a full group as its layers ungrouped"
    );
}

/// A smudge across the left half.
fn smudge() -> (StrokeStyle, Vec<openpaint_core::StrokeSample>) {
    let style = StrokeStyle::new(airbrush(), 30.0, 0.8, Color::BLACK)
        .expect("style")
        .with_tool(StrokeTool::Smudge);
    let path = (0..20u16)
        .map(|i| {
            sample(
                60.0 + 5.0 * f32::from(i),
                60.0 + 3.0 * f32::from(i),
                0.9,
                u32::from(i) * 8,
            )
        })
        .collect();
    (style, path)
}

/// Every filter at work: a blur (on cells of 4 x 4 pixels, S35), a smudge after it, sharpening
/// inside a selection after that, a stroke on top; a second layer with a whole-layer record over
/// a blur on cells of 2 x 2.
pub fn filtered() -> Artwork {
    let blurred = adjusted(&painted(), None, blur(9.0));
    let smudged = with_stroke(&blurred, smudge());
    let selection = region(|x, y| x > 100 && y < 120);
    let sharpened = adjusted(&smudged, Some(selection), sharpen(0.8));
    let stroked = with_stroke(&sharpened, diagonal());
    let second = with_stroke(
        &with_layer(&stroked, 0.7),
        (
            plain(16.0, 1.0, color(0.1, 0.6, 0.2)),
            vec![sample(200.0, 40.0, 1.0, 0), sample(300.0, 160.0, 1.0, 8)],
        ),
    );
    let soft = adjusted(&second, None, blur(5.0));
    let moved = Placement::around([250.0, 100.0], 1.4, 0.3, [false, false]).expect("placement");
    soft.place_region(None, moved).expect("record").artwork
}

#[test]
fn incremental_equals_cold_through_filters_smudges_records_and_undo() {
    let core = core();
    let blank = blank();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&blank), &[PixelRect::full(SIZE)])
        .expect("first");
    let painted = painted();
    let blurred = adjusted(&painted, None, blur(3.0));
    let smudged = with_stroke(&blurred, smudge());
    let sharpened = adjusted(
        &smudged,
        Some(region(|x, y| x > 100 && y < 120)),
        sharpen(0.8),
    );
    let last = filtered();
    let steps = [
        &painted, &blurred, &smudged, &sharpened, &last, &sharpened, &blurred, &last, &blank,
    ];
    let mut shown = blank.clone();
    for (index, artwork) in steps.into_iter().enumerate() {
        let regions = changed_regions(&shown, artwork);
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(artwork), &regions)
            .expect("step");
        let incremental = target.read(&core).expect("read");
        assert_eq!(incremental, cold(&core, artwork), "step {index}");
        shown = artwork.clone();
    }
}

#[test]
fn a_stroke_drawn_after_a_filter_equals_its_commit() {
    let core = core();
    let blurred = adjusted(&painted(), None, blur(4.0));
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&blurred), &[PixelRect::full(SIZE)])
        .expect("first");
    for (style, samples) in [diagonal(), smudge()] {
        let mut session = blurred.begin_stroke(style, samples[0]).expect("begin");
        for chunk in samples[1..].chunks(5) {
            session.extend(chunk).expect("extend");
            let frame = openpaint_render::FrameRequest {
                artwork: &blurred,
                active: Some(session.active()),
                background: Color::WHITE,
            };
            RegionEvaluator::new(&core, &mut target)
                .evaluate(&frame, &[])
                .expect("frame");
        }
        let committed = session.finish().artwork;
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&committed), &[])
            .expect("commit");
        assert_eq!(target.read(&core).expect("read"), cold(&core, &committed));
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&blurred), &changed_regions(&committed, &blurred))
            .expect("undo");
    }
}

#[test]
fn banded_exports_equal_one_piece_at_every_scale() {
    let core = core();
    let artwork = filtered();
    let bands = Banding {
        rows: 37,
        columns: 100,
    };
    assert_eq!(
        export(&core, &artwork, 1, bands),
        cold(&core, &artwork),
        "1x is the canvas"
    );
    for scale in [2, 4] {
        assert_eq!(
            export(&core, &artwork, scale, bands),
            whole(&core, &artwork, scale),
            "{scale}x"
        );
    }
}
