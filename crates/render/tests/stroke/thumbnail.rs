//! The gallery thumbnail matches the artwork: the GPU reduction of an evaluated canvas equals,
//! byte for byte, the mean of the canvas pixels in each box computed on the CPU, for an even and
//! an uneven scale.

use openpaint_core::{ArtworkCommand, MaskCommand};
use openpaint_render::{
    CanvasTarget, PixelRect, RegionEvaluator, thumbnail_reference, thumbnail_size,
};

use crate::fixture::{SIZE, blank, core, diagonal, pixel, request, wave, with_layer, with_stroke};

#[test]
fn the_thumbnail_is_the_box_mean_of_the_canvas() {
    let core = core();
    let artwork = with_stroke(&with_layer(&with_stroke(&blank(), wave()), 0.6), diagonal());
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let canvas = target.read(&core).expect("readback");
    for max in [[80, 80], [97, 61], [448, 336]] {
        let thumbnail = core.thumbnail(&target, max).expect("thumbnail");
        let size = thumbnail_size(SIZE, max);
        assert_eq!(thumbnail.size(), size);
        assert!(
            thumbnail.pixels() == thumbnail_reference(&canvas, size).as_slice(),
            "the thumbnail at {size:?} differs from the canvas's box means"
        );
    }
    let small = core.thumbnail(&target, [80, 80]).expect("thumbnail");
    assert_eq!(
        pixel(&small, [0, 0]),
        [255, 255, 255, 255],
        "paper stays white"
    );
    assert_ne!(
        thumbnail_reference(&canvas, [80, 50]),
        vec![255; 80 * 50 * 4],
        "the strokes show"
    );
}

/// A layer row's picture (S38) is the layer alone, or its mask's grey, reduced by box means:
/// faded, hidden and masked layers show their own paint whole.
#[test]
fn a_layer_thumbnail_is_the_box_mean_of_the_layer_alone() {
    let core = core();
    let under = with_stroke(&blank(), wave());
    let faded = with_stroke(&with_layer(&under, 0.3), diagonal());
    let top = faded.active();
    let masked = faded
        .apply(ArtworkCommand::Mask(MaskCommand::Add(top)))
        .expect("mask")
        .artwork;
    let artwork = with_stroke(&masked, wave());
    for (layer, mask) in [(artwork.layers()[0].id(), false), (top, false), (top, true)] {
        let alone = if mask {
            core.mask_pixels(&artwork, layer)
        } else {
            core.layer_pixels(&artwork, layer)
        }
        .expect("layer pixels");
        for max in [[77, 58], [448, 336]] {
            let picture = core
                .layer_thumbnail(&artwork, (layer, mask), max)
                .expect("thumbnail");
            let size = thumbnail_size(SIZE, max);
            assert_eq!(picture.size(), size);
            assert!(
                picture.pixels() == thumbnail_reference(&alone, size).as_slice(),
                "{layer:?} (mask {mask}) at {size:?} differs from its pixels' box means"
            );
        }
    }
    let empty = with_layer(&artwork, 1.0);
    let clear = core
        .layer_thumbnail(&empty, (empty.active(), false), [40, 30])
        .expect("empty");
    assert!(
        clear.pixels().iter().all(|&byte| byte == 0),
        "nothing painted: transparent"
    );
}
