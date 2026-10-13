//! A view tier's work in one frame (S32): evaluating the canvas's changes in place or making
//! the tiles they reach wait, filling missing tiles within the frame budget (S53), and logging
//! how long a fill took.

use std::time::Instant;

use crate::core::Core;
use crate::cost::tile_cost;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::plan::Plan;
use crate::scene::Fresh;
use crate::view_atlas::Tier;
use crate::view_needed::Needed;
use crate::view_tier::ViewWork;
/// Output pixels of changes to ready tiles evaluated in place in one frame at most; a larger
/// change makes the tiles it touches wait to be evaluated whole.
const EDIT_PIXELS: i64 = 4 * 256 * 256;

/// The ready tiles of `tier` the canvas pixels `changed` reach, each with the output pixels
/// reached.
pub(crate) fn touched(tier: &Tier, changed: &[PixelRect]) -> Vec<([u32; 2], PixelRect)> {
    let mut parts = Vec::new();
    for rect in changed {
        let placed = tier.map.place(rect);
        for (tile, part) in tier.map.meeting(&placed) {
            if tier.is_ready(tile) && !part.is_empty() {
                parts.push((tile, part));
            }
        }
    }
    parts
}

/// Evaluates the pixels of ready tiles that `changed` (canvas pixels) reach again, the
/// in-progress stroke from its first sample on, when they are few and their estimate fits what
/// is left of the frame budget (S53); else the tiles they touch wait to be filled whole as
/// background work, the canvas shown there meanwhile: so a stroke drawn over a dense place at a
/// tier shows on the enlarged canvas instead of holding its frames up.
pub(crate) fn in_place(
    core: &Core,
    tier: &mut Tier,
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    (changed, work): (&[PixelRect], &mut ViewWork),
) -> Result<(), RenderError> {
    let parts = touched(tier, changed);
    if parts.is_empty() {
        return Ok(());
    }
    let area: i64 = parts
        .iter()
        .map(|(_, part)| {
            i64::from(part.max[0] - part.min[0]) * i64::from(part.max[1] - part.min[1])
        })
        .sum();
    let mut tiles: Vec<[u32; 2]> = parts.iter().map(|(tile, _)| *tile).collect();
    tiles.sort_unstable();
    tiles.dedup();
    let regions: Vec<PixelRect> = (parts.iter())
        .filter_map(|(tile, part)| {
            let slot = tier.map.slot(*tile)?;
            Some(tier.map.to_target(*tile, slot, part))
        })
        .collect();
    let missing = || RenderError::Invalid("the canvas was not evaluated for this frame");
    let (scene, _) = canvas
        .scene_of(request.artwork, request.active)
        .ok_or_else(missing)?;
    let (_, costs) = canvas
        .scene_costs(request.artwork, request.active)
        .ok_or_else(missing)?;
    let fresh = match request.active.filter(|_| scene.has_active()) {
        Some(active) => Fresh::new(active, 0, scene.active_clip),
        None => Fresh::default(),
    };
    let plan = (area <= EDIT_PIXELS).then(|| tier.plan(scene, &fresh, &regions));
    let fits = |plan: &Plan| {
        let each = plan.tiles.iter().map(|tile| tile_cost(plan, tile, costs));
        let (sum, most) = each.fold((0.0, 0.0), |(sum, most), cost| (sum + cost, cost.max(most)));
        core.pace().fits(sum, most)
    };
    if let Some(plan) = plan.filter(fits) {
        core.evaluate.tally.binned(plan.indices.len());
        tier.dispatch(core, (canvas, request), (&plan, &plan.tiles), &fresh, None)?;
        work.updated = tiles;
        return Ok(());
    }
    for tile in &tiles {
        tier.set_ready(*tile, false);
    }
    work.dropped = tiles;
    Ok(())
}

/// Drops tiles beyond the keep ring.
pub(crate) fn release_far(tier: &mut Tier, needed: &Needed) {
    let held: Vec<[u32; 2]> = tier.map.held().map(|(tile, _)| tile).collect();
    for tile in &held {
        if !needed.keeps(*tile) {
            tier.release(*tile);
        }
    }
}

/// Needed tiles being evaluated whole, from the first frame that missed one until none is
/// missing: logged then, as the time a pan or a new tier took to fill in.
pub(crate) struct Filling {
    started: Instant,
    frames: u32,
    tiles: usize,
}

impl Filling {
    pub(crate) fn step(filling: Option<Self>, work: &ViewWork) -> Option<Self> {
        let mut filling = match filling {
            Some(filling) => filling,
            None if !work.filled.is_empty() => Self {
                started: Instant::now(),
                frames: 0,
                tiles: 0,
            },
            None => return None,
        };
        filling.frames += 1;
        filling.tiles += work.filled.len();
        if work.waiting > 0 {
            return Some(filling);
        }
        // A pan that one frame fills is no news; longer fills are what a pan or pinch costs.
        let level = if filling.frames > 1 {
            log::Level::Info
        } else {
            log::Level::Debug
        };
        log::log!(
            level,
            "view tier {}x filled {} tiles in {} ms over {} frames",
            work.tier.unwrap_or(0),
            filling.tiles,
            filling.started.elapsed().as_millis(),
            filling.frames
        );
        None
    }
}
