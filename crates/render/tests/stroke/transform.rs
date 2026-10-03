//! Transforms on the GPU: a layer or a selection lifted and placed again. In place nothing
//! changes; a move shows the same pixels elsewhere; outside a selection nothing changes; and
//! the done criterion of S20: every preview frame evaluated incrementally on one target, and
//! the last one equals the committed artwork, reloaded and evaluated cold, bit for bit.

use openpaint_core::{Artwork, CanvasSize, Color, Patch, Placement, RegionMask};
use openpaint_render::{CanvasTarget, Core, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{
    SIZE, blank, cold, core, diagonal, pixel, plain, request, sample, wave, with_stroke,
};

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

/// The wave and a solid black bar on one layer.
fn drawn() -> Artwork {
    let bar = (
        plain(12.0, 1.0, Color::BLACK),
        vec![sample(40.0, 40.0, 1.0, 0), sample(140.0, 40.0, 1.0, 8)],
    );
    with_stroke(&with_stroke(&blank(), wave()), bar)
}

/// The active layer's pixels (inside `clip`, if given) as a patch the size of the canvas, as
/// `openpaint_raster::lift` takes them but not trimmed to their paint.
fn lift(core: &Core, artwork: &Artwork, clip: Option<&RegionMask>) -> Patch {
    let pixels = core
        .layer_pixels(artwork, artwork.active())
        .expect("sample")
        .into_pixels();
    let mut taken = pixels.clone();
    for (index, pixel) in taken.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let at = [index as u32 % SIZE[0], index as u32 / SIZE[0]];
        if clip.is_some_and(|clip| !clip.contains(at)) {
            pixel.fill(0);
        }
    }
    Patch::new(canvas(), [0, 0], SIZE, taken).expect("patch")
}

/// `artwork` with its active layer (inside `clip`, if given) lifted and placed by `placement`.
fn transformed(
    core: &Core,
    artwork: &Artwork,
    clip: Option<&RegionMask>,
    placement: Placement,
) -> Artwork {
    let patch = lift(core, artwork, clip);
    artwork
        .transform_region(clip.cloned(), patch, placement)
        .expect("transform")
        .artwork
}

/// At most one step apart in every channel.
fn near(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(&b).all(|(x, y)| x.abs_diff(*y) <= 1)
}

#[test]
fn a_layer_transformed_in_place_looks_the_same() {
    let core = core();
    let base = drawn();
    let still = transformed(&core, &base, None, Placement::IDENTITY);
    let (before, after) = (cold(&core, &base), cold(&core, &still));
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let (a, b) = (pixel(&before, [x, y]), pixel(&after, [x, y]));
            assert!(near(a, b), "at {x}, {y}: {a:?} against {b:?}");
        }
    }
}

#[test]
fn a_moved_layer_shows_its_pixels_at_the_new_place() {
    let core = core();
    let base = drawn();
    let moved = transformed(&core, &base, None, Placement::moved([30.0, -20.0]).unwrap());
    let (before, after) = (cold(&core, &base), cold(&core, &moved));
    for y in 25..SIZE[1] {
        for x in 0..SIZE[0] - 30 {
            let (a, b) = (pixel(&before, [x, y]), pixel(&after, [x + 30, y - 20]));
            assert!(near(a, b), "{x}, {y}");
        }
    }
    assert_eq!(
        pixel(&after, [60, 40]),
        [255; 4],
        "the bar's old place is clear"
    );
    assert_eq!(
        pixel(&after, [100, 20])[..3],
        [0, 0, 0],
        "the bar's new place"
    );
}

#[test]
fn outside_a_selection_nothing_changes() {
    let core = core();
    let base = drawn();
    let clip = RegionMask::polygon(
        canvas(),
        &[[20.0, 20.0], [160.0, 20.0], [160.0, 60.0], [20.0, 60.0]],
    )
    .expect("a rectangle");
    let turned = Placement::around([90.0, 40.0], 0.5, 0.7, [false; 2]).unwrap();
    let moved = transformed(&core, &base, Some(&clip), turned);
    let (before, after) = (cold(&core, &base), cold(&core, &moved));
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let lifted = (19..=161).contains(&x) && (19..=61).contains(&y);
            if !lifted {
                assert_eq!(pixel(&before, [x, y]), pixel(&after, [x, y]), "{x}, {y}");
            }
        }
    }
    assert_eq!(
        pixel(&after, [30, 40]),
        [255; 4],
        "the bar's end is lifted away"
    );
}

/// The placements a drag passes through: still, moved, scaled about a corner, turned, flipped.
fn drag() -> Vec<Placement> {
    let moved = Placement::moved([12.5, 7.25]).unwrap();
    let scaled = moved
        .then(Placement::around([40.0, 30.0], 1.37, 0.0, [false; 2]).unwrap())
        .unwrap();
    let turned = scaled
        .then(Placement::around([160.0, 100.0], 1.0, -0.61, [false; 2]).unwrap())
        .unwrap();
    let flipped = turned
        .then(Placement::around([150.0, 90.0], 1.0, 0.0, [true, false]).unwrap())
        .unwrap();
    vec![Placement::IDENTITY, moved, scaled, turned, flipped]
}

fn preview_frames_equal_the_committed_artwork(clip: Option<&RegionMask>) {
    let core = core();
    let base = with_stroke(&drawn(), diagonal());
    let patch = lift(&core, &base, clip);
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    let mut shown = base.clone();
    for placement in drag() {
        let preview = base
            .transform_region(clip.cloned(), patch.clone(), placement)
            .expect("preview")
            .artwork;
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

#[test]
fn the_last_preview_frame_equals_the_committed_layer_transform() {
    preview_frames_equal_the_committed_artwork(None);
}

#[test]
fn the_last_preview_frame_equals_the_committed_selection_transform() {
    let clip = RegionMask::polygon(
        canvas(),
        &[[50.0, 20.0], [250.0, 60.0], [200.0, 180.0], [70.0, 150.0]],
    );
    preview_frames_equal_the_committed_artwork(clip.as_ref());
}

#[test]
fn strokes_after_a_transform_and_its_undo_evaluate_incrementally_as_cold() {
    let core = core();
    let base = drawn();
    let moved = transformed(&core, &base, None, Placement::moved([-15.0, 9.0]).unwrap());
    let later = with_stroke(&moved, diagonal());
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&base), &[PixelRect::full(SIZE)])
        .expect("base");
    for (from, to) in [(&base, &moved), (&moved, &later), (&later, &base)] {
        evaluator
            .evaluate(&request(to), &changed_regions(from, to))
            .expect("step");
    }
    assert_eq!(target.read(&core).expect("read"), cold(&core, &base));
}
