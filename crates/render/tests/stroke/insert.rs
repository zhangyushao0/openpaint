//! An inserted picture on the GPU (S26): a new layer whose content is a placed patch with
//! nothing lifted. Unmoved it shows its pixels exactly over the paper; every placement a hand
//! passes through, evaluated incrementally on one target, ends bit-equal to the committed
//! artwork reloaded and evaluated cold; a mask and a clipping layer on it evaluate incrementally
//! as cold too.

use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, Color, LayerCommand, MaskCommand, Patch, Placement,
};
use openpaint_render::{CanvasTarget, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{SIZE, blank, cold, core, diagonal, pixel, plain, request, sample, wave};
use crate::fixture::{with_layer, with_stroke};

const AT: [u32; 2] = [100, 60];
const PICTURE: [u32; 2] = [96, 64];

/// A 96 x 64 picture, premultiplied: an opaque gradient with a transparent hole and a half
/// transparent band, as `open_picture` hands it over.
fn picture() -> Patch {
    let canvas = CanvasSize::new(SIZE[0], SIZE[1]).expect("size");
    let mut pixels = Vec::new();
    for y in 0..PICTURE[1] {
        for x in 0..PICTURE[0] {
            let (dx, dy) = (x as i32 - 48, y as i32 - 32);
            let pixel = if dx * dx + dy * dy < 100 {
                [0, 0, 0, 0]
            } else if y < 8 {
                [100, 20, 10, 128]
            } else {
                [(x * 2) as u8, (y * 3) as u8, 160, 255]
            };
            pixels.extend(pixel);
        }
    }
    Patch::new(canvas, AT, PICTURE, pixels).expect("patch")
}

/// `artwork` with the picture inserted above its active layer, placed by `placement`.
fn inserted(artwork: &Artwork, placement: Placement) -> Artwork {
    let layered = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .expect("layer")
        .artwork;
    layered
        .transform_region(None, picture(), placement)
        .expect("placed")
        .artwork
}

fn drawn() -> Artwork {
    with_stroke(&blank(), wave())
}

#[test]
fn an_unmoved_picture_shows_its_pixels_over_the_paper() {
    let core = core();
    let image = cold(&core, &inserted(&blank(), Placement::IDENTITY));
    let patch = picture();
    for y in 0..PICTURE[1] {
        for x in 0..PICTURE[0] {
            let at = ((y * PICTURE[0] + x) * 4) as usize;
            let p = &patch.pixels()[at..at + 4];
            let alpha = u32::from(p[3]);
            // Premultiplied over white: p + 255 (1 - a).
            let want: Vec<u8> = (0..3)
                .map(|c| (u32::from(p[c]) + (255 * (255 - alpha) + 127) / 255).min(255) as u8)
                .collect();
            let got = pixel(&image, [AT[0] + x, AT[1] + y]);
            for c in 0..3 {
                assert!(
                    got[c].abs_diff(want[c]) <= 1,
                    "at {x}, {y}: {got:?} against {want:?}"
                );
            }
            if alpha == 255 {
                assert_eq!(got[..3], p[..3], "opaque pixels exactly at {x}, {y}");
            }
        }
    }
    assert_eq!(
        pixel(&image, [AT[0] - 1, AT[1]]),
        [255; 4],
        "nothing outside"
    );
}

#[test]
fn every_placement_evaluated_incrementally_ends_equal_to_the_committed_picture() {
    let core = core();
    let base = drawn();
    let moved = Placement::moved([21.5, -13.25]).unwrap();
    let scaled = moved
        .then(Placement::around([120.0, 50.0], 1.6, 0.0, [false; 2]).unwrap())
        .unwrap();
    let turned = scaled
        .then(Placement::around([160.0, 100.0], 1.0, 0.45, [true, false]).unwrap())
        .unwrap();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    let mut shown = base.clone();
    for placement in [Placement::IDENTITY, moved, scaled, turned] {
        let preview = inserted(&base, placement);
        evaluator
            .evaluate(&request(&preview), &changed_regions(&shown, &preview))
            .expect("frame");
        shown = preview;
    }
    let last = target.read(&core).expect("the last frame");
    let committed = Artwork::from_storage(shown.to_parts()).expect("reloaded");
    assert_eq!(last, cold(&core, &committed), "shown = committed, cold");
    // Undone: the version before, as cold.
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&base), &changed_regions(&committed, &base))
        .expect("undo");
    assert_eq!(target.read(&core).expect("read"), cold(&core, &base));
}

#[test]
fn a_mask_and_a_clipping_layer_on_the_picture_evaluate_incrementally_as_cold() {
    let core = core();
    let base = drawn();
    let picture_layer = inserted(&base, Placement::moved([10.0, 5.0]).unwrap());
    let layer = picture_layer.active();
    let masked = picture_layer
        .apply(ArtworkCommand::Mask(MaskCommand::Add(layer)))
        .expect("mask")
        .artwork;
    let hidden = (
        plain(14.0, 1.0, Color::BLACK),
        vec![sample(90.0, 90.0, 1.0, 0), sample(230.0, 100.0, 1.0, 8)],
    );
    let masked = with_stroke(&masked, hidden);
    let clipping = with_layer(&masked, 1.0);
    let clip = LayerCommand::SetClipping {
        layer: clipping.active(),
        clipping: true,
    };
    let clipping = clipping
        .apply(ArtworkCommand::Layer(clip))
        .expect("clip")
        .artwork;
    let painted = with_stroke(&clipping, diagonal());
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    let steps = [&base, &picture_layer, &masked, &clipping, &painted];
    for pair in steps.windows(2) {
        evaluator
            .evaluate(&request(pair[1]), &changed_regions(pair[0], pair[1]))
            .expect("step");
    }
    let last = target.read(&core).expect("read");
    let reloaded = Artwork::from_storage(painted.to_parts()).expect("reloaded");
    assert_eq!(last, cold(&core, &reloaded));
    // The diagonal shows only on the picture: off it, the canvas is as without it.
    let without = cold(&core, &clipping);
    assert_eq!(pixel(&last, [62, 32]), pixel(&without, [62, 32]));
}
