//! The screen shows the canvas through the view: at one screen pixel per canvas pixel on whole
//! pixels it shows the evaluated bytes unchanged; zoomed and turned, each canvas point shows
//! where the view's map puts it; outside the canvas and outside the canvas area, the surround.

use openpaint_core::Color;
use openpaint_render::{
    BrushCursor, CanvasPlacement, CanvasTarget, Core, PixelRect, RegionEvaluator, RgbaImage,
    ScreenFrame, ScreenPainter, ScreenTarget, UiFrame,
};

use crate::fixture::{
    SIZE, blank, core, diagonal, pixel, plain, request, sample, wave, with_stroke,
};

const SCREEN: [u32; 2] = [400, 300];
const SURROUND: [u8; 4] = [40, 50, 60, 255];

fn evaluated(core: &Core, artwork: &openpaint_core::Artwork) -> CanvasTarget {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    target
}

fn shown(core: &Core, placement: CanvasPlacement<'_>) -> RgbaImage {
    let screen = ScreenTarget::new(core, SCREEN).expect("screen");
    let mut painter = ScreenPainter::offscreen(core);
    let frame = ScreenFrame {
        surround: SURROUND,
        canvas: Some(placement),
        ui: UiFrame {
            primitives: Vec::new(),
            textures: Default::default(),
            pixels_per_point: 1.0,
        },
    };
    core.render_screen(&mut painter, &screen, frame)
        .expect("screen");
    screen.read(core).expect("readback")
}

#[test]
fn at_one_pixel_per_pixel_the_screen_shows_the_canvas_bytes() {
    let core = core();
    let artwork = with_stroke(&with_stroke(&blank(), wave()), diagonal());
    let target = evaluated(&core, &artwork);
    let canvas = target.read(&core).expect("readback");
    let placement = CanvasPlacement {
        target: &target,
        origin: [30, 50],
        size: SIZE,
        to_canvas: [1.0, 0.0, -30.0, 0.0, 1.0, -50.0],
        cursor: None,
        pixels_per_point: 1.0,
    };
    let screen = shown(&core, placement);
    let same = (0..SIZE[1])
        .all(|y| (0..SIZE[0]).all(|x| pixel(&screen, [x + 30, y + 50]) == pixel(&canvas, [x, y])));
    assert!(same, "the screen differs from the canvas");
    assert_eq!(pixel(&screen, [5, 5]), SURROUND);
    assert_eq!(pixel(&screen, [380, 260]), SURROUND);
}

#[test]
fn a_zoomed_and_turned_view_shows_each_canvas_point_where_its_map_puts_it() {
    let core = core();
    let target = banded(&core);
    let screen = shown(&core, turned(&target, None));
    assert_eq!(
        pixel(&screen, [200, 150]),
        [0, 0, 0, 255],
        "the band's centre"
    );
    // Canvas (160, 60), above the band, shows 20 pixels right of the centre.
    assert_eq!(pixel(&screen, [220, 150]), [255; 4], "paper");
    // Canvas (100, 100) is in the band, 30 pixels above the centre on screen.
    assert_eq!(pixel(&screen, [200, 120]), [0, 0, 0, 255], "band");
    // Canvas (160, 190), below the band, shows at (155, 150).
    assert_eq!(pixel(&screen, [155, 150]), [255; 4], "paper");
    assert_eq!(pixel(&screen, [5, 5]), SURROUND, "off the canvas");
    // Canvas (160, 40) is paper, but past the area's edge only the surround shows.
    assert_eq!(
        pixel(&screen, [240, 150]),
        SURROUND,
        "outside the canvas area"
    );
}

/// A black band across the canvas: rows 70 to 130, columns 40 to 280.
fn banded(core: &Core) -> CanvasTarget {
    let band = (
        plain(60.0, 1.0, Color::BLACK),
        vec![sample(40.0, 100.0, 1.0, 0), sample(280.0, 100.0, 1.0, 8)],
    );
    evaluated(core, &with_stroke(&blank(), band))
}

/// Half size, turned a quarter clockwise, the canvas's centre at the screen's (200, 150); the
/// canvas area ends at x = 230.
fn turned(target: &CanvasTarget, cursor: Option<BrushCursor>) -> CanvasPlacement<'_> {
    CanvasPlacement {
        target,
        origin: [0, 0],
        size: [230, 300],
        to_canvas: [0.0, 2.0, -140.0, -2.0, 0.0, 500.0],
        cursor,
        pixels_per_point: 1.0,
    }
}

#[test]
fn the_hover_outline_is_white_over_dark_and_black_over_light_and_leaves_the_rest() {
    let core = core();
    let target = banded(&core);
    let ring = |centre, radius| {
        Some(BrushCursor {
            centre,
            radius,
            width: 2.0,
        })
    };
    let screen = shown(&core, turned(&target, ring([200.5, 150.5], 30.0)));
    // The ring's top lies in the band, its left on the paper; its middle is untouched.
    assert_eq!(pixel(&screen, [200, 120]), [255; 4], "over the band");
    assert_eq!(pixel(&screen, [170, 150]), [0, 0, 0, 255], "over the paper");
    assert_eq!(
        pixel(&screen, [200, 150]),
        [0, 0, 0, 255],
        "inside the ring: the band"
    );
    assert_eq!(
        pixel(&screen, [178, 150]),
        [255; 4],
        "inside the ring: the paper"
    );
    // Over the dark surround off the canvas, white.
    let off = shown(&core, turned(&target, ring([5.5, 25.5], 20.0)));
    assert_eq!(pixel(&off, [5, 5]), [255; 4], "over the surround");
    assert_eq!(
        pixel(&off, [5, 25]),
        SURROUND,
        "inside the ring off the canvas"
    );
}
