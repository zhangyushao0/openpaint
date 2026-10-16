//! A view tier's tiles drawn by the raster core on a worker (S60, DECISIONS D87): the frame loop
//! hands a tile over as a `RasterJob` (the canvas's scene shared, the tier's atlas and the slot),
//! and the runtime's raster worker plans, encodes and submits it with a `RasterDrawer`, so a
//! tile filling costs the screen's frames nothing but handing it over. The worker submits a tile
//! once the background work queued on the GPU leaves it room (`fits`), so while nothing touches
//! the screen tiles go to the GPU at its pace, between the frames drawn, the next always queued
//! behind the one running; a tile handed over while nothing touched is dropped unsubmitted once a
//! hand arrives (the worker's `wanted`). Its answer, `RasterDone`, goes back to the view, which
//! shows the tile from the next frame on unless a change reached it meanwhile (it is drawn again
//! then). The worker also makes the raster core's pipelines and its targets as the device is made
//! (`warm`) and the atlas of the next new tier ahead (`atlas`), so neither a zoom nor its first
//! tile allocates on the frame loop's thread.

use std::sync::Arc;
use std::time::Duration;

use crate::core::Core;
use crate::error::RenderError;
use crate::output::Output;
use crate::raster::{ATLAS, Finish, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::raster_route::RasterScene;
use crate::scene::Scene;
use crate::view_tiles::TIER_TILE;

/// How long the worker waits between looks at the GPU's queue for room, and how many looks at
/// most (about two of the tablet's frames) before it submits anyway.
const LOOK: Duration = Duration::from_millis(1);
const PATIENCE: u32 = 16;

/// One view tile for the raster worker: the region of the scene's output at a tier's scale and
/// where in the tier's atlas it goes.
pub struct RasterJob {
    pub(crate) id: u64,
    pub(crate) scene: Arc<Scene>,
    pub(crate) raster: Arc<RasterScene>,
    pub(crate) buffers: SceneBuffers,
    pub(crate) output: Output,
    pub(crate) size: [u32; 2],
    pub(crate) paper: [f32; 4],
    pub(crate) into: wgpu::TextureView,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) at: [u32; 2],
}

/// What the worker did with a job: drew it (submitted, its GPU time estimated at `time` ns by the
/// raster core's corrected table), or dropped it; and the bytes its targets hold now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterDone {
    pub(crate) id: u64,
    pub drawn: bool,
    pub(crate) time: f64,
    pub bytes: u64,
}

/// What the view asks of the raster worker, in order.
pub enum RasterOrder {
    Draw(Box<RasterJob>),
    /// Let the targets go (the GPU memory budget's step); the next job makes them again.
    LetGo,
    /// Make the atlas of the next new tier, of this many slots (S60: making one allocates tens of
    /// MB, 20 to 36 ms on the tablet).
    Atlas(u32),
}

/// What the raster worker answers: a tile drawn or dropped, or an atlas made.
pub enum RasterAnswer {
    Tile(RasterDone),
    Atlas(wgpu::Texture),
}

/// The raster worker's side: its targets, kept between tiles.
#[derive(Default)]
pub struct RasterDrawer {
    targets: Option<RasterTargets>,
}

impl RasterDrawer {
    /// Bytes its targets hold.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.targets.as_ref().map_or(0, RasterTargets::bytes)
    }

    /// Lets its targets go.
    pub fn let_go(&mut self) {
        self.targets = None;
    }

    /// Makes the raster core's pipelines and its targets now, unless they are made (S60: as the
    /// device is made, so neither the first zoom nor its first tile waits for them, nor the
    /// frame loop for the device's allocator while they are made).
    ///
    /// # Errors
    /// As the GPU: a shader or a size the device refuses.
    pub fn warm(&mut self, core: &Core) -> Result<(), RenderError> {
        core.make_raster_pipelines()?;
        if self.targets.is_none() {
            self.targets = Some(RasterTargets::new(core, [TIER_TILE; 2], ATLAS)?);
        }
        Ok(())
    }

    /// Plans and encodes `job`, then submits it once the background work queued on the GPU leaves
    /// it room (`ahead`: handed over while nothing touched the screen, else while a hand does;
    /// `room`), dropped unsubmitted as soon as `wanted` says it is no longer. A job the raster
    /// core fails is logged and answered undrawn (the device's loss is the frame loop's to find).
    pub fn draw(
        &mut self,
        core: &Core,
        (job, ahead): (RasterJob, bool),
        wanted: impl Fn() -> bool,
    ) -> RasterDone {
        let id = job.id;
        self.try_draw(core, (job, ahead), wanted)
            .unwrap_or_else(|error| {
                log::error!("the raster worker could not draw a tile: {error}");
                self.answer(id, (false, 0.0))
            })
    }

    fn try_draw(
        &mut self,
        core: &Core,
        (job, ahead): (RasterJob, bool),
        wanted: impl Fn() -> bool,
    ) -> Result<RasterDone, RenderError> {
        let id = job.id;
        if !wanted() {
            return Ok(self.answer(id, (false, 0.0)));
        }
        if self.targets.is_none() {
            self.targets = Some(RasterTargets::new(core, [TIER_TILE; 2], ATLAS)?);
        }
        let Some(targets) = self.targets.as_mut() else {
            return Ok(self.answer(id, (false, 0.0)));
        };
        let scene = (&*job.scene, &*job.raster);
        let plan = RasterPlan::build(scene, (job.output, job.size), targets.atlas())?;
        // The scene is not needed past the plan: a new one can take this one's spans (S56).
        drop(job.scene);
        let (time, stamps) = core.pace().raster_start(plan.counts.estimate(plan.size));
        let finish = Finish {
            view: &job.into,
            format: job.format,
            at: job.at,
        };
        let drawn = (&job.buffers, &plan);
        let place = (job.output, job.paper);
        let prepared = targets.prepare(core, drawn, place, &finish, stamps.as_ref());
        let submitted = prepared.map(|(buffer, _)| submit(core, (buffer, time), ahead, &wanted));
        if let Some(stamps) = &stamps {
            let drawn = submitted.as_ref().is_ok_and(|drawn| *drawn);
            core.pace().raster_finish(&core.gpu, stamps, drawn);
        }
        let drawn = submitted?;
        Ok(self.answer(id, (drawn, time)))
    }

    /// The answer to job `id`: drawn, its GPU time estimated at `time` ns, or not.
    fn answer(&self, id: u64, (drawn, time): (bool, f64)) -> RasterDone {
        RasterDone {
            id,
            drawn,
            time,
            bytes: self.bytes(),
        }
    }

    /// An atlas of `slots` view tiles, made now; `None` when the device refuses it (logged).
    #[must_use]
    pub fn atlas(core: &Core, slots: u32) -> Option<wgpu::Texture> {
        crate::view_atlas::atlas_texture(core, crate::view_atlas::atlas_for(slots))
            .inspect_err(|error| log::error!("no atlas made ahead for the next tier: {error}"))
            .ok()
    }

    /// Serves every order `view` has, at once, on this thread: what the runtime's worker does,
    /// for tests that hold a view without one.
    pub fn serve(&mut self, core: &Core, view: &mut crate::view_tier::ViewCache) {
        for order in view.take_orders() {
            match order {
                RasterOrder::Draw(job) => {
                    let ahead = core.pace().idle();
                    let done = self.draw(core, (*job, ahead), || true);
                    view.answer(RasterAnswer::Tile(done));
                }
                RasterOrder::LetGo => self.let_go(),
                RasterOrder::Atlas(slots) => {
                    if let Some(atlas) = Self::atlas(core, slots) {
                        view.answer(RasterAnswer::Atlas(atlas));
                    }
                }
            }
        }
    }
}

/// Submits `buffer`, a tile of `time` ns of GPU time, once the GPU's queue of background work has
/// room for it (`ahead`: handed over while nothing touched the screen, else while a hand does);
/// returns whether it did (not when `wanted` said no first).
fn submit(
    core: &Core,
    (buffer, time): (wgpu::CommandBuffer, f64),
    ahead: bool,
    wanted: &impl Fn() -> bool,
) -> bool {
    // At most `PATIENCE` looks: work another thread keeps queuing never starves the tiles.
    for _ in 0..PATIENCE {
        if room(core, time, ahead) {
            break;
        }
        if !wanted() {
            return false;
        }
        std::thread::sleep(LOOK);
    }
    core.gpu.submit([buffer]);
    core.background_submitted(time);
    core.evaluate.tally.rastered(1);
    true
}

/// Whether the GPU's queue of background work has room for `time` ns more (`fits`): while
/// nothing touches the screen (`ahead`) behind one piece or within the idle most; while a hand
/// does, alone or within a touched frame's share.
fn room(core: &Core, time: f64, ahead: bool) -> bool {
    let (most, behind) = if ahead {
        (core.pace().most_queued(), 1)
    } else {
        (core.pace().share(), 0)
    };
    let queued = core.background_queued();
    fits((queued, core.background_pieces()), time, (most, behind))
}

/// Background work of `time` ns joins `queued` ns in `pieces` on the GPU's queue: when at most
/// `behind` pieces are there (the one the GPU runs, which this one waits behind, whatever either
/// costs), or when all of it keeps within `most`. While nothing touches the screen the GPU so has
/// the next tile when one ends, and a frame submitted when a hand arrives waits for at most the
/// piece running and one more, or for `most` of smaller ones; while a hand touches, a frame waits
/// for at most one tile.
pub(crate) fn fits((queued, pieces): (f64, u64), time: f64, (most, behind): (f64, u64)) -> bool {
    pieces <= behind || queued + time <= most
}

#[cfg(test)]
mod tests {
    use super::fits;

    #[test]
    fn a_tile_waits_behind_one_piece_or_joins_the_queue_within_its_most() {
        let ms = 1_000_000.0;
        let idle = (10.0 * ms, 1);
        assert!(fits((0.0, 0), 30.0 * ms, idle), "alone, whatever it costs");
        assert!(
            fits((25.0 * ms, 1), 30.0 * ms, idle),
            "behind the one the GPU runs"
        );
        assert!(
            !fits((12.0 * ms, 2), 1.0 * ms, idle),
            "two pieces queued past the most"
        );
        assert!(
            fits((4.0 * ms, 3), 6.0 * ms, idle),
            "small ones within the most"
        );
        assert!(!fits((4.5 * ms, 3), 6.0 * ms, idle), "beyond it: waits");
        let touched = (4.0 * ms, 0);
        assert!(
            fits((0.0, 0), 9.0 * ms, touched),
            "alone while a hand touches"
        );
        assert!(
            !fits((6.0 * ms, 1), 6.0 * ms, touched),
            "never behind another then"
        );
        assert!(
            fits((1.0 * ms, 2), 2.0 * ms, touched),
            "small ones within the share"
        );
    }
}
