//! The display cache's round trip (S54): a canvas evaluated cold, copied out and put into a new
//! target holds the cold canvas's texels bit for bit, at 8 bits and at ten (every code), and
//! evaluating an edit over it gives what a cold evaluation of the edited artwork gives. The
//! gallery picture enlarged covers a new target.

use std::sync::Arc;

use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, Curve, Dynamics, Grain, StrokeSample,
    StrokeStyle,
};

use crate::core::{Core, test_core};
use crate::gpu::TEN_BITS;
use crate::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, changed_regions};

const SIZE: [u32; 2] = [150, 90];

/// A stroke of `softness` from `from` to `to`, wide and faint enough to leave many levels.
fn stroked(artwork: &Artwork, softness: f32, [from, to]: [[f32; 2]; 2]) -> Artwork {
    let flat = || Curve::new(&[[0.0, 1.0]]).expect("curve");
    let dynamics = || Dynamics::new(flat(), flat()).expect("dynamics");
    let recipe = BrushRecipe::new(softness, dynamics(), dynamics(), Grain::NONE).expect("recipe");
    let colour = Color::rgb(0.8, 0.3, 0.1).expect("colour");
    let style = StrokeStyle::new(Arc::new(recipe), 40.0, 0.4, colour).expect("style");
    let sample = |[x, y]: [f32; 2], time_ms| {
        let at = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(at, 1.0, 0.0, 0.0, time_ms).expect("sample")
    };
    let mut session = artwork.begin_stroke(style, sample(from, 0)).expect("begin");
    session.extend(&[sample(to, 16)]).expect("extend");
    session.finish().artwork
}

fn artwork() -> Artwork {
    let blank = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
    let soft = stroked(&blank, 1.0, [[-20.0, 30.0], [170.0, 60.0]]);
    stroked(&soft, 0.0, [[20.0, 80.0], [130.0, 5.0]])
}

/// `target` with `regions` of `artwork` evaluated into it.
fn evaluated(core: &Core, target: &mut CanvasTarget, artwork: &Artwork, regions: &[PixelRect]) {
    let request = FrameRequest {
        artwork,
        active: None,
        background: Color::WHITE,
    };
    RegionEvaluator::new(core, target)
        .evaluate(&request, regions)
        .expect("evaluated");
}

/// Its texels as it holds them.
fn texels(core: &Core, target: &CanvasTarget) -> Vec<u8> {
    target.copy_out(core).bytes(core).expect("read back")
}

/// The round trip for targets `made` makes: cold, copied out, put in a new one; then an edit.
fn round_trip(core: &Core, made: impl Fn() -> CanvasTarget) -> (Vec<u8>, usize) {
    let before = artwork();
    let all = [PixelRect::full(SIZE)];
    let mut cold = made();
    evaluated(core, &mut cold, &before, &all);
    let cached = texels(core, &cold);
    assert_eq!(cached.len(), (SIZE[0] * SIZE[1] * 4) as usize);
    let mut shown = made();
    shown.show_cached(core, &cached, true).expect("shown");
    assert!(!shown.pending());
    assert_eq!(texels(core, &shown), cached, "the cache bit for bit");
    assert_eq!(
        shown.read(core).expect("read"),
        cold.read(core).expect("read")
    );
    let after = stroked(&before, 0.5, [[10.0, 10.0], [140.0, 85.0]]);
    evaluated(core, &mut shown, &after, &changed_regions(&before, &after));
    let mut edited = made();
    evaluated(core, &mut edited, &after, &all);
    assert_eq!(
        texels(core, &shown),
        texels(core, &edited),
        "an edit over it = cold"
    );
    let distinct = |bytes: &[u8]| {
        let mut words: Vec<&[u8; 4]> = bytes.as_chunks::<4>().0.iter().collect();
        words.sort_unstable();
        words.dedup();
        words.len()
    };
    (cached.clone(), distinct(&cached))
}

#[test]
fn a_cached_canvas_is_the_cold_canvas_bit_for_bit_at_eight_and_ten_bits() {
    let core = test_core();
    let (eight, levels) = round_trip(core, || CanvasTarget::exact(core, SIZE).expect("target"));
    let shown = CanvasTarget::new(core, SIZE).expect("target");
    assert_eq!(
        shown.format_code(),
        if core.gpu.display == TEN_BITS { 10 } else { 8 }
    );
    let (display, codes) = round_trip(core, || CanvasTarget::new(core, SIZE).expect("target"));
    if core.gpu.display != TEN_BITS {
        eprintln!("this device shows the canvas at 8 bits: the screen's round trip was 8-bit");
        assert_eq!(display, eight);
        return;
    }
    assert_eq!(
        CanvasTarget::exact(core, SIZE)
            .expect("target")
            .format_code(),
        8
    );
    assert!(levels >= 40, "{levels} colours at 8 bits");
    assert!(
        codes > levels,
        "{codes} ten-bit texels over {levels} 8-bit colours"
    );
}

#[test]
fn the_gallery_picture_covers_a_new_target_enlarged() {
    let core = test_core();
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    // Two columns: red, then blue.
    let picture = [[255, 0, 0, 255], [0, 0, 255, 255]].concat().repeat(2);
    target
        .show_picture(core, ([2, 2], &picture))
        .expect("shown");
    let read = target.read(core).expect("read");
    let at = |x: u32, y: u32| {
        let i = ((y * SIZE[0] + x) * 4) as usize;
        read.pixels()[i..i + 4].to_vec()
    };
    assert_eq!(
        at(0, 0),
        [255, 0, 0, 255],
        "left: the picture's left column"
    );
    assert_eq!(at(SIZE[0] - 1, SIZE[1] - 1), [0, 0, 255, 255]);
    let middle = at(SIZE[0] / 2, SIZE[1] / 2);
    assert!(
        middle[0] > 60 && middle[2] > 60,
        "bilinear between: {middle:?}"
    );
    assert!(
        target.show_picture(core, ([3, 2], &picture)).is_err(),
        "pixels of another size"
    );
}
