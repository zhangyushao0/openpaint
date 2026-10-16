//! The view tiers as the runtime drives them, for the tier tests (`view.rs`, `raster_smudge.rs`):
//! the canvas target and its tiers on a screen, the raster worker's side served after each frame,
//! and what the tiles are compared with.

use std::collections::BTreeSet;

use openpaint_core::{ActiveStroke, Artwork, Color};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RasterDrawer, RegionEvaluator, RgbaImage,
    ViewCache, ViewFrame, ViewWork, changed_regions,
};

use crate::fixture::{SIZE, cold, core};
use crate::raster::assert_near;
use crate::scale::whole;

/// The screen the tests show the canvas on, in pixels.
const SCREEN: [u32; 2] = [400, 300];
pub const TILE: u32 = 256;

/// `zoom` screen pixels per canvas pixel, canvas point `centre` in the screen's middle.
pub fn frame(zoom: f32, [cx, cy]: [f32; 2]) -> ViewFrame {
    let s = 1.0 / zoom;
    let [w, h] = SCREEN.map(|side| side as f32 / 2.0);
    ViewFrame {
        to_canvas: [s, 0.0, cx - w * s, 0.0, s, cy - h * s],
        origin: [0, 0],
        size: SCREEN,
    }
}

/// The canvas target and its view tiers, as the runtime holds them, and the raster worker's side
/// (S60), served at once after each frame.
pub struct Screen {
    pub target: CanvasTarget,
    pub view: ViewCache,
    drawer: RasterDrawer,
    /// The frame clock, 60 frames a second.
    now: f64,
}

impl Screen {
    pub fn new(core: &Core, artwork: &Artwork) -> Self {
        let mut target = CanvasTarget::new(core, SIZE).expect("target");
        RegionEvaluator::new(core, &mut target)
            .evaluate(&request(artwork, None), &[PixelRect::full(SIZE)])
            .expect("evaluate");
        Self {
            target,
            view: ViewCache::new(core),
            drawer: RasterDrawer::default(),
            now: 0.0,
        }
    }

    /// One frame: the canvas evaluated for `regions` (and the stroke), then the tiers, then the
    /// tiles they handed to the raster worker drawn (shown from the next frame on).
    pub fn frame(
        &mut self,
        core: &Core,
        (artwork, active): (&Artwork, Option<&ActiveStroke>),
        regions: &[PixelRect],
        view: &ViewFrame,
    ) -> ViewWork {
        let work = self.update(core, (artwork, active), regions, view);
        self.drawer.serve(core, &mut self.view);
        work
    }

    /// `frame` without the worker: the tiles handed over wait for `serve`.
    pub fn update(
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

    /// The raster worker draws the tiles handed over since it last did.
    pub fn serve(&mut self, core: &Core) {
        self.drawer.serve(core, &mut self.view);
    }

    /// Frames until every needed tile is ready; the tiles evaluated whole meanwhile.
    pub fn settle(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        view: &ViewFrame,
    ) -> BTreeSet<[u32; 2]> {
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
    pub fn assert_tiles(&self, core: &Core, export: &RgbaImage, scale: u32) -> usize {
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

    /// Every ready tile of the tier against `export` by D86's rule (`assert_near`).
    pub fn assert_tiles_near(&self, core: &Core, export: &RgbaImage) {
        let [columns, rows] = export.size().map(|side| side.div_ceil(TILE));
        for row in 0..rows {
            for column in 0..columns {
                let Some(tile) = self.view.tile_pixels(core, [column, row]).expect("read") else {
                    continue;
                };
                let part = crop(export, [column * TILE, row * TILE], tile.size());
                let part = RgbaImage::from_rgba(tile.size(), part).expect("cropped");
                assert_near(&part, &tile, &format!("tile {column}, {row}"));
            }
        }
    }
}

pub fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
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

/// After each of `edits` from `first`, exactly the tiles the change reaches are evaluated or drawn
/// again, and once filled in every tile equals the export at the tier's scale.
pub fn changes_reach_their_tiles(first: &Artwork, edits: &[Artwork]) {
    let core = core();
    let scale = 4;
    let view = frame(3.7, [150.0, 110.0]);
    let mut screen = Screen::new(&core, first);
    let ready = screen.settle(&core, first, &view);
    let mut shown = first.clone();
    // Each edit, then undo back to the start.
    for next in edits {
        let changed = changed_regions(&shown, next);
        let work = screen.frame(&core, (next, None), &changed, &view);
        let again: BTreeSet<[u32; 2]> = work.updated.iter().chain(&work.dropped).copied().collect();
        let expected: BTreeSet<[u32; 2]> = reached(&changed, scale)
            .intersection(&ready)
            .copied()
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(again, expected, "the tiles the change reaches, no others");
        screen.settle(&core, next, &view);
        let compared = screen.assert_tiles(&core, &whole(&core, next, scale), scale);
        assert_eq!(compared, ready.len(), "every needed tile stays ready");
        assert_eq!(
            screen.target.read(&core).expect("canvas"),
            cold(&core, next)
        );
        shown = next.clone();
    }
}
