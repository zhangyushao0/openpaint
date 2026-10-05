//! The eraser takes paint away from its own layer (destination out) with any recipe; Alpha Lock
//! makes paint land only where the layer already has paint, keeping its alpha (source atop);
//! both equal cold evaluation when drawn incrementally; picking reads what the target shows.

use openpaint_core::{
    Artwork, ArtworkCommand, Color, LayerCommand, StrokeSample, StrokeStyle, StrokeTool,
};
use openpaint_render::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{
    SIZE, blank, cold, core, diagonal_with, pixel, plain, request, rich, sample, wave, with_stroke,
};

const WHITE: [u8; 4] = [255; 4];
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
/// On the red line, where the vertical stroke crosses it.
const CROSSING: [u32; 2] = [160, 100];
/// On the red line, away from the vertical stroke.
const ON_RED: [u32; 2] = [80, 100];
/// On the vertical stroke, away from the red line.
const OFF_RED: [u32; 2] = [160, 40];

/// The wave, the eraser through it with every recipe stage, and an alpha-locked blue stroke:
/// same on Vulkan and DX12.
const ERASE_GOLDEN: u64 = 0xf90a_9679_a5bf_8706;

fn line(style: StrokeStyle, from: [f32; 2], to: [f32; 2]) -> (StrokeStyle, Vec<StrokeSample>) {
    let samples = (0..=8u16)
        .map(|i| {
            let t = f32::from(i) / 8.0;
            let [x, y] = [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
            ];
            sample(x, y, 1.0, u32::from(i) * 8)
        })
        .collect();
    (style, samples)
}

fn red_line() -> (StrokeStyle, Vec<StrokeSample>) {
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    line(plain(24.0, 1.0, red), [40.0, 100.0], [280.0, 100.0])
}

/// A vertical stroke through the crossing, painting `color` or erasing at `opacity`.
fn vertical(tool: StrokeTool, opacity: f32) -> (StrokeStyle, Vec<StrokeSample>) {
    let blue = Color::rgb(0.0, 0.0, 1.0).expect("blue");
    let style = plain(24.0, opacity, blue).with_tool(tool);
    line(style, [160.0, 20.0], [160.0, 180.0])
}

fn command(artwork: &Artwork, command: LayerCommand) -> Artwork {
    artwork
        .apply(ArtworkCommand::Layer(command))
        .expect("layer command")
        .artwork
}

fn alpha_locked(artwork: &Artwork) -> Artwork {
    let layer = artwork.active();
    command(
        artwork,
        LayerCommand::SetAlphaLocked {
            layer,
            locked: true,
        },
    )
}

#[test]
fn the_eraser_takes_paint_away_by_its_opacity() {
    let core = core();
    let red = with_stroke(&blank(), red_line());
    let erased = with_stroke(&red, vertical(StrokeTool::Erase, 1.0));
    let image = cold(&core, &erased);
    assert_eq!(pixel(&image, CROSSING), WHITE, "erased to the paper");
    assert_eq!(pixel(&image, ON_RED), RED);
    assert_eq!(pixel(&image, OFF_RED), WHITE, "nothing to erase there");
    let half = with_stroke(&red, vertical(StrokeTool::Erase, 0.5));
    assert_eq!(pixel(&cold(&core, &half), CROSSING), [255, 128, 128, 255]);
}

#[test]
fn the_eraser_works_on_its_own_layer_only() {
    let core = core();
    let red = with_stroke(&blank(), red_line());
    let bottom = red.active();
    let blue = with_stroke(
        &command(&red, LayerCommand::Create),
        vertical(StrokeTool::Paint, 1.0),
    );
    let back = command(&blue, LayerCommand::Select(bottom));
    let erased = with_stroke(
        &back,
        line(
            plain(24.0, 1.0, Color::BLACK).with_tool(StrokeTool::Erase),
            [40.0, 100.0],
            [280.0, 100.0],
        ),
    );
    let image = cold(&core, &erased);
    assert_eq!(
        pixel(&image, CROSSING),
        BLUE,
        "the layer above keeps its paint"
    );
    assert_eq!(pixel(&image, ON_RED), WHITE);
}

#[test]
fn alpha_lock_paints_only_over_the_layers_paint() {
    let core = core();
    let locked = alpha_locked(&with_stroke(&blank(), red_line()));
    let painted = with_stroke(&locked, vertical(StrokeTool::Paint, 1.0));
    let image = cold(&core, &painted);
    assert_eq!(pixel(&image, CROSSING), BLUE);
    assert_eq!(pixel(&image, ON_RED), RED);
    assert_eq!(
        pixel(&image, OFF_RED),
        WHITE,
        "no paint where the layer had none"
    );
    // On the antialiased edge of the line's round end the blue replaces the red and keeps its
    // alpha: as much paper shows through as before.
    let edge = [31, 91];
    let blue = Color::rgb(0.0, 0.0, 1.0).expect("blue");
    let over_end = line(plain(24.0, 1.0, blue), [32.0, 70.0], [32.0, 130.0]);
    let before = pixel(&cold(&core, &locked), edge);
    let after = pixel(&cold(&core, &with_stroke(&locked, over_end)), edge);
    assert!(
        before[1] > 0 && before[1] < 255,
        "a partly covered pixel: {before:?}"
    );
    assert_eq!(after, [before[1], before[1], 255, 255], "{before:?}");
    // Turning the lock off later leaves the stroke drawn under it as it was.
    let layer = painted.active();
    let unlocked = command(
        &painted,
        LayerCommand::SetAlphaLocked {
            layer,
            locked: false,
        },
    );
    assert!(cold(&core, &unlocked) == image);
}

/// Draws `stroke` on `artwork` chunk by chunk as an in-progress stroke, then commits it, and
/// checks the target equals the finished artwork evaluated cold; then undoes it through
/// `changed_regions` and checks again.
fn incremental_equals_cold(artwork: &Artwork, (style, samples): (StrokeStyle, Vec<StrokeSample>)) {
    let core = core();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut session = artwork.begin_stroke(style, samples[0]).expect("begin");
    for chunk in samples[1..].chunks(3) {
        session.extend(chunk).expect("extend");
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(artwork)
        };
        evaluator.evaluate(&frame, &[]).expect("evaluate");
    }
    let finished = session.finish().artwork;
    evaluator
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    assert!(target.read(&core).expect("readback") == cold(&core, &finished));
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(artwork), &changed_regions(&finished, artwork))
        .expect("evaluate");
    assert!(target.read(&core).expect("readback") == cold(&core, artwork));
}

#[test]
fn erasing_and_alpha_locked_painting_drawn_in_chunks_equal_cold() {
    let waved = with_stroke(&blank(), wave());
    let (style, samples) = diagonal_with(rich());
    let eraser = (style.clone().with_tool(StrokeTool::Erase), samples.clone());
    incremental_equals_cold(&waved, eraser);
    incremental_equals_cold(&alpha_locked(&waved), (style, samples));
}

#[test]
fn erasing_and_alpha_lock_match_their_golden_hash() {
    let core = core();
    let waved = with_stroke(&blank(), wave());
    let (style, samples) = diagonal_with(rich());
    let erased = with_stroke(&waved, (style.with_tool(StrokeTool::Erase), samples));
    let painted = with_stroke(&alpha_locked(&erased), vertical(StrokeTool::Paint, 0.8));
    let image = cold(&core, &painted);
    assert_eq!(
        image.content_hash(),
        ERASE_GOLDEN,
        "hash {:016x} on {}",
        image.content_hash(),
        core.adapter()
    );
}

#[test]
fn a_picked_colour_is_the_pixel_the_readback_gives() {
    let core = core();
    let artwork = with_stroke(&with_stroke(&blank(), wave()), red_line());
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let image = target.read(&core).expect("readback");
    for at in [[0, 0], CROSSING, ON_RED, [160, 117], [33, 101], [319, 199]] {
        assert_eq!(
            target.pick(&core, at),
            Ok(Some(pixel(&image, at))),
            "at {at:?}"
        );
    }
    assert_eq!(target.pick(&core, [320, 0]), Ok(None));
    assert_eq!(target.pick(&core, [0, 200]), Ok(None));
}
