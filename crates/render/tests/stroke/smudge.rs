//! Smudge on a real GPU (D46): colour dragged only where the stroke reaches; the in-progress
//! residual, grown a few samples per frame, equal bit for bit to a cold evaluation of the same
//! samples, finished, reloaded from its parts, undone and redone; Alpha Lock keeping alpha; a
//! selection limiting what changes; a smudge over an earlier smudge.

use openpaint_core::{
    ActiveStroke, Artwork, ArtworkCommand, Color, LayerCommand, RegionMask, StrokeSample,
    StrokeStyle, StrokeTool,
};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage, changed_regions,
    stroke_region,
};

use crate::fixture::{SIZE, airbrush, blank, cold, core, plain, request, sample, with_stroke};

/// A red band above a blue one, sharp edges to drag across.
fn bands() -> Artwork {
    let band = |y: f32, color: Color| {
        let samples =
            (0..2u16).map(|i| sample(20.0 + 280.0 * f32::from(i), y, 1.0, u32::from(i) * 8));
        (plain(60.0, 1.0, color), samples.collect::<Vec<_>>())
    };
    let red = Color::rgb(0.9, 0.1, 0.05).expect("red");
    let blue = Color::rgb(0.05, 0.2, 0.85).expect("blue");
    with_stroke(&with_stroke(&blank(), band(60.0, red)), band(130.0, blue))
}

fn smudge_style() -> StrokeStyle {
    let color = Color::BLACK;
    StrokeStyle::new(airbrush(), 36.0, 0.85, color)
        .expect("style")
        .with_tool(StrokeTool::Smudge)
}

/// From the red band down through the blue one and out onto the paper, pressure rising.
fn path() -> Vec<StrokeSample> {
    (0..48u16)
        .map(|i| {
            let t = f32::from(i) / 47.0;
            let x = 70.0 + 180.0 * t + 12.0 * (t * 9.0).sin();
            sample(x, 40.0 + 140.0 * t, 0.4 + 0.6 * t, u32::from(i) * 8)
        })
        .collect()
}

fn smudged(artwork: &Artwork) -> Artwork {
    with_stroke(artwork, (smudge_style(), path()))
}

fn frame<'a>(artwork: &'a Artwork, active: &'a ActiveStroke) -> FrameRequest<'a> {
    FrameRequest {
        active: Some(active),
        ..request(artwork)
    }
}

fn evaluate(core: &Core, target: &mut CanvasTarget, request: &FrameRequest<'_>, all: bool) {
    let regions = if all {
        vec![PixelRect::full(SIZE)]
    } else {
        Vec::new()
    };
    RegionEvaluator::new(core, target)
        .evaluate(request, &regions)
        .expect("evaluate");
}

fn read(core: &Core, target: &CanvasTarget) -> RgbaImage {
    target.read(core).expect("readback")
}

/// The RGBA texels of two readbacks side by side.
fn texels<'a>(
    a: &'a RgbaImage,
    b: &'a RgbaImage,
) -> impl Iterator<Item = (&'a [u8; 4], &'a [u8; 4])> {
    a.pixels()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.pixels().as_chunks::<4>().0)
}

/// Pixels that differ between two readbacks, and whether all of them lie in `inside`.
fn changed(a: &RgbaImage, b: &RgbaImage, inside: impl Fn(u32, u32) -> bool) -> (usize, bool) {
    let mut count = 0;
    let mut within = true;
    for (index, (x, y)) in texels(a, b).enumerate() {
        if x != y {
            count += 1;
            let at = index as u32;
            within &= inside(at % SIZE[0], at / SIZE[0]);
        }
    }
    (count, within)
}

#[test]
fn a_smudge_drawn_frame_by_frame_equals_its_cold_evaluation_finished_reloaded_and_redone() {
    let core = core();
    let base = bands();
    let samples = path();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    evaluate(&core, &mut target, &request(&base), true);
    let untouched = read(&core, &target);
    let mut session = base
        .begin_stroke(smudge_style(), samples[0])
        .expect("begin");
    let mut midway = None;
    let mut drawn = 1;
    for size in [1, 2, 3, 5, 8].iter().cycle() {
        evaluate(&core, &mut target, &frame(&base, session.active()), false);
        if drawn >= samples.len() / 2 && midway.is_none() {
            midway = Some((session.active().clone(), read(&core, &target)));
        }
        if drawn == samples.len() {
            break;
        }
        let end = (drawn + size).min(samples.len());
        session.extend(&samples[drawn..end]).expect("extend");
        drawn = end;
    }
    let (active, pixels) = midway.expect("midway");
    let mut fresh = CanvasTarget::new(&core, SIZE).expect("target");
    evaluate(&core, &mut fresh, &frame(&base, &active), true);
    assert_eq!(
        read(&core, &fresh),
        pixels,
        "in progress = its cold evaluation"
    );
    let finished = session.finish().artwork;
    evaluate(&core, &mut target, &request(&finished), false);
    let incremental = read(&core, &target);
    assert_eq!(incremental, cold(&core, &finished), "committed = cold");
    let reloaded = Artwork::from_storage(finished.to_parts()).expect("reload");
    assert_eq!(cold(&core, &reloaded), incremental, "reopened = drawn");
    let reach = stroke_region(&smudge_style(), &samples, 0).expect("reach");
    let inside = |x: u32, y: u32| {
        let (x, y) = (x as i32, y as i32);
        x >= reach.min[0] && y >= reach.min[1] && x < reach.max[0] && y < reach.max[1]
    };
    let (count, within) = changed(&untouched, &incremental, inside);
    assert!(within, "nothing changes outside the smudge's reach");
    assert!(count > 1000, "the smudge carries colour: {count} pixels");
    let undo = changed_regions(&finished, &base);
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&base), &undo)
        .expect("undo");
    assert_eq!(read(&core, &target), untouched, "undo restores every pixel");
    let redo = changed_regions(&base, &finished);
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&finished), &redo)
        .expect("redo");
    assert_eq!(
        read(&core, &target),
        incremental,
        "redo smudges again exactly"
    );
}

#[test]
fn a_smudge_over_a_smudge_reads_it_and_stays_equal_to_cold() {
    let core = core();
    let first = smudged(&bands());
    let across: Vec<StrokeSample> = (0..30u16)
        .map(|i| sample(40.0 + 8.0 * f32::from(i), 100.0, 0.9, u32::from(i) * 8))
        .collect();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    evaluate(&core, &mut target, &request(&first), true);
    let mut session = first
        .begin_stroke(smudge_style(), across[0])
        .expect("begin");
    for chunk in across[1..].chunks(3) {
        session.extend(chunk).expect("extend");
        evaluate(&core, &mut target, &frame(&first, session.active()), false);
    }
    let second = session.finish().artwork;
    evaluate(&core, &mut target, &request(&second), false);
    assert_eq!(read(&core, &target), cold(&core, &second));
    assert_ne!(cold(&core, &second), cold(&core, &first));
    let alone = with_stroke(&bands(), (smudge_style(), across));
    assert_ne!(
        cold(&core, &second),
        cold(&core, &alone),
        "it carries the first's colour"
    );
}

#[test]
fn alpha_lock_keeps_every_alpha_and_still_moves_colour() {
    let core = core();
    let base = bands();
    let layer = base.active();
    let lock = LayerCommand::SetAlphaLocked {
        layer,
        locked: true,
    };
    let locked = base
        .apply(ArtworkCommand::Layer(lock))
        .expect("lock")
        .artwork;
    let before = core.layer_pixels(&locked, layer).expect("layer");
    let after = core.layer_pixels(&smudged(&locked), layer).expect("layer");
    let mut recoloured = 0;
    for (old, new) in texels(&before, &after) {
        assert_eq!(old[3], new[3], "alpha stays");
        recoloured += usize::from(old[..3] != new[..3]);
    }
    assert!(recoloured > 200, "colour moves: {recoloured}");
    let free = core.layer_pixels(&smudged(&base), layer).expect("layer");
    let alpha_moved = texels(&before, &free);
    assert!(
        alpha_moved.filter(|(a, b)| a[3] != b[3]).count() > 500,
        "unlocked, alpha moves"
    );
}

#[test]
fn a_selection_limits_what_the_smudge_changes_not_what_it_reads() {
    let core = core();
    let base = bands();
    let canvas = base.canvas();
    let lower = RegionMask::polygon(
        canvas,
        &[[0.0, 100.0], [320.0, 100.0], [320.0, 200.0], [0.0, 200.0]],
    )
    .expect("lower half");
    let samples = path();
    let mut session = base
        .begin_stroke(smudge_style(), samples[0])
        .expect("begin");
    session = session.clipped(lower).expect("clip");
    session.extend(&samples[1..]).expect("extend");
    let clipped = session.finish().artwork;
    let (count, within) = changed(&cold(&core, &base), &cold(&core, &clipped), |_, y| y >= 100);
    assert!(within, "outside the selection nothing changes");
    assert!(count > 500, "inside it the smudge works: {count}");
    // Inside, it is the same smudge as without the selection: it read the red above it too.
    let (free, inside) = (cold(&core, &smudged(&base)), cold(&core, &clipped));
    let (outside, within) = changed(&free, &inside, |_, y| y < 100);
    assert!(
        within && outside > 0,
        "only above the selection do they differ"
    );
}
