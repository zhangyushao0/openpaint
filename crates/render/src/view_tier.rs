//! `ViewCache`: sharp zoom. While the view magnifies the canvas by more than about √2, the
//! screen shows a view tier instead of the canvas target: the composited canvas drawn at a
//! power-of-two scale (2, 4 or 8 output pixels per canvas pixel), in tiles of `TIER_TILE` output
//! pixels aligned to the canvas, holding the visible tiles and a ring of one tile. The canvas
//! target stays the truth for everything else and is drawn first each frame; the tier draws
//! through its scene and takes what it changed. A stroke's frames are the tier's alone: the
//! canvas draws one after it only where it could not, and takes the stroke whole at its lift.
//!
//! Each frame: a change to a ready tile is drawn in place when it fits the frame, else the tile
//! waits to be drawn whole (the canvas shows meanwhile); missing tiles are drawn whole by the
//! raster worker as background work, visible ones nearest the screen's centre first
//! (`view_raster_fill.rs`); the stroke being drawn is drawn at once where the view shows it
//! (`view_raster.rs`). A new tier starts only once the view stands still (a pinch ends); until it
//! has every visible tile the one before shows under it, resampled.
//!
//! Tiles the view no longer needs stay ready until a needed tile wants their slot, and a tier the
//! view leaves is kept whole (`view_kept.rs`), its tiles kept up to date by dropping those a
//! change reaches, so a return to a place or a scale shows at once what was there; tiers are let
//! go only when the GPU memory budget says (the least recently shown first), never the tier
//! shown.

use std::f32::consts::SQRT_2;

use crate::canvas::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::output::{MAX_SCALE, Output};
use crate::readback::texture_bytes;
use crate::view_atlas::Tier;
use crate::view_fill::{Filling, touched};
use crate::view_needed::Needed;
use crate::view_raster::{RasterTier, in_place_raster, snapshots_ahead, stroke_raster};
use crate::view_raster_fill::fill_raster;
use crate::view_shown::Shown;
use crate::view_tiles::TileMap;

/// How long the view stands still before a new tier starts: a pause in a pinch is no end.
const SETTLE_SECONDS: f64 = 0.12;

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
    pub(crate) current: Option<Tier>,
    /// The tier shown before the current one, under it until the current has every visible tile.
    pub(crate) previous: Option<Tier>,
    /// Tiers the view left, whole, for a return to their scale.
    pub(crate) kept: Vec<Tier>,
    /// The view of the last update and when (seconds) it became that view: one unchanged for
    /// `SETTLE_SECONDS` has settled.
    last: Option<(ViewFrame, f64)>,
    shown: Shown,
    /// Tiles being filled since the view last needed none.
    filling: Option<Filling>,
    /// The most bytes the tiers held at once.
    peak: u64,
    /// The most bytes the tiers may hold, as the GPU memory budget leaves them: a
    /// tier that would not fit, the others let go, is not made and the canvas shows.
    pub(crate) room: u64,
    /// The tier the view wants did not fit the room.
    refused: bool,
    /// The tiles handed to the raster worker.
    pub(crate) raster: RasterTier,
    /// The atlas of the next new tier, made ahead on the raster worker, and whether one is wanted
    ///.
    pub(crate) spare: Option<wgpu::Texture>,
    pub(crate) wanting: bool,
    /// The snapshots under the next stroke of the tier shown are being taken ahead or wait to be
    /// (`view_raster.rs`).
    pub(crate) taking: bool,
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
            kept: Vec::new(),
            last: None,
            shown: Shown::new(core),
            peak: 0,
            filling: None,
            room: u64::MAX,
            refused: false,
            raster: RasterTier::default(),
            spare: None,
            wanting: true,
            taking: false,
        }
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
        let dirtied = canvas.take_dirtied();
        // The canvas's own raster draws count as a use of the targets it shares with the tiers.
        if std::mem::take(&mut canvas.raster.drew) {
            canvas.raster.used_at = now;
        }
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
        self.take_answers(core, canvas, &mut work);
        self.reached(&changed);
        for tier in [&mut self.current, &mut self.previous]
            .into_iter()
            .flatten()
        {
            tier.shown_at = now;
        }
        let view = (frame, now, wanted);
        let changes = ((&changed[..], &dirtied[..]), &mut work);
        let covered = self.work_on_current(core, (canvas, request), view, changes)?;
        // A frame of the stroke the tier could not draw whole is the canvas's now.
        let late = canvas.stroke_left(core, request, covered)?;
        self.reached(&late);
        canvas.tiered = self.current.is_some();
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
        // The first tier's atlas is made ahead, once nothing touches the screen.
        self.ask_spare(core, canvas, frame);
        work.bytes = self.bytes();
        self.filling = Filling::step(self.filling.take(), &work);
        if work.bytes > self.peak {
            self.peak = work.bytes;
            log::info!("view tiers hold {:.1} MB (most so far)", mb(self.peak));
        }
        Ok(work)
    }

    /// Tiers not shown in full keep only tiles no change reached (canvas pixels `changed`); tiles
    /// the raster worker draws that a change reached are drawn again.
    fn reached(&mut self, changed: &[PixelRect]) {
        for tier in self.previous.iter_mut().chain(self.kept.iter_mut()) {
            for (tile, _) in touched(tier, changed) {
                tier.set_ready(tile, false);
            }
        }
        let tiers = (self.current.iter_mut())
            .chain(self.previous.iter_mut())
            .chain(self.kept.iter_mut());
        for tier in tiers {
            for tile in self.raster.reached(&tier.map, changed) {
                tier.set_ready(tile, false);
            }
        }
    }

    /// The shown tier's work this frame: what changed in place, then a share of the missing tiles
    /// when it is the tier `wanted`, handed to the raster worker. Its drawing follows
    /// the artwork first, the canvas pixels `dirtied` naming what changes reached. Whether it
    /// drew the stroke being drawn (or lifted) at every tile the frame reached.
    fn work_on_current(
        &mut self,
        core: &Core,
        (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
        (frame, now, wanted): (&ViewFrame, f64, Option<u32>),
        ((changed, dirtied), work): ((&[PixelRect], &[PixelRect]), &mut ViewWork),
    ) -> Result<bool, RenderError> {
        self.taking = false;
        let raster = &mut self.raster;
        let Some(current) = &mut self.current else {
            return Ok(false);
        };
        let scale = current.scale();
        canvas.follow(&mut current.drawing, request, (Some(dirtied), scale));
        // What changed shows at once when it fits; missing tiles are background work. The
        // stroke being drawn, and its lift, at once where the view shows them.
        let mut covered = false;
        // The tiles the view needs, once a frame (their grid and the view do not change in it).
        let needed = Needed::of(&current.map, frame);
        if request.active.is_some() || current.drawing.tracking() {
            let place = (&mut *canvas, request);
            stroke_raster(core, (current, &needed), place, (work, now))?;
            covered = work.dropped.is_empty();
        } else {
            in_place_raster(core, current, (&mut *canvas, request), (changed, work, now))?;
        }
        if Some(current.scale()) != wanted {
            return Ok(covered);
        }
        // Tiles the view left stay ready until a needed tile wants their slot.
        // The canvas fills in first: no missing tile is filled meanwhile.
        if !canvas.pending() {
            fill_raster(core, (current, &needed), (canvas, request), raster, now)?;
        }
        work.waiting = needed.waiting(current);
        let ready = work.waiting == 0;
        let taking = snapshots_ahead(core, (current, &needed), (canvas, request), ready)?;
        if needed.shows(current) {
            self.keep_previous();
        }
        self.taking = taking;
        Ok(covered)
    }

    /// Starts tier `wanted` (or none): the tier shown so far becomes the previous one, shown
    /// under it while it fills; a tier of that scale kept from before is taken up again with
    /// what it holds, else a new one is made where the room holds it, kept tiers let go
    /// for it, the least recently shown first.
    fn switch(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        wanted: Option<u32>,
        frame: &ViewFrame,
    ) -> Result<(), RenderError> {
        self.keep_previous();
        self.previous = self.current.take();
        // Only the tier shown draws a stroke: another's drawing goes with its slots.
        if let Some(previous) = self.previous.as_mut() {
            previous.drawing = crate::canvas::Drawing::default();
        }
        let Some(scale) = wanted else {
            // The canvas shows: the tier left is kept whole.
            self.keep_previous();
            return Ok(());
        };
        let zoom = zoom(&frame.to_canvas);
        if let Some(at) = self.kept.iter().position(|tier| tier.scale() == scale) {
            let tier = self.kept.swap_remove(at);
            log::info!(
                "view tier {scale}x for zoom {zoom:.2}: kept, {} tiles ready, {:.1} MB; the \
                 tiers hold {:.1} MB",
                tier.ready_tiles(),
                mb(tier.bytes()),
                mb(self.bytes() + tier.bytes())
            );
            self.current = Some(tier);
            return Ok(());
        }
        let canvas = request.artwork.canvas();
        let output = Output::at(scale, [0, 0])?;
        let map = TileMap::new([canvas.width(), canvas.height()], output, (1, 1));
        let slots = Needed::of(&map, frame).count();
        let bytes = Tier::bytes_for(&map, slots as u32);
        while self.bytes() + bytes > self.room && self.let_go_oldest() > 0 {}
        // A tier the GPU memory budget leaves no room for is not made.
        if self.bytes() + bytes > self.room {
            self.refused = true;
            return Ok(());
        }
        let making = std::time::Instant::now();
        let tier = Tier::new(
            core,
            (request.artwork, scale),
            slots as u32,
            &mut self.spare,
        )?;
        log::info!(
            "view tier {scale}x for zoom {zoom:.2}: {slots} tiles wanted, {:.1} MB made in {} \
             ms; the tiers hold {:.1} MB",
            mb(tier.bytes()),
            making.elapsed().as_millis(),
            mb(self.bytes() + tier.bytes())
        );
        self.current = Some(tier);
        Ok(())
    }

    /// The tier shown, output pixels per canvas pixel; `None` while the canvas target shows.
    #[must_use]
    pub fn tier(&self) -> Option<u32> {
        self.current.as_ref().map(Tier::scale)
    }

    /// Bytes the tiers' textures hold now, kept tiers included.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        [&self.current, &self.previous]
            .into_iter()
            .flatten()
            .chain(&self.kept)
            .map(Tier::bytes)
            .sum::<u64>()
            + self.spare.as_ref().map_or(0, texture_bytes)
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

pub(crate) fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[cfg(test)]
#[path = "view_tier_tests.rs"]
mod tests;
