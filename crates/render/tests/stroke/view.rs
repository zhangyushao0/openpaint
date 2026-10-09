//! Sharp zoom (S32): a view tier's tiles equal the banded export at the tier's scale for their
//! region, bit for bit; after a stroke, a fill, a transform record and undo exactly the tiles
//! the change reaches are evaluated again and every tile still equals the export (incremental
//! = cold per tile); a stroke drawn while zoomed shows at the tier as it will be committed (its
//! end taper from the lift on, S44); the canvas target stays the cold canvas.

use std::collections::BTreeSet;

use openpaint_core::{ActiveStroke, Artwork, Color, Placement, RegionMask, StrokeStyle};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage, ViewCache, ViewFrame,
    ViewWork, changed_regions,
};

use crate::fixture::{SIZE, cold, core, plain, sample, with_stroke};
use crate::scale::{every_source, whole};

/// The screen the tests show the canvas on, in pixels.
const SCREEN: [u32; 2] = [400, 300];
const TILE: u32 = 256;

/// `zoom` screen pixels per canvas pixel, canvas point `centre` in the screen's middle.
fn frame(zoom: f32, [cx, cy]: [f32; 2]) -> ViewFrame {
    let s = 1.0 / zoom;
    let [w, h] = SCREEN.map(|side| side as f32 / 2.0);
    ViewFrame {
        to_canvas: [s, 0.0, cx - w * s, 0.0, s, cy - h * s],
        origin: [0, 0],
        size: SCREEN,
    }
}

/// The canvas target and its view tiers, as the runtime holds them.
struct Screen {
    target: CanvasTarget,
    view: ViewCache,
    /// The frame clock, 60 frames a second.
    now: f64,
}

impl Screen {
    fn new(core: &Core, artwork: &Artwork) -> Self {
        let mut target = CanvasTarget::new(core, SIZE).expect("target");
        RegionEvaluator::new(core, &mut target)
            .evaluate(&request(artwork, None), &[PixelRect::full(SIZE)])
            .expect("evaluate");
        Self {
            target,
            view: ViewCache::new(core),
            now: 0.0,
        }
    }

    /// One frame: the canvas evaluated for `regions` (and the stroke), then the tiers.
    fn frame(
        &mut self,
        core: &Core,
        (artwork, active): (&Artwork, Option<&ActiveStroke>),
        regions: &[PixelRect],
        view: &ViewFrame,
    ) -> ViewWork {
        let request = request(artwork, active);
        RegionEvaluator::new(core, &mut self.target)
            .evaluate(&request, regions)
            .expect("evaluate");
        self.now += 1.0 / 60.0;
        self.view
            .update(core, &mut self.target, &request, (view, self.now))
            .expect("view")
    }

    /// Frames until every needed tile is ready; the tiles evaluated whole meanwhile.
    fn settle(&mut self, core: &Core, artwork: &Artwork, view: &ViewFrame) -> BTreeSet<[u32; 2]> {
        let mut filled = BTreeSet::new();
        for _ in 0..200 {
            let work = self.frame(core, (artwork, None), &[], view);
            filled.extend(work.filled.iter().copied());
            if work.tier.is_some() && work.waiting == 0 {
                return filled;
            }
        }
        panic!("the tier never filled in");
    }

    /// Every ready tile of the tier compared with `export`; how many.
    fn assert_tiles(&self, core: &Core, export: &RgbaImage, scale: u32) -> usize {
        let [columns, rows] = export.size().map(|side| side.div_ceil(TILE));
        let mut compared = 0;
        for row in 0..rows {
            for column in 0..columns {
                let Some(tile) = self.view.tile_pixels(core, [column, row]).expect("read") else {
                    continue;
                };
                assert_eq!(
                    tile.pixels(),
                    crop(export, [column * TILE, row * TILE], tile.size()),
                    "tile {column}, {row} at {scale}x"
                );
                compared += 1;
            }
        }
        compared
    }
}

fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
    FrameRequest {
        artwork,
        active,
        background: Color::WHITE,
    }
}

fn crop(image: &RgbaImage, [x, y]: [u32; 2], [width, height]: [u32; 2]) -> Vec<u8> {
    let stride = image.size()[0] as usize * 4;
    (y..y + height)
        .flat_map(|row| {
            let at = row as usize * stride + x as usize * 4;
            image.pixels()[at..at + width as usize * 4].to_vec()
        })
        .collect()
}

/// The tiles of the tier at `scale` the canvas pixels `changed` reach (each widened by a canvas
/// pixel, as the plan places bounds).
fn reached(changed: &[PixelRect], scale: u32) -> BTreeSet<[u32; 2]> {
    let mut tiles = BTreeSet::new();
    let output = SIZE.map(|side| (side * scale) as i32);
    for rect in changed {
        let side = |value: i32, out: i32, axis: usize| {
            ((value + out) * scale as i32).clamp(0, output[axis])
        };
        let (x0, y0) = (side(rect.min[0], -1, 0), side(rect.min[1], -1, 1));
        let (x1, y1) = (side(rect.max[0], 1, 0), side(rect.max[1], 1, 1));
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        for row in y0 as u32 / TILE..=(y1 as u32 - 1) / TILE {
            for column in x0 as u32 / TILE..=(x1 as u32 - 1) / TILE {
                tiles.insert([column, row]);
            }
        }
    }
    tiles
}

/// At 2x, 4x and 8x every tile of `artwork`'s tier equals the export at that scale.
pub fn tiles_are_the_export(core: &Core, artwork: &Artwork) {
    for (zoom, scale) in [(2.2, 2), (3.7, 4), (7.0, 8)] {
        let mut screen = Screen::new(core, artwork);
        let view = frame(zoom, [150.0, 110.0]);
        screen.settle(core, artwork, &view);
        assert_eq!(screen.view.tier(), Some(scale));
        let compared = screen.assert_tiles(core, &whole(core, artwork, scale), scale);
        assert!(compared >= 4, "{compared} tiles at {scale}x");
    }
}

#[test]
fn a_filtered_tier_tile_is_the_export_at_its_scale_bit_for_bit() {
    tiles_are_the_export(&core(), &crate::filters::filtered());
}

#[test]
fn a_tier_tile_is_the_export_at_its_scale_bit_for_bit() {
    let core = core();
    let artwork = every_source();
    tiles_are_the_export(&core, &artwork);
    // At or below about one screen pixel per canvas pixel the canvas itself shows.
    let mut screen = Screen::new(&core, &artwork);
    let fit = frame(1.3, [160.0, 100.0]);
    for _ in 0..3 {
        let work = screen.frame(&core, (&artwork, None), &[], &fit);
        assert_eq!((work.tier, work.bytes), (None, 0));
    }
}

#[test]
fn changes_evaluate_again_exactly_the_tiles_they_reach() {
    let core = core();
    let first = every_source();
    let scale = 4;
    let view = frame(3.7, [150.0, 110.0]);
    let mut screen = Screen::new(&core, &first);
    let ready = screen.settle(&core, &first, &view);
    let stroke = with_stroke(
        &first,
        (
            plain(6.0, 1.0, Color::BLACK),
            vec![sample(120.0, 90.0, 1.0, 0), sample(150.0, 100.0, 1.0, 8)],
        ),
    );
    let square = |x: u32, y: u32| (130..150).contains(&x) && (100..118).contains(&y);
    let pixels: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| square(at % SIZE[0], at / SIZE[0]))
        .collect();
    let canvas = stroke.canvas();
    let mask = RegionMask::from_pixels(canvas, &pixels).expect("mask");
    let filled = stroke
        .fill_region(mask.clone(), Color::rgb(0.2, 0.8, 0.3).expect("colour"))
        .expect("fill")
        .artwork;
    let turn = Placement::around([140.0, 109.0], 1.2, 0.3, [false, false]).expect("turn");
    let moved = filled
        .place_region(Some(mask), turn)
        .expect("moved")
        .artwork;
    let mut shown = first.clone();
    // Each edit, then undo back to the start.
    for next in [stroke, filled, moved, first.clone()] {
        let changed = changed_regions(&shown, &next);
        let work = screen.frame(&core, (&next, None), &changed, &view);
        let again: BTreeSet<[u32; 2]> = work.updated.iter().chain(&work.dropped).copied().collect();
        let expected: BTreeSet<[u32; 2]> = reached(&changed, scale)
            .intersection(&ready)
            .copied()
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(again, expected, "the tiles the change reaches, no others");
        screen.settle(&core, &next, &view);
        let compared = screen.assert_tiles(&core, &whole(&core, &next, scale), scale);
        assert_eq!(compared, ready.len(), "every needed tile stays ready");
        assert_eq!(
            screen.target.read(&core).expect("canvas"),
            cold(&core, &next)
        );
        shown = next;
    }
}

#[test]
fn a_stroke_drawn_while_zoomed_shows_at_the_tier_as_committed() {
    let style = plain(5.0, 1.0, Color::rgb(0.8, 0.1, 0.4).expect("colour"));
    drawn_while_zoomed(&style, &style);
}

/// The same with a taper at both ends (S44): while drawn, the tiles are the export of the
/// stroke so far without its end taper; from the lift on, of the committed stroke.
#[test]
fn a_tapered_stroke_drawn_while_zoomed_narrows_its_end_at_the_tier_at_the_lift() {
    let color = Color::rgb(0.8, 0.1, 0.4).expect("colour");
    let tapered = |end: f32| {
        let taper = openpaint_core::Taper::new(1.0, end, 0.1).expect("taper");
        let recipe = openpaint_core::BrushRecipe::plain().with_taper(taper);
        StrokeStyle::new(std::sync::Arc::new(recipe), 5.0, 1.0, color).expect("style")
    };
    drawn_while_zoomed(&tapered(2.0), &tapered(0.0));
}

/// `style` drawn sample by sample at 8x: each frame's tiles are the export of the stroke so far
/// drawn with `blunt` (`style` without its end taper), and once committed the export of it.
pub fn drawn_while_zoomed(style: &StrokeStyle, blunt: &StrokeStyle) {
    let core = core();
    let base = every_source();
    let scale = 8;
    let view = frame(7.0, [150.0, 110.0]);
    let mut screen = Screen::new(&core, &base);
    screen.settle(&core, &base, &view);
    let first = sample(130.0, 95.0, 0.6, 0);
    let mut session = base.begin_stroke(style.clone(), first).expect("begin");
    let mut drawn = vec![first];
    for step in 1..12u16 {
        let at = f32::from(step);
        let next = sample(
            130.0 + 3.0 * at,
            95.0 + 2.0 * at,
            0.6 + 0.03 * at,
            u32::from(step) * 8,
        );
        session.extend(&[next]).expect("extend");
        drawn.push(next);
        let work = screen.frame(&core, (&base, Some(session.active())), &[], &view);
        assert!(
            work.dropped.is_empty(),
            "a stroke's few pixels are evaluated in place"
        );
        let so_far = with_stroke(&base, (blunt.clone(), drawn.clone()));
        screen.assert_tiles(&core, &whole(&core, &so_far, scale), scale);
    }
    let committed = session.finish().artwork;
    screen.frame(&core, (&committed, None), &[], &view);
    screen.assert_tiles(&core, &whole(&core, &committed, scale), scale);
    assert_eq!(
        screen.target.read(&core).expect("canvas"),
        cold(&core, &committed)
    );
}
