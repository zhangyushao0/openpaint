//! A view tier's missing tiles filled in as background work (S53, DECISIONS D80): S32 evaluated
//! up to four whole view tiles a frame whatever they held, so zoomed in over a dense place on the
//! tablet each frame took a second. Now the tile being filled goes on where the last frame left
//! it; the next missing one, nearest the screen's middle first, gets a slot; each is binned a
//! block at a time (kept across frames while the canvas's scene stays) and each frame evaluates
//! the plan tiles the frame budget allows. A view tile is ready, and shows, once all of it is.
//! Nothing is filled while a stroke is drawn: its frames are the stroke's.

use std::time::{Duration, Instant};

use crate::core::Core;
use crate::cost::tile_cost;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::protocol::TileRecord;
use crate::scene::Fresh;
use crate::view_atlas::{Tier, TileFill};
use crate::view_needed::Needed;
use crate::view_tier::ViewWork;

/// Blocks one frame bins or goes on evaluating at most (cheap ones: the paper, few strokes), and
/// the CPU time it bins for at most before it starts another.
const FILL_BLOCKS: usize = 64;
const BINNING: Duration = Duration::from_millis(2);
/// A view tile is binned in blocks of this many output pixels a side: 64 plan tiles each.
const BLOCK: i32 = 128;

/// One frame's share of the missing tiles `needed` that `tier` lacks.
///
/// # Errors
/// As the evaluator's dispatch.
pub(crate) fn fill(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    work: &mut ViewWork,
) -> Result<(), RenderError> {
    let Some((serial, _)) = canvas.scene_costs(request.artwork, request.active) else {
        return Ok(());
    };
    if request.active.is_some() {
        return Ok(());
    }
    let started = Instant::now();
    for _ in 0..FILL_BLOCKS {
        if core.pace().spent() {
            break;
        }
        if tier.fill.as_ref().is_some_and(|fill| fill.scene != serial) {
            tier.fill = None;
        }
        if tier.fill.is_none() {
            let Some(tile) = next_missing(core, tier, needed)? else {
                break;
            };
            tier.fill = Some(begin(tier, tile, serial));
        }
        let Some(mut fill) = tier.fill.take() else {
            break;
        };
        if fill.left.is_empty() {
            if started.elapsed() > BINNING {
                tier.fill = Some(fill);
                break;
            }
            bin_block(core, tier, &mut fill, (canvas, request))?;
        }
        // S54: a block reading a residual the arena lacks waits while it is evaluated.
        let block: Vec<TileRecord> = fill.left.iter().map(|(tile, _)| *tile).collect();
        if !canvas.residuals_ready(core, request, (&block, tier.scale()))? {
            tier.fill = Some(fill);
            break;
        }
        let taken = take(core, &mut fill);
        if !taken.is_empty() {
            dispatch(core, (tier, &fill), (canvas, request), &taken)?;
        }
        if fill.left.is_empty() && fill.blocks.is_empty() {
            tier.set_ready(fill.tile, true);
            work.filled.push(fill.tile);
        } else {
            tier.fill = Some(fill);
            if taken.is_empty() {
                break;
            }
        }
    }
    core.evaluate.tally.filled(work.filled.len());
    Ok(())
}

/// The first needed tile that is not ready, given a slot (a spare one let go, or the atlas
/// grown); `None` when every needed tile is ready or no slot is left.
fn next_missing(
    core: &Core,
    tier: &mut Tier,
    needed: &Needed,
) -> Result<Option<[u32; 2]>, RenderError> {
    let missing = needed
        .ordered()
        .filter(|tile| tier.map.slot(*tile).is_none())
        .count() as u32;
    for tile in needed.ordered() {
        if tier.is_ready(tile) {
            continue;
        }
        if tier.map.slot(tile).is_none() {
            let spare = (tier.map.held())
                .map(|(held, _)| held)
                .find(|held| !needed.needs(*held));
            if let (None, Some(spare)) = (tier.map.free(), spare) {
                tier.release(spare);
            }
            if !tier.hold(core, tile, missing)? {
                continue;
            }
        }
        return Ok(Some(tile));
    }
    Ok(None)
}

/// `tile`'s fill begun: its slot's pixels cut into blocks, none binned yet.
fn begin(tier: &Tier, tile: [u32; 2], scene: u64) -> TileFill {
    let slot = tier.map.slot(tile).unwrap_or(0);
    let rect = tier.map.to_target(tile, slot, &tier.map.tile_rect(tile));
    let mut blocks = Vec::new();
    for y in (rect.min[1]..rect.max[1]).step_by(BLOCK as usize) {
        for x in (rect.min[0]..rect.max[0]).step_by(BLOCK as usize) {
            blocks.push(PixelRect {
                min: [x, y],
                max: [(x + BLOCK).min(rect.max[0]), (y + BLOCK).min(rect.max[1])],
            });
        }
    }
    // Taken from the end: the first block first.
    blocks.reverse();
    TileFill {
        tile,
        scene,
        blocks,
        plan: crate::plan::Plan::default(),
        left: std::collections::VecDeque::new(),
    }
}

/// The next block of `fill` binned from the canvas's scene, each plan tile estimated.
fn bin_block(
    core: &Core,
    tier: &Tier,
    fill: &mut TileFill,
    (canvas, request): (&CanvasTarget, &FrameRequest<'_>),
) -> Result<(), RenderError> {
    let Some(block) = fill.blocks.pop() else {
        return Ok(());
    };
    let missing = || RenderError::Invalid("the canvas was not evaluated for this frame");
    let (scene, _) = canvas
        .scene_of(request.artwork, request.active)
        .ok_or_else(missing)?;
    let (_, costs) = canvas
        .scene_costs(request.artwork, request.active)
        .ok_or_else(missing)?;
    let plan = tier.plan(scene, &Fresh::default(), &[block]);
    core.evaluate.tally.binned(plan.indices.len());
    fill.left = (plan.tiles.iter())
        .map(|record| (*record, tile_cost(&plan, record, costs)))
        .collect();
    fill.plan = plan;
    Ok(())
}

/// The plan tiles of `fill` the frame budget takes, in order, with their estimates.
fn take(core: &Core, fill: &mut TileFill) -> Vec<(TileRecord, f64)> {
    let mut pace = core.pace();
    let mut taken = Vec::new();
    while let Some((record, cost)) = fill.left.front().copied() {
        if !pace.take(cost) {
            break;
        }
        fill.left.pop_front();
        taken.push((record, cost));
    }
    taken
}

/// Evaluates `taken` plan tiles of `fill`'s block into `tier`, timed.
fn dispatch(
    core: &Core,
    (tier, fill): (&mut Tier, &TileFill),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    taken: &[(TileRecord, f64)],
) -> Result<(), RenderError> {
    let tiles: Vec<TileRecord> = taken.iter().map(|(tile, _)| *tile).collect();
    let mut pace = core.pace();
    let estimate = pace.close();
    let timing = pace.time(estimate);
    let clocked = timing.and_then(|timing| Some((pace.clock()?, timing)));
    let plan = (&fill.plan, tiles.as_slice());
    let dispatched = tier.dispatch(core, (canvas, request), plan, &Fresh::default(), clocked);
    if let Some(timing) = timing {
        pace.finish(&core.gpu, timing);
    }
    dispatched
}
