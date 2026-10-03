//! Fills on the GPU: exactly the mask with a hard edge, at their place among the strokes,
//! Alpha Lock, incremental = cold after a fill and its undo, and layer sampling.

use openpaint_core::{Artwork, ArtworkCommand, CanvasSize, Color, LayerCommand, RegionMask};
use openpaint_render::{CanvasTarget, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{SIZE, blank, cold, core, pixel, plain, request, sample, with_stroke};

const RED: [u8; 4] = [255, 0, 0, 255];
const WHITE: [u8; 4] = [255; 4];

/// The disc of radius 50 about (160, 100) with a 1-pixel hole at its centre.
fn disc() -> RegionMask {
    let mut inside = vec![false; (SIZE[0] * SIZE[1]) as usize];
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let (dx, dy) = (x as f32 - 160.0, y as f32 - 100.0);
            inside[(y * SIZE[0] + x) as usize] =
                dx * dx + dy * dy <= 2500.0 && (dx, dy) != (0.0, 0.0);
        }
    }
    let canvas = CanvasSize::new(SIZE[0], SIZE[1]).expect("size");
    RegionMask::from_pixels(canvas, &inside).expect("mask")
}

fn red() -> Color {
    Color::rgb(1.0, 0.0, 0.0).expect("red")
}

/// A horizontal black bar across the canvas at y = 100.
fn bar(artwork: &Artwork) -> Artwork {
    let style = plain(10.0, 1.0, Color::BLACK);
    with_stroke(
        artwork,
        (
            style,
            vec![sample(20.0, 100.0, 1.0, 0), sample(300.0, 100.0, 1.0, 8)],
        ),
    )
}

#[test]
fn a_fill_paints_exactly_its_mask_with_a_hard_edge() {
    let core = core();
    let filled = blank().fill_region(disc(), red()).expect("fill").artwork;
    let image = cold(&core, &filled);
    let mask = disc();
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let expected = if mask.contains([x, y]) { RED } else { WHITE };
            assert_eq!(pixel(&image, [x, y]), expected, "at {x}, {y}");
        }
    }
}

#[test]
fn a_fill_lies_between_the_strokes_before_and_after_it() {
    let core = core();
    let under = bar(&blank())
        .fill_region(disc(), red())
        .expect("fill")
        .artwork;
    assert_eq!(
        pixel(&cold(&core, &under), [140, 100]),
        RED,
        "the fill covers the bar"
    );
    let over = bar(&under);
    assert_eq!(
        pixel(&cold(&core, &over), [140, 100])[..3],
        [0, 0, 0],
        "a later bar covers it"
    );
    assert_eq!(pixel(&cold(&core, &over), [160, 70]), RED);
}

#[test]
fn with_alpha_lock_a_fill_colours_only_the_layers_paint() {
    let core = core();
    let drawn = bar(&blank());
    let lock = LayerCommand::SetAlphaLocked {
        layer: drawn.active(),
        locked: true,
    };
    let locked = drawn
        .apply(ArtworkCommand::Layer(lock))
        .expect("lock")
        .artwork;
    let filled = locked.fill_region(disc(), red()).expect("fill").artwork;
    let image = cold(&core, &filled);
    assert_eq!(pixel(&image, [140, 100]), RED, "on the bar");
    assert_eq!(
        pixel(&image, [160, 70]),
        WHITE,
        "no paint there: nothing to colour"
    );
    assert_eq!(
        pixel(&image, [40, 100])[..3],
        [0, 0, 0],
        "the bar outside the mask"
    );
}

#[test]
fn incremental_evaluation_after_a_fill_and_its_undo_equals_cold() {
    let core = core();
    let base = bar(&blank());
    let filled = base.fill_region(disc(), red()).expect("fill").artwork;
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    let regions = changed_regions(&base, &filled);
    evaluator
        .evaluate(&request(&filled), &regions)
        .expect("fill");
    assert_eq!(target.read(&core).expect("read"), cold(&core, &filled));
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    let regions = changed_regions(&filled, &base);
    evaluator.evaluate(&request(&base), &regions).expect("undo");
    assert_eq!(target.read(&core).expect("read"), cold(&core, &base));
}

#[test]
fn layer_pixels_are_one_layers_premultiplied_paint_even_hidden() {
    let core = core();
    let style = plain(10.0, 0.5, red());
    let line = vec![sample(20.0, 50.0, 1.0, 0), sample(300.0, 50.0, 1.0, 8)];
    let drawn = with_stroke(&blank(), (style, line));
    let layer = drawn.active();
    let other = bar(&drawn
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .expect("create")
        .artwork);
    let hide = LayerCommand::SetVisible {
        layer,
        visible: false,
    };
    let hidden = other
        .apply(ArtworkCommand::Layer(hide))
        .expect("hide")
        .artwork;
    let pixels = core.layer_pixels(&hidden, layer).expect("sample");
    assert_eq!(pixels.size(), SIZE);
    assert_eq!(
        pixel(&pixels, [100, 100]),
        [0; 4],
        "the other layer's bar is not in it"
    );
    let [r, g, b, a] = pixel(&pixels, [100, 50]);
    assert_eq!((g, b), (0, 0));
    assert!(
        (127..=128).contains(&a) && r == a,
        "half red, premultiplied: {r} {a}"
    );
}
