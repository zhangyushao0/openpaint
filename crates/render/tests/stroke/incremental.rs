//! Incremental evaluation equals cold evaluation bit for bit.

use openpaint_core::{Artwork, StrokeSample, StrokeSession, StrokeStyle};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage, stroke_region,
};

use crate::fixture::{SIZE, blank, cold, core, diagonal, request, wave, with_layer, with_stroke};

/// Chunk sizes cycle through these, so chunk boundaries fall at varied places.
const CHUNKS: [usize; 5] = [1, 2, 3, 5, 8];

/// Wave on the bottom layer, then a 70 % layer for the diagonal.
fn under() -> Artwork {
    with_layer(&with_stroke(&blank(), wave()), 0.7)
}

/// How each frame of a growing stroke names what to evaluate.
#[derive(Clone, Copy)]
enum Dirty {
    /// The caller passes the region of the new samples.
    Caller,
    /// The caller passes nothing; the evaluator finds the in-progress stroke's changes itself.
    Evaluator,
}

/// Draws `stroke` on `artwork` as an in-progress stroke growing chunk by chunk, evaluating
/// after every chunk. Returns the session holding the whole stroke, not yet finished.
fn draw_incrementally(
    core: &Core,
    target: &mut CanvasTarget,
    artwork: &Artwork,
    (style, samples): &(StrokeStyle, Vec<StrokeSample>),
    dirty: Dirty,
) -> StrokeSession {
    let mut evaluator = RegionEvaluator::new(core, target);
    let mut session = artwork.begin_stroke(*style, samples[0]).expect("begin");
    let mut drawn = 1;
    let mut region = stroke_region(style, &samples[..1]);
    for size in CHUNKS.iter().cycle() {
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(artwork)
        };
        let regions: Vec<PixelRect> = match dirty {
            Dirty::Caller => region.into_iter().collect(),
            Dirty::Evaluator => Vec::new(),
        };
        evaluator.evaluate(&frame, &regions).expect("evaluate");
        if drawn == samples.len() {
            break;
        }
        let end = (drawn + size).min(samples.len());
        session.extend(&samples[drawn..end]).expect("extend");
        // The new segments start at the last sample drawn before them.
        region = stroke_region(style, &samples[drawn - 1..end]);
        drawn = end;
    }
    session
}

fn read(core: &Core, target: &CanvasTarget) -> RgbaImage {
    target.read(core).expect("readback")
}

fn evaluated(core: &Core, artwork: &Artwork) -> CanvasTarget {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    target
}

#[test]
fn a_stroke_drawn_in_chunks_equals_the_finished_artwork_evaluated_cold() {
    let core = core();
    let under = under();
    for dirty in [Dirty::Caller, Dirty::Evaluator] {
        let mut target = evaluated(&core, &under);
        let before = read(&core, &target);
        let session = draw_incrementally(&core, &mut target, &under, &diagonal(), dirty);
        let finished = session.finish().artwork;
        assert_eq!(finished, with_stroke(&under, diagonal()));
        // The committed stroke already shows; evaluating the new version changes nothing.
        let incremental = read(&core, &target);
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&finished), &[])
            .expect("evaluate");
        let cold = cold(&core, &finished);
        assert_ne!(before, cold, "the diagonal changes pixels");
        assert!(incremental == cold, "incremental differs from cold");
        assert!(read(&core, &target) == cold, "committing changed pixels");
    }
}

#[test]
fn samples_that_arrive_with_the_lift_are_drawn_when_the_stroke_is_committed() {
    let core = core();
    let under = under();
    let (style, samples) = diagonal();
    let mut target = evaluated(&core, &under);
    let head = (style, samples[..25].to_vec());
    let mut session = draw_incrementally(&core, &mut target, &under, &head, Dirty::Evaluator);
    session.extend(&samples[25..]).expect("extend");
    let finished = session.finish().artwork;
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    assert!(
        read(&core, &target) == cold(&core, &finished),
        "the tail is missing"
    );
}

#[test]
fn a_cancelled_stroke_is_erased() {
    let core = core();
    let under = under();
    let mut target = evaluated(&core, &under);
    let session = draw_incrementally(&core, &mut target, &under, &diagonal(), Dirty::Evaluator);
    let base = session.cancel();
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&base), &[])
        .expect("evaluate");
    assert!(
        read(&core, &target) == cold(&core, &under),
        "the stroke stayed"
    );
}

#[test]
fn evaluating_in_scattered_regions_equals_one_full_evaluation() {
    let core = core();
    let artwork = with_stroke(&under(), diagonal());
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    // Overlapping, uneven strips that run off the target, then the rest in single calls.
    let strips = [(-40, 37), (30, 101), (101, 102), (150, 233), (200, 500)];
    let regions: Vec<_> = strips
        .iter()
        .map(|&(x0, x1)| PixelRect {
            min: [x0, -5],
            max: [x1, 400],
        })
        .collect();
    evaluator
        .evaluate(&request(&artwork), &regions)
        .expect("evaluate");
    for x in (102..150).step_by(13) {
        let column = PixelRect {
            min: [x, 0],
            max: [x + 13, SIZE[1] as i32],
        };
        evaluator
            .evaluate(&request(&artwork), &[column])
            .expect("evaluate");
    }
    assert!(
        read(&core, &target) == cold(&core, &artwork),
        "regions differ"
    );
}

#[test]
fn a_target_of_another_size_is_rejected() {
    let core = core();
    let mut target = CanvasTarget::new(&core, [SIZE[0], SIZE[1] + 1]).expect("target");
    let artwork = blank();
    let result = RegionEvaluator::new(&core, &mut target).evaluate(&request(&artwork), &[]);
    assert!(result.is_err(), "{result:?}");
}
