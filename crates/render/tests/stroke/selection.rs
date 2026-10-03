//! Painting inside a selection (S18): a stroke drawn inside a selection changes no pixel outside
//! it, whatever its brush or tool, drawn incrementally or committed and evaluated cold; and the
//! screen marks the selection with an outline and stripes outside it, leaving the canvas inside
//! as it is.

use openpaint_core::{Artwork, RegionMask, StrokeSample, StrokeStyle, StrokeTool};
use openpaint_render::{
    CanvasPlacement, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage,
    ScreenFrame, ScreenPainter, ScreenTarget, UiFrame,
};

use crate::fixture::{
    SIZE, blank, cold, core, diagonal, diagonal_with, graphite, pixel, request, wave, with_stroke,
};

/// A lasso through the middle of the canvas: a slanted hexagon.
fn lasso(artwork: &Artwork) -> RegionMask {
    let points = [
        [90.0, 40.5],
        [200.0, 30.0],
        [250.0, 100.0],
        [210.0, 170.25],
        [100.0, 160.0],
        [70.0, 95.0],
    ];
    RegionMask::polygon(artwork.canvas(), &points).expect("a lasso")
}

/// `stroke` drawn inside `clip` on `artwork`, evaluated after every few samples into `target`;
/// returns the committed artwork.
fn draw_clipped(
    core: &Core,
    target: &mut CanvasTarget,
    artwork: &Artwork,
    (style, samples): &(StrokeStyle, Vec<StrokeSample>),
    clip: &RegionMask,
) -> Artwork {
    let mut evaluator = RegionEvaluator::new(core, target);
    let session = artwork
        .begin_stroke(style.clone(), samples[0])
        .expect("begin");
    let mut session = session.clipped(clip.clone()).expect("clipped");
    for chunk in samples[1..].chunks(3) {
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(artwork)
        };
        evaluator.evaluate(&frame, &[]).expect("evaluate");
        session.extend(chunk).expect("extend");
    }
    let frame = FrameRequest {
        active: Some(session.active()),
        ..request(artwork)
    };
    evaluator.evaluate(&frame, &[]).expect("evaluate");
    let committed = session.finish().artwork;
    evaluator
        .evaluate(&request(&committed), &[])
        .expect("commit");
    committed
}

/// How many pixels differ inside and outside `clip` between two canvases.
fn differences(before: &RgbaImage, after: &RgbaImage, clip: &RegionMask) -> (usize, usize) {
    let (mut inside, mut outside) = (0, 0);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            if pixel(before, [x, y]) != pixel(after, [x, y]) {
                if clip.contains([x, y]) {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
        }
    }
    (inside, outside)
}

#[test]
fn pixels_outside_the_selection_stay_unchanged() {
    let core = core();
    let base = with_stroke(&blank(), wave());
    let clip = lasso(&base);
    let before = cold(&core, &base);
    let (style, samples) = diagonal();
    // The eraser runs along the wave, past the selection at both ends.
    let eraser = (style.clone().with_tool(StrokeTool::Erase), wave().1);
    let pencil = diagonal_with(graphite());
    // Paint, a building-up pencil and the eraser, each crossing the selection's edge.
    for stroke in [(style, samples), pencil, eraser] {
        let mut target = CanvasTarget::new(&core, SIZE).expect("target");
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&base), &[PixelRect::full(SIZE)])
            .expect("evaluate");
        let drawn = draw_clipped(&core, &mut target, &base, &stroke, &clip);
        let incremental = target.read(&core).expect("readback");
        let (inside, outside) = differences(&before, &incremental, &clip);
        assert_eq!(outside, 0, "nothing changes outside the selection");
        assert!(inside > 500, "the stroke shows inside ({inside} pixels)");
        assert!(
            incremental == cold(&core, &drawn),
            "drawn incrementally equals the committed stroke evaluated cold"
        );
        let unclipped = with_stroke(&base, stroke);
        let (_, beyond) = differences(&before, &cold(&core, &unclipped), &clip);
        assert!(beyond > 0, "without the selection it would reach outside");
    }
}

fn shown(core: &Core, target: &CanvasTarget) -> RgbaImage {
    let screen = ScreenTarget::new(core, SIZE).expect("screen");
    let mut painter = ScreenPainter::offscreen(core);
    let placement = CanvasPlacement {
        target,
        origin: [0, 0],
        size: SIZE,
        to_canvas: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        cursor: None,
        pixels_per_point: 2.0,
    };
    let frame = ScreenFrame {
        surround: [40, 50, 60, 255],
        canvas: Some(placement),
        ui: UiFrame {
            primitives: Vec::new(),
            textures: Default::default(),
            pixels_per_point: 2.0,
        },
    };
    core.render_screen(&mut painter, &screen, frame)
        .expect("screen");
    screen.read(core).expect("readback")
}

#[test]
fn the_screen_outlines_the_selection_and_stripes_outside_it_only() {
    let core = core();
    let artwork = blank();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let canvas = target.read(&core).expect("readback");
    let square = [[100.0, 50.0], [220.0, 50.0], [220.0, 150.0], [100.0, 150.0]];
    let clip = RegionMask::polygon(artwork.canvas(), &square).expect("a square");
    target.show_selection(&core, Some(&clip));
    let marked = shown(&core, &target);
    let inside = (50..150).all(|y| (100..220).all(|x| pixel(&marked, [x, y]) == [255; 4]));
    assert!(inside, "the canvas inside shows as it is");
    // Paper is light: the outline, 1.5 points (3 pixels) outside the edge, is black.
    assert_eq!(
        pixel(&marked, [98, 100]),
        [0, 0, 0, 255],
        "left of the edge"
    );
    assert_eq!(pixel(&marked, [160, 151]), [0, 0, 0, 255], "under the edge");
    assert_ne!(
        pixel(&marked, [160, 154]),
        [0, 0, 0, 255],
        "past the outline"
    );
    let outside: Vec<[u8; 4]> = (0..40).map(|x| pixel(&marked, [x, 10])).collect();
    assert!(outside.iter().all(|p| p[0] < 255), "outside is dimmed");
    assert!(
        outside.iter().any(|p| p[0] < 200) && outside.iter().any(|p| p[0] > 230),
        "with darker stripes"
    );
    target.show_selection(&core, None);
    assert!(
        shown(&core, &target) == canvas,
        "deselected, the screen is the canvas"
    );
}
