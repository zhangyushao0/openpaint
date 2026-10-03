//! The gallery thumbnail matches the artwork: the GPU reduction of an evaluated canvas equals,
//! byte for byte, the mean of the canvas pixels in each box computed on the CPU, for an even and
//! an uneven scale.

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
