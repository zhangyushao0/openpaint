//! The screen shows the canvas through the view: at one screen pixel per canvas pixel on whole
//! pixels it shows the evaluated bytes unchanged; zoomed and turned, each canvas point shows
//! where the view's map puts it; outside the canvas and outside the canvas area, the surround.

use openpaint_core::Color;
use openpaint_render::{
    BrushCursor, CanvasPlacement, CanvasTarget, Core, PixelRect, RgbaImage, ScreenFrame,
    ScreenPainter, ScreenTarget, UiFrame,
};

use crate::fixture::{
    SIZE, blank, core, diagonal, pixel, plain, request, sample, wave, with_stroke,
};

const SCREEN: [u32; 2] = [400, 300];
const SURROUND: [u8; 4] = [40, 50, 60, 255];

fn evaluated(core: &Core, artwork: &openpaint_core::Artwork) -> CanvasTarget {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    target
        .draw(core, &request(artwork), &[PixelRect::full(SIZE)])
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
        view: None,
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
        view: None,
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

/// Zoomed past the canvas's resolution the screen shows the view tier: each screen pixel is the
/// tier's export read bilinearly where the view's map puts it, inside a tile (one index load and
/// the sampler's filter, whose weights the GPU holds in a few bits) and across its edges (each
/// texel found apart) alike, within a level and a half.
#[test]
fn through_a_tier_each_pixel_is_its_export_read_bilinearly_across_tile_edges_too() {
    let core = core();
    let artwork = crate::scale::every_source();
    let mut tiers = crate::tiers::Screen::new(&core, &artwork);
    // At 2.2 the 2x tier shows; canvas x 59 to 241 spans its tiles' edge at x 128.
    let view = crate::tiers::frame(2.2, [150.0, 110.0]);
    tiers.settle(&core, &artwork, &view);
    let export = crate::scale::whole(&core, &artwork, 2);
    let placement = CanvasPlacement {
        target: &tiers.target,
        origin: view.origin,
        size: view.size,
        to_canvas: view.to_canvas,
        cursor: None,
        pixels_per_point: 1.0,
        view: Some(&tiers.view),
    };
    let screen = shown(&core, placement);
    let [a, b, c, d, e, f] = view.to_canvas;
    let [width, height] = export.size().map(|side| side as i32);
    let texel = |x: i32, y: i32| {
        pixel(
            &export,
            [x.clamp(0, width - 1), y.clamp(0, height - 1)].map(|v| v as u32),
        )
    };
    let mut off = Vec::new();
    for y in 0..SCREEN[1] {
        for x in 0..SCREEN[0] {
            let [sx, sy] = [x as f32 + 0.5, y as f32 + 0.5];
            let at = [a * sx + b * sy + c, d * sx + e * sy + f].map(|v| v * 2.0 - 0.5);
            let base = at.map(f32::floor);
            let [tx, ty] = [at[0] - base[0], at[1] - base[1]];
            let [bx, by] = base.map(|v| v as i32);
            let corners = [
                texel(bx, by),
                texel(bx + 1, by),
                texel(bx, by + 1),
                texel(bx + 1, by + 1),
            ];
            let want: Vec<f32> = (0..4)
                .map(|k| {
                    let [p, q, r, s] = corners.map(|t| f32::from(t[k]));
                    (p + (q - p) * tx) * (1.0 - ty) + (r + (s - r) * tx) * ty
                })
                .collect();
            let got = pixel(&screen, [x, y]);
            if (0..4).any(|k| (f32::from(got[k]) - want[k]).abs() > 1.5) {
                off.push(([x, y], got, want));
            }
        }
    }
    assert!(
        off.is_empty(),
        "{} pixels off, the first {:?}",
        off.len(),
        off.first()
    );
}

#[test]
fn the_precision_check_covers_the_screen_with_a_ramp_from_100_to_120() {
    let core = core();
    let target = banded(&core);
    let screen = ScreenTarget::new(&core, SCREEN).expect("screen");
    let mut painter = ScreenPainter::offscreen(&core);
    painter.show_gradient(&core, true);
    let frame = ScreenFrame {
        surround: SURROUND,
        canvas: Some(turned(&target, None)),
        ui: UiFrame {
            primitives: Vec::new(),
            textures: Default::default(),
            pixels_per_point: 1.0,
        },
    };
    core.render_screen(&mut painter, &screen, frame)
        .expect("screen");
    let image = screen.read(&core).expect("readback");
    for y in [0, 100, 200, 299] {
        assert_eq!(pixel(&image, [0, y]), [100, 100, 100, 255], "left, row {y}");
        assert_eq!(
            pixel(&image, [399, y]),
            [120, 120, 120, 255],
            "right, row {y}"
        );
        assert_eq!(
            pixel(&image, [200, y]),
            [110, 110, 110, 255],
            "middle, row {y}"
        );
    }
}
