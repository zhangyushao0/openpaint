//! A view tier's tiles drawn by the raster core: a missing tile is drawn whole in one region of
//! 256 pixels a side into its slot, as background work, the screen's middle first; a change to
//! ready tiles is drawn in place when it fits what is left of the frame, else the tiles wait to be
//! drawn again. While a stroke is drawn the tier's own drawing of it updates the tiles it reaches
//! that the view shows (`stroke_raster`), so a tier equals the export at its scale bit for bit.
//!
//! The background tiles are planned, encoded and submitted by the runtime's raster worker
//! (`raster_worker.rs`); this side hands them over (`view_raster_fill.rs`) and draws the changes
//! in place with the frame loop's targets, which are the canvas's (`canvas/raster.rs`: the canvas
//! draws its own changes with them, and holds the worker's orders and answers for both). The
//! smudge residuals a tile reads are the canvas's, made present before it is drawn or handed over
//! and named with the arena's buffer as it is then (`smudge::Held`); a tile reading one still
//! being evaluated waits.

use crate::canvas::{CanvasTarget, EDIT_PIXELS, Onto, Planned, TAKING, paper};

use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster_plan::RasterPlan;
use crate::scene::Scene;
use crate::smudge::{Bases, Held, around_output};
use crate::view_atlas::Tier;
use crate::view_fill::touched;
use crate::view_needed::Needed;
use crate::view_tier::{ViewCache, ViewWork};
use crate::view_tiles::TileMap;
use crate::waiting::far;

/// The view tiers' side of the raster worker: the tiles handed to it.
#[derive(Default)]
pub(crate) struct RasterTier {
    /// Tiles handed to the worker and not answered yet, oldest first.
    pub(crate) issued: Vec<Issued>,
}

/// A tile handed to the raster worker: its job, its tier's scale, the tile, the slot and the
/// atlas it goes into, and whether a change reached it since it was handed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Issued {
    pub(crate) id: u64,
    pub(crate) scale: u32,
    pub(crate) tile: [u32; 2],
    pub(crate) slot: u32,
    pub(crate) grown: u32,
    pub(crate) stale: bool,
}

/// The plan of output pixels `rect` at `scale` of `scene`, its smudges' residuals and filters'
/// results at `bases`, with an atlas of `atlas` pixels.
fn plan(
    (scene, bases): (&Scene, &Bases),
    (scale, rect): (u32, PixelRect),
    atlas: [u32; 2],
) -> Result<(RasterPlan, Output), RenderError> {
    let origin = rect.min.map(|at| at.max(0) as u32);
    let output = Output::at(scale, origin)?;
    let size = rect.size();
    Ok((
        RasterPlan::build((scene, bases), (output, size), atlas)?,
        output,
    ))
}

/// Draws the pixels of ready tiles that `changed` (canvas pixels) reach again, when they are few,
/// the residuals they read are at hand and their estimate fits what is left of the frame budget;
/// else (or while the raster core's pipelines are still being made) the tiles they touch
/// wait to be drawn whole as background work.
///
/// # Errors
/// `Invalid` when the canvas was not evaluated for `request` this frame; as the raster core.
pub(crate) fn in_place_raster(
    core: &Core,
    tier: &mut Tier,
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    (changed, work, now): (&[PixelRect], &mut ViewWork, f64),
) -> Result<(), RenderError> {
    let parts = touched(tier, changed);
    if parts.is_empty() {
        return Ok(());
    }
    let mut tiles: Vec<[u32; 2]> = parts.iter().map(|(tile, _)| *tile).collect();
    tiles.sort_unstable();
    tiles.dedup();
    let area: i64 = (parts.iter())
        .map(|(_, part)| {
            i64::from(part.max[0] - part.min[0]) * i64::from(part.max[1] - part.min[1])
        })
        .sum();
    let scale = tier.scale();
    let near: Vec<PixelRect> = (parts.iter())
        .map(|(_, part)| around_output(part, scale))
        .collect();
    let held = if area <= EDIT_PIXELS && core.raster_made().is_some() {
        residuals_near(core, (canvas, request), &near)?
    } else {
        None
    };
    if let Some(held) = held
        && draw_in_place(core, (tier, &parts), (canvas, request), held)?
    {
        canvas.raster.used_at = now;
        work.updated = tiles;
        return Ok(());
    }
    for tile in &tiles {
        tier.set_ready(*tile, false);
    }
    work.dropped = tiles;
    Ok(())
}

/// The stroke being drawn (or just lifted or dropped) at `tier` (`canvas/drawing_frame/`): drawn
/// by the tier's drawing into the tiles the view shows that are ready, at once; the tiles it
/// reaches that are not, or that the view does not show (its ring), are not ready any more (drawn
/// whole once no stroke is).
///
/// # Errors
/// As the raster core.
pub(crate) fn stroke_raster(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    (work, now): (&mut ViewWork, f64),
) -> Result<(), RenderError> {
    let started = std::time::Instant::now();
    let mut drawing = std::mem::take(&mut tier.drawing);
    let moved = std::mem::take(&mut canvas.moved);
    let done = {
        let held = &*tier;
        let place = |tile: [u32; 2]| {
            let shown = needed.is_shown(tile) && held.is_ready(tile);
            let slot = held.map.slot(tile).filter(|_| shown)?;
            Some(held.map.slot_origin(slot))
        };
        let onto = Onto {
            scale: held.scale(),
            texture: held.atlas(),
            view: held.atlas_view(),
            format: held.atlas().format(),
            place: &place,
        };
        canvas.draw_stroke(core, request, &mut drawing, (&onto, &moved))
    };
    tier.drawing = drawing;
    canvas.moved = moved;
    let done = done?;
    crate::profile::tier_drew(done.drew(), started.elapsed());
    for tile in &done.skipped {
        tier.set_ready(*tile, false);
    }
    canvas.raster.used_at = now;
    work.updated = done.drawn;
    work.updated.dedup();
    work.dropped = done.skipped;
    Ok(())
}

/// Once its shown tiles are ready (`ready`) and the canvas waits for nothing, the snapshots
/// under the next stroke of `tier`'s shown tiles (`needed`) handed to the raster worker while
/// nothing touches the screen, or one a frame while the pen hovers, nearest it first
/// (`canvas/drawing_ahead.rs`); whether any is being taken or waits to be (within the
/// snapshots' room).
///
/// # Errors
/// As the raster core.
pub(crate) fn snapshots_ahead(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    ready: bool,
) -> Result<bool, RenderError> {
    // Where no pen lands now (not shown, nor the ring): its snapshots go.
    tier.drawing.keep_only(|tile| needed.needs(tile));
    let drawing = &tier.drawing;
    if !ready || canvas.pending() || drawing.base.is_none() || drawing.tracking() {
        return Ok(false);
    }
    let others = canvas.bytes().snapshots;
    let room = TAKING.saturating_sub(drawing.taking.len());
    let scale = tier.scale();
    let mut wanted = drawing.wanted(nearest(needed, canvas.hovered(), &tier.map), room);
    let pending =
        !drawing.taking.is_empty() || (!wanted.is_empty() && canvas.room_for(drawing, others));
    let quiet = {
        let pace = core.pace();
        pace.idle() || pace.hovering()
    };
    if !quiet {
        return Ok(pending);
    }
    wanted.truncate(canvas.ahead_now(core, wanted.len()));
    let mut drawing = std::mem::take(&mut tier.drawing);
    let handed = canvas.made_ahead(core, &mut drawing).and_then(|()| {
        canvas.hand_snapshots(core, request, &mut drawing, (scale, &wanted, others))
    });
    tier.drawing = drawing;
    handed.map(|()| pending)
}

/// The shown tiles of `map`'s tier, nearest the pen first while it hovers over canvas point
/// `pen`, else as `needed` orders them (nearest the screen's middle).
fn nearest(needed: &Needed, pen: Option<[f32; 2]>, map: &TileMap) -> Vec<[u32; 2]> {
    let mut shown: Vec<[u32; 2]> = needed.shown().collect();
    if let Some(pen) = pen {
        let at = pen.map(|value| value * map.scale() as f32);
        shown.sort_by_cached_key(|tile| far(&map.tile_rect(*tile), at));
    }
    shown
}

impl ViewCache {
    /// The snapshots under the next stroke of the tier shown are being taken ahead or wait to be:
    /// frames are wanted until none is.
    #[must_use]
    pub const fn taking_ahead(&self) -> bool {
        self.taking
    }

    /// Bytes the tier shown keeps for strokes (its snapshots and coverage).
    #[must_use]
    pub fn drawing_bytes(&self) -> u64 {
        self.current.as_ref().map_or(0, |tier| tier.drawing.bytes())
    }

    /// Lets the tier shown's slots for strokes go unless a stroke is drawn; their bytes.
    pub fn let_go_drawing(&mut self) -> u64 {
        (self.current.as_mut()).map_or(0, |tier| tier.drawing.let_go())
    }
}

/// Draws `parts` of `tier`'s ready tiles (output pixels) anew, their smudges' residuals as `held`
/// holds them, when their estimate fits what is left of the frame budget; whether it did.
fn draw_in_place(
    core: &Core,
    (tier, parts): (&Tier, &[([u32; 2], PixelRect)]),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    held: Held,
) -> Result<bool, RenderError> {
    let missing = || RenderError::Invalid("the canvas was not evaluated for this frame");
    let atlas = canvas.raster_atlas(core)?;
    let (scene, _, _) = (canvas.scene_of(request.artwork, request.active)).ok_or_else(missing)?;
    let mut plans: Vec<Planned> = Vec::new();
    for (tile, part) in parts {
        let slot = tier.map.slot(*tile).ok_or_else(missing)?;
        let at = tier.map.to_target(*tile, slot, part).min;
        let (plan, output) = plan((scene, &held.bases), (tier.scale(), *part), atlas)?;
        plans.push((plan, output, at.map(|v| v.max(0) as u32)));
    }
    let cost: f64 = (plans.iter())
        .map(|(plan, _, _)| plan.counts.estimate(plan.size))
        .sum();
    if !core.pace().fits_raster(cost) {
        return Ok(false);
    }
    let into = (tier.atlas_view(), tier.atlas().format(), paper(request));
    canvas.draw_planned(core, &plans, held, into)?;
    Ok(true)
}

/// The residuals the raster core's draws near the canvas pixels `near` read, made present in the
/// arena and named with the buffers they lie in (`smudge::Held`); `None` while one is still
/// being evaluated (a share each frame): the tiles reading them wait.
///
/// # Errors
/// As the evaluation's.
pub(crate) fn residuals_near(
    core: &Core,
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    near: &[PixelRect],
) -> Result<Option<Held>, RenderError> {
    if !canvas.residuals_ready(core, request, near)? {
        return Ok(None);
    }
    canvas.reach(core, request.artwork, near)?;
    Ok(canvas.held())
}
