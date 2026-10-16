//! A view tier's tiles drawn by the raster core (S59, DECISIONS D86): when the canvas's scene goes
//! to it (`raster_route.rs`, the routing logged when it changes), a missing tile is drawn whole
//! in one region of 256 pixels a side into its slot, as background work, the screen's middle
//! first; a change to ready tiles is drawn in place when it fits what is left of the frame, else
//! the tiles wait to be drawn again. While a stroke is drawn the evaluator updates the tiles it
//! reaches as before (the stroke in progress stays on the evaluator until S62); those tiles, and
//! any the evaluator drew before the scene came to the raster core, are drawn again afterwards,
//! shown meanwhile, so every tile of such a scene ends as the raster core draws it, and a tier
//! equals the export at its scale bit for bit. S60 (D87): the background tiles are planned,
//! encoded and submitted by the runtime's raster worker (`view_raster_jobs.rs`); this side hands
//! them over (`view_raster_fill.rs`) and draws the changes in place with targets of its own. The
//! targets are the budget's (`Unit::Raster`): these and the worker's. S61 (D88): the smudge
//! residuals a tile reads are the canvas's, made present before it is drawn or handed over and
//! named with the arena's buffer as it is then (`smudge::Held`); a tile reading one still being
//! evaluated waits.

use std::sync::Arc;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::{ATLAS, Finish, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::raster_route::{RasterScene, Routed};
use crate::scene::Scene;
use crate::smudge::{Held, around_output};
use crate::view_atlas::Tier;
use crate::view_fill::{EDIT_PIXELS, touched};
use crate::view_raster_jobs::{RasterDone, RasterOrder};
use crate::view_tier::ViewWork;

/// The raster core's side of the view tiers: its targets for changes drawn in place, the routing
/// of the canvas's last scene, when it last drew (the budget lets the targets go by recency), and
/// the tiles handed to the raster worker (S60).
#[derive(Default)]
pub(crate) struct RasterTier {
    targets: Option<RasterTargets>,
    /// The canvas scene's serial and its routing.
    pub(crate) scene: Option<(u64, Result<Arc<RasterScene>, Routed>)>,
    /// The routing logged last: `None` until one is.
    logged: Option<Result<(), Routed>>,
    pub(crate) used_at: f64,
    /// Tiles handed to the worker and not answered yet, oldest first.
    pub(crate) issued: Vec<Issued>,
    /// Orders for the worker not taken yet, and its answers not applied yet.
    pub(crate) orders: Vec<RasterOrder>,
    pub(crate) answers: Vec<RasterDone>,
    /// The worker's targets' bytes, and the last tile's GPU time (ns), as it said last.
    pub(crate) drawer: u64,
    pub(crate) last: f64,
    /// The next job's id.
    pub(crate) next: u64,
}

/// A tile handed to the raster worker (S60): its job, its tier's scale, the tile, the slot and the
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

impl RasterTier {
    /// The canvas's scene for `request` goes to the raster core (decided once per scene, the
    /// routing logged when it changes).
    pub(crate) fn routes(&mut self, canvas: &CanvasTarget, request: &FrameRequest<'_>) -> bool {
        let Some((scene, _, serial)) = canvas.scene_serial(request.artwork, request.active) else {
            return false;
        };
        if self.scene.as_ref().is_none_or(|(held, _)| *held != serial) {
            let routed = RasterScene::of(scene).map(Arc::new);
            let routing = routed.as_ref().map(drop).map_err(|routed| *routed);
            if self.logged != Some(routing) {
                match routing {
                    Ok(()) => log::info!("view tiers drawn by the raster core"),
                    Err(routed) => log::info!("view tiers evaluated: the artwork holds {routed}"),
                }
                self.logged = Some(routing);
            }
            self.scene = Some((serial, routed));
        }
        matches!(self.scene, Some((_, Ok(_))))
    }

    /// Bytes its targets and the worker's hold.
    pub(crate) fn bytes(&self) -> u64 {
        self.targets.as_ref().map_or(0, RasterTargets::bytes) + self.drawer
    }

    /// Lets its targets go, and the worker's (an order); returns their bytes.
    pub(crate) fn let_go(&mut self) -> u64 {
        let bytes = self.bytes();
        self.targets = None;
        if self.drawer > 0 {
            self.orders.push(RasterOrder::LetGo);
            self.drawer = 0;
        }
        bytes
    }

    /// The plan of output pixels `rect` at `scale` of the routed scene `scene`, its smudges'
    /// residuals at `bases`.
    fn plan(
        &self,
        (scene, bases): (&Scene, &[Option<u32>]),
        (scale, rect): (u32, PixelRect),
    ) -> Result<(RasterPlan, Output), RenderError> {
        let Some((_, Ok(raster))) = &self.scene else {
            return Err(RenderError::Invalid(
                "the scene does not go to the raster core",
            ));
        };
        let origin = rect.min.map(|at| at.max(0) as u32);
        let output = Output::at(scale, origin)?;
        let size = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]).max(0) as u32);
        Ok((
            RasterPlan::build((scene, raster, bases), (output, size), [ATLAS; 2])?,
            output,
        ))
    }

    /// Draws `plan` at `output` into `tier`'s target pixel `at`, on the frame loop's thread.
    fn draw(
        &mut self,
        core: &Core,
        (tier, at): (&Tier, [u32; 2]),
        (scene, background): (&SceneBuffers, [f32; 4]),
        (plan, output): (&RasterPlan, Output),
    ) -> Result<(), RenderError> {
        if self.targets.is_none() {
            self.targets = Some(RasterTargets::new(
                core,
                [crate::view_tiles::TIER_TILE; 2],
                ATLAS,
            )?);
        }
        let Some(targets) = self.targets.as_mut() else {
            return Ok(());
        };
        let finish = Finish {
            view: tier.atlas_view(),
            format: tier.atlas().format(),
            at,
        };
        targets.draw(core, (scene, plan), (output, background), &finish, None)?;
        core.evaluate.tally.rastered(1);
        Ok(())
    }
}

/// The paper of `request`, premultiplied and opaque.
pub(crate) fn paper(request: &FrameRequest<'_>) -> [f32; 4] {
    let paper = request.background;
    [paper.r(), paper.g(), paper.b(), 1.0]
}

/// Draws the pixels of ready tiles that `changed` (canvas pixels) reach again, when they are few,
/// the residuals they read are at hand and their estimate fits what is left of the frame budget;
/// else (or while the raster core's pipelines are still being made, S60) the tiles they touch
/// wait to be drawn whole as background work.
///
/// # Errors
/// `Invalid` when the canvas was not evaluated for `request` this frame; as the raster core.
pub(crate) fn in_place_raster(
    core: &Core,
    tier: &mut Tier,
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    raster: &mut RasterTier,
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
        && draw_in_place(core, (tier, &parts), (canvas, request), raster, held)?
    {
        raster.used_at = now;
        work.updated = tiles;
        return Ok(());
    }
    for tile in &tiles {
        tier.set_ready(*tile, false);
    }
    work.dropped = tiles;
    Ok(())
}

/// Draws `parts` of `tier`'s ready tiles (output pixels) anew, their smudges' residuals as `held`
/// holds them, when their estimate fits what is left of the frame budget; whether it did.
fn draw_in_place(
    core: &Core,
    (tier, parts): (&Tier, &[([u32; 2], PixelRect)]),
    (canvas, request): (&CanvasTarget, &FrameRequest<'_>),
    raster: &mut RasterTier,
    held: Held,
) -> Result<bool, RenderError> {
    let missing = || RenderError::Invalid("the canvas was not evaluated for this frame");
    let (scene, ops, serial) =
        (canvas.scene_serial(request.artwork, request.active)).ok_or_else(missing)?;
    let mut plans = Vec::new();
    for (tile, part) in parts {
        let slot = tier.map.slot(*tile).ok_or_else(missing)?;
        let at = tier.map.to_target(*tile, slot, part).min;
        let planned = raster.plan((scene, &held.bases), (tier.scale(), *part))?;
        plans.push((at.map(|v| v.max(0) as u32), planned));
    }
    let cost: f64 = (plans.iter())
        .map(|(_, (plan, _))| plan.counts.estimate(plan.size))
        .sum();
    if !core.pace().fits_raster(cost) {
        return Ok(false);
    }
    let buffers = SceneBuffers::of(ops, serial, held);
    for (at, (plan, output)) in &plans {
        let drawn = (&buffers, paper(request));
        raster.draw(core, (tier, *at), drawn, (plan, *output))?;
    }
    Ok(true)
}

/// The residuals the raster core's draws near the canvas pixels `near` read, made present in the
/// arena and named with the buffers they lie in (`smudge::Held`); `None` while one is still
/// being evaluated (a share each frame): the tiles reading them wait (S61).
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
