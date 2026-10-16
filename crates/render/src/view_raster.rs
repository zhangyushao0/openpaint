//! A view tier's tiles drawn by the raster core (S59, DECISIONS D86): when the canvas's scene goes
//! to it (`raster_route.rs`, the routing logged when it changes), a missing tile is drawn whole
//! in one region of 256 pixels a side into its slot, as background work within the frame budget
//! (the raster core's estimate corrected by its own GPU readings, `pace.rs`), the screen's middle
//! first; a change to ready tiles is drawn in place when it fits what is left of the frame, else
//! the tiles wait to be drawn again. While a stroke is drawn the evaluator updates the tiles it
//! reaches as before (the stroke in progress stays on the evaluator until S61); those tiles, and
//! any the evaluator drew before the scene came to the raster core, are drawn again afterwards,
//! shown meanwhile, so every tile of such a scene ends as the raster core draws it, and a tier
//! equals the export at its scale bit for bit. The targets are the budget's (`Unit::Raster`).

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::pace::Group;
use crate::program::SceneOps;
use crate::raster::{ATLAS, Finish, RasterTargets};
use crate::raster_plan::RasterPlan;
use crate::raster_route::{RasterScene, Routed};
use crate::scene::Scene;
use crate::view_atlas::Tier;
use crate::view_background::next_missing;
use crate::view_fill::{EDIT_PIXELS, touched};
use crate::view_needed::Needed;
use crate::view_tier::ViewWork;
use crate::view_tiles::TIER_TILE;

/// The raster core's side of the view tiers: its targets, the routing of the canvas's last
/// scene, and when it last drew (the budget lets the targets go by recency).
#[derive(Default)]
pub(crate) struct RasterTier {
    targets: Option<RasterTargets>,
    /// The canvas scene's serial and its routing.
    scene: Option<(u64, Result<RasterScene, Routed>)>,
    /// The routing logged last: `None` until one is.
    logged: Option<Result<(), Routed>>,
    pub(crate) used_at: f64,
}

impl RasterTier {
    /// The canvas's scene for `request` goes to the raster core (decided once per scene, the
    /// routing logged when it changes).
    pub(crate) fn routes(&mut self, canvas: &CanvasTarget, request: &FrameRequest<'_>) -> bool {
        let Some((scene, _, serial)) = canvas.scene_serial(request.artwork, request.active) else {
            return false;
        };
        if self.scene.as_ref().is_none_or(|(held, _)| *held != serial) {
            let routed = RasterScene::of(scene);
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

    /// Bytes its targets hold.
    pub(crate) fn bytes(&self) -> u64 {
        self.targets.as_ref().map_or(0, RasterTargets::bytes)
    }

    /// Lets its targets go; returns their bytes.
    pub(crate) fn let_go(&mut self) -> u64 {
        let bytes = self.bytes();
        self.targets = None;
        bytes
    }

    /// The plan of output pixels `rect` at `scale` of the routed scene `scene`.
    fn plan(
        &mut self,
        core: &Core,
        scene: &Scene,
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
        if self.targets.is_none() {
            self.targets = Some(RasterTargets::new(core, [TIER_TILE; 2], ATLAS)?);
        }
        let atlas = self
            .targets
            .as_ref()
            .map_or([ATLAS; 2], RasterTargets::atlas);
        Ok((
            RasterPlan::build((scene, raster), (output, size), atlas)?,
            output,
        ))
    }

    /// Draws `plan` at `output` into `tier`'s target pixel `at`; timed as background work of
    /// `cost` ns when `timed`.
    fn draw(
        &mut self,
        core: &Core,
        (tier, at): (&Tier, [u32; 2]),
        (ops, serial, background): (&SceneOps, u64, [f32; 4]),
        (plan, output): (&RasterPlan, Output),
        timed: Option<f64>,
    ) -> Result<(), RenderError> {
        let Some(targets) = self.targets.as_mut() else {
            return Ok(());
        };
        let finish = Finish {
            view: tier.atlas_view(),
            format: tier.atlas().format(),
            at,
        };
        let scene = ((ops, serial), plan);
        let mut pace = core.pace();
        let timing = timed.and_then(|cost| pace.time(Group::raster(cost)));
        let clocked = timing.and_then(|timing| Some((pace.clock()?, timing)));
        let drawn = targets.draw(core, scene, (output, background), &finish, clocked);
        if let Some(timing) = timing {
            pace.finish(&core.gpu, timing);
        }
        drawn?;
        core.evaluate.tally.rastered(1);
        Ok(())
    }
}

/// The paper of `request`, premultiplied and opaque.
fn paper(request: &FrameRequest<'_>) -> [f32; 4] {
    let paper = request.background;
    [paper.r(), paper.g(), paper.b(), 1.0]
}

/// One frame's share of the tiles `needed` that `tier` lacks, or holds as the evaluator drew them,
/// each drawn whole by the raster core while the frame budget takes its estimate. Nothing is drawn
/// while a stroke is.
///
/// # Errors
/// As the raster core.
pub(crate) fn fill_raster(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&CanvasTarget, &FrameRequest<'_>),
    raster: &mut RasterTier,
    (work, now): (&mut ViewWork, f64),
) -> Result<(), RenderError> {
    let Some((scene, ops, serial)) = canvas.scene_serial(request.artwork, request.active) else {
        return Ok(());
    };
    if request.active.is_some() {
        return Ok(());
    }
    // An evaluation of a tile begun before the scene came here is left.
    tier.fill = None;
    let scale = tier.scale();
    while !core.pace().spent() {
        let Some(tile) = next_tile(core, tier, needed)? else {
            break;
        };
        let (plan, output) = raster.plan(core, scene, (scale, tier.map.tile_rect(tile)))?;
        let cost = plan.counts.estimate(plan.size);
        if !core.pace().take_raster(cost) {
            break;
        }
        let slot = tier.map.slot(tile).unwrap_or(0);
        let at = tier.map.slot_origin(slot);
        let drawn = (ops, serial, paper(request));
        raster.draw(core, (tier, at), drawn, (&plan, output), Some(cost))?;
        raster.used_at = now;
        tier.set_ready(tile, true);
        tier.set_evaluated(tile, false);
        work.filled.push(tile);
    }
    core.evaluate.tally.filled(work.filled.len());
    Ok(())
}

/// The next tile to draw: the first needed one not ready (given a slot), else the first needed one
/// the evaluator drew.
fn next_tile(
    core: &Core,
    tier: &mut Tier,
    needed: &Needed,
) -> Result<Option<[u32; 2]>, RenderError> {
    if let Some(tile) = next_missing(core, tier, needed)? {
        return Ok(Some(tile));
    }
    Ok(needed
        .ordered()
        .find(|tile| tier.is_ready(*tile) && tier.is_evaluated(*tile)))
}

/// Draws the pixels of ready tiles that `changed` (canvas pixels) reach again, when they are few
/// and their estimate fits what is left of the frame budget; else the tiles they touch wait to be
/// drawn whole as background work.
///
/// # Errors
/// `Invalid` when the canvas was not evaluated for `request` this frame; as the raster core.
pub(crate) fn in_place_raster(
    core: &Core,
    tier: &mut Tier,
    (canvas, request): (&CanvasTarget, &FrameRequest<'_>),
    raster: &mut RasterTier,
    (changed, work, now): (&[PixelRect], &mut ViewWork, f64),
) -> Result<(), RenderError> {
    let parts = touched(tier, changed);
    if parts.is_empty() {
        return Ok(());
    }
    let missing = || RenderError::Invalid("the canvas was not evaluated for this frame");
    let (scene, ops, serial) =
        (canvas.scene_serial(request.artwork, request.active)).ok_or_else(missing)?;
    let mut tiles: Vec<[u32; 2]> = parts.iter().map(|(tile, _)| *tile).collect();
    tiles.sort_unstable();
    tiles.dedup();
    let area: i64 = (parts.iter())
        .map(|(_, part)| {
            i64::from(part.max[0] - part.min[0]) * i64::from(part.max[1] - part.min[1])
        })
        .sum();
    if area <= EDIT_PIXELS {
        let scale = tier.scale();
        let mut plans = Vec::new();
        for (tile, part) in &parts {
            let slot = tier.map.slot(*tile).ok_or_else(missing)?;
            let at = tier
                .map
                .to_target(*tile, slot, part)
                .min
                .map(|v| v.max(0) as u32);
            plans.push((at, raster.plan(core, scene, (scale, *part))?));
        }
        let cost: f64 = (plans.iter())
            .map(|(_, (plan, _))| plan.counts.estimate(plan.size))
            .sum();
        if core.pace().fits_raster(cost) {
            for (at, (plan, output)) in &plans {
                let drawn = (ops, serial, paper(request));
                raster.draw(core, (tier, *at), drawn, (plan, *output), None)?;
            }
            raster.used_at = now;
            work.updated = tiles;
            return Ok(());
        }
    }
    for tile in &tiles {
        tier.set_ready(*tile, false);
    }
    work.dropped = tiles;
    Ok(())
}
