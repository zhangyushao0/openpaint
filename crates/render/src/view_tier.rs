//! `ViewCache`: sharp zoom (S32). While the view magnifies the canvas by more than about
//! √2, the screen shows a view tier instead of the canvas target: the composited canvas
//! evaluated at a power-of-two scale (2, 4 or 8 output pixels per canvas pixel), in tiles of
//! `TIER_TILE` output pixels aligned to the canvas, holding the visible tiles and a ring of one
//! tile. The canvas target stays the truth for everything else and is evaluated first each
//! frame; the tier evaluates through its scene and takes what it changed.
//!
//! Each frame: a change to a ready tile is evaluated in place when it is small, else the tile
//! waits to be evaluated whole (the canvas shows meanwhile); missing tiles are evaluated whole,
//! at most `FILL_TILES` a frame, visible ones nearest the screen's centre first; tiles beyond
//! two rings are dropped. A new tier starts only once the view stands still (a pinch ends);
//! until it has every visible tile the one before shows under it, resampled.

use std::f32::consts::SQRT_2;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::image::RgbaImage;
use crate::output::{MAX_SCALE, Output};
use crate::view_atlas::Tier;
use crate::view_background::fill;
use crate::view_fill::{Filling, in_place, release_far, touched};
use crate::view_needed::Needed;
use crate::view_shown::Shown;
use crate::view_tiles::TileMap;

/// How long the view stands still before a new tier starts: a pause in a pinch is no end.
const SETTLE_SECONDS: f64 = 0.12;
/// The most bytes the current and the previous tier hold together: past it, the previous one
/// goes when a tier starts (the canvas shows under the new one meanwhile).
const BUDGET_BYTES: u64 = 90 << 20;

/// Where the canvas shows on screen: the view's map from screen pixels to canvas pixels (as
/// `CanvasPlacement::to_canvas`) and the canvas area, top-left and size in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewFrame {
    pub to_canvas: [f32; 6],
    pub origin: [i32; 2],
    pub size: [u32; 2],
}

/// What one `ViewCache::update` did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ViewWork {
    /// The tier shown, output pixels per canvas pixel; `None`: the canvas target itself.
    pub tier: Option<u32>,
    /// View tiles (column, row) evaluated whole.
    pub filled: Vec<[u32; 2]>,
    /// Ready tiles whose changed pixels were evaluated in place.
    pub updated: Vec<[u32; 2]>,
    /// Ready tiles a large change made wait to be evaluated whole.
    pub dropped: Vec<[u32; 2]>,
    /// The view wants another tier than the one shown: it starts once the view stands still.
    pub changing: bool,
    /// Visible and ring tiles not ready yet.
    pub waiting: usize,
    /// Bytes of every tier's textures now.
    pub bytes: u64,
}

/// The view tiers of one canvas target.
pub struct ViewCache {
    current: Option<Tier>,
    /// The tier shown before the current one, until the current has every visible tile.
    previous: Option<Tier>,
    /// The view of the last update and when (seconds) it became that view: one unchanged for
    /// `SETTLE_SECONDS` has settled.
    last: Option<(ViewFrame, f64)>,
    shown: Shown,
    /// Tiles being filled since the view last needed none.
    filling: Option<Filling>,
    /// The most bytes the tiers held at once.
    peak: u64,
    /// The most bytes the tiers may hold: `BUDGET_BYTES`, or less when the GPU memory budget
    /// leaves less (S49); a tier that would not fit is not made and the canvas shows.
    room: u64,
    /// The tier the view wants did not fit the room.
    refused: bool,
}

impl ViewWork {
    /// More frames would change what shows: tiles are missing or another tier is wanted.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.changing || self.waiting > 0
    }
}

impl ViewCache {
    #[must_use]
    pub fn new(core: &Core) -> Self {
        Self {
            current: None,
            previous: None,
            last: None,
            shown: Shown::new(core),
            peak: 0,
            filling: None,
            room: BUDGET_BYTES,
            refused: false,
        }
    }

    /// The tiers hold at most `bytes` from now on (never more than 90 MB): when they hold more,
    /// the tier before goes, then the tier shown (the canvas shows instead). Returns the bytes
    /// given back.
    pub fn set_room(&mut self, bytes: u64) -> u64 {
        self.room = bytes.min(BUDGET_BYTES);
        let before = self.bytes();
        if self.bytes() > self.room {
            self.previous = None;
        }
        if self.bytes() > self.room {
            self.current = None;
        }
        before - self.bytes()
    }

    /// Brings the tiers up to date with `canvas`, just evaluated for `request`, shown through
    /// `frame` at `now` (seconds on the frame clock): takes what the canvas changed, picks the
    /// tier for the view's zoom (a new one once the view stood still `SETTLE_SECONDS`), evaluates
    /// changes and missing tiles within the frame's budget.
    ///
    /// # Errors
    /// `Invalid` when `canvas` was not evaluated for `request` this frame; device errors.
    pub fn update(
        &mut self,
        core: &Core,
        canvas: &mut CanvasTarget,
        request: &FrameRequest<'_>,
        (frame, now): (&ViewFrame, f64),
    ) -> Result<ViewWork, RenderError> {
        let changed = canvas.take_changed();
        let wanted = tier_for(zoom(&frame.to_canvas));
        let since = match self.last {
            Some((last, since)) if last == *frame => since,
            _ => now,
        };
        self.last = Some((*frame, since));
        let settled = now - since >= SETTLE_SECONDS;
        self.refused = false;
        if settled && self.current.as_ref().map(Tier::scale) != wanted {
            self.switch(core, request, wanted, frame)?;
        }
        let mut work = ViewWork::default();
        if let Some(previous) = &mut self.previous {
            for (tile, _) in touched(previous, &changed) {
                previous.set_ready(tile, false);
            }
        }
        if let Some(current) = &mut self.current {
            // What changed shows at once when it fits; missing tiles are background work (S53).
            in_place(core, current, (canvas, request), (&changed, &mut work))?;
            if Some(current.scale()) == wanted {
                let needed = Needed::of(&current.map, frame);
                release_far(current, &needed);
                // The canvas fills in first (S49): no missing tile is filled meanwhile.
                if !canvas.pending() {
                    fill(core, (current, &needed), (canvas, request), &mut work)?;
                }
                work.waiting = needed.waiting(current);
                if needed.shows(current) {
                    self.previous = None;
                }
            }
        }
        for tier in [&mut self.current, &mut self.previous]
            .into_iter()
            .flatten()
        {
            tier.upload_index(core);
        }
        self.shown
            .refresh(core, [self.current.as_ref(), self.previous.as_ref()]);
        work.tier = self.current.as_ref().map(Tier::scale);
        work.changing = work.tier != wanted && !self.refused;
        work.bytes = self.bytes();
        self.filling = Filling::step(self.filling.take(), &work);
        if work.bytes > self.peak {
            self.peak = work.bytes;
            log::info!("view tiers hold {:.1} MB (most so far)", mb(self.peak));
        }
        Ok(work)
    }

    /// Starts tier `wanted` (or none); the tier shown so far becomes the previous one.
    fn switch(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        wanted: Option<u32>,
        frame: &ViewFrame,
    ) -> Result<(), RenderError> {
        self.previous = None;
        let old = self.current.take();
        if let Some(scale) = wanted {
            let canvas = request.artwork.canvas();
            let output = Output::at(scale, [0, 0])?;
            let map = TileMap::new([canvas.width(), canvas.height()], output, (1, 1));
            let slots = Needed::of(&map, frame).count();
            // A tier the GPU memory budget leaves no room for is not made (S49).
            if Tier::bytes_for(&map, slots as u32) > self.room {
                self.refused = true;
                return Ok(());
            }
            let tier = Tier::new(core, request.artwork, scale, slots as u32)?;
            // The tier shown so far stays under the new one where the view shows it, in an
            // atlas of just those tiles, while both fit the budget.
            let previous = match old {
                Some(old) => {
                    let shows = Needed::of(&old.map, frame);
                    old.kept(core, request.artwork, |tile| shows.shown(tile))?
                }
                None => None,
            };
            self.previous =
                previous.filter(|previous| previous.bytes() + tier.bytes() <= self.room);
            log::info!(
                "view tier {scale}x for zoom {:.2}: {slots} tiles wanted, {:.1} MB, the tier before kept: {:.1} MB",
                zoom(&frame.to_canvas),
                mb(tier.bytes()),
                mb(self.previous.as_ref().map_or(0, Tier::bytes))
            );
            self.current = Some(tier);
        }
        Ok(())
    }

    /// The tier shown, output pixels per canvas pixel; `None` while the canvas target shows.
    #[must_use]
    pub fn tier(&self) -> Option<u32> {
        self.current.as_ref().map(Tier::scale)
    }

    /// Bytes the tiers' textures hold now.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        [&self.current, &self.previous]
            .into_iter()
            .flatten()
            .map(Tier::bytes)
            .sum()
    }

    /// The most bytes the tiers held at once so far.
    #[must_use]
    pub fn peak(&self) -> u64 {
        self.peak
    }

    /// View tile `tile` (column, row) of the current tier, read back (waits for the GPU);
    /// `None` when it is not ready.
    ///
    /// # Errors
    /// Device errors.
    pub fn tile_pixels(
        &self,
        core: &Core,
        tile: [u32; 2],
    ) -> Result<Option<RgbaImage>, RenderError> {
        match &self.current {
            Some(tier) => tier.read(core, tile),
            None => Ok(None),
        }
    }

    pub(crate) fn group(&self) -> &wgpu::BindGroup {
        self.shown.group()
    }
}

/// Screen pixels per canvas pixel of the view's map.
fn zoom(to_canvas: &[f32; 6]) -> f32 {
    let [a, b, _, d, e, _] = *to_canvas;
    let area = (a * e - b * d).abs();
    if area > 0.0 { area.sqrt().recip() } else { 0.0 }
}

/// The tier for `zoom`: none up to √2 (the canvas itself, magnified at most √2), else the
/// power of two nearest in ratio (a tier is shown between √2 smaller and √2 larger), at most
/// `MAX_SCALE`. Nearest rather than the next above: it holds at most twice the screen's
/// pixels instead of four times, and an edge stays within about 1.4 screen pixels.
pub(crate) fn tier_for(zoom: f32) -> Option<u32> {
    if zoom.is_nan() || zoom <= SQRT_2 {
        return None;
    }
    let power = zoom.log2().round().clamp(1.0, MAX_SCALE.ilog2() as f32);
    Some(1 << power as u32)
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tier_is_the_nearest_power_of_two_above_the_square_root_of_two() {
        let tiers: Vec<Option<u32>> = [
            0.5, 1.0, 1.41, 1.42, 2.0, 2.82, 2.84, 4.0, 5.6, 5.7, 8.0, 30.0,
        ]
        .map(tier_for)
        .into();
        let expected = [
            None,
            None,
            None,
            Some(2),
            Some(2),
            Some(2),
            Some(4),
            Some(4),
            Some(4),
        ]
        .into_iter()
        .chain([Some(8), Some(8), Some(8)]);
        assert!(tiers.into_iter().eq(expected));
        assert_eq!(zoom(&[0.25, 0.0, 3.0, 0.0, 0.25, 1.0]), 4.0);
        let turned = std::f32::consts::FRAC_1_SQRT_2 / 2.0;
        let rotated = zoom(&[turned, -turned, 0.0, turned, turned, 0.0]);
        assert!((rotated - 2.0).abs() < 1e-5, "{rotated}");
    }
}
