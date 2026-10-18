//! A canvas target's scene: built and uploaded once per artwork version, changed in place when a
//! version only gives a previewed adjustment new values (S35, `preview.rs`), with the plan it
//! was last evaluated by and the previewed layer's snapshot kept as long as both stay valid.

use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::affine::Affine;
use crate::bounded::{frame_share, paper_only, tile_rects};
use crate::core::Core;
use crate::cost::item_costs;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::preview::{PreviewSnapshot, retuned};
use crate::program::{Dispatch, SceneOps, SnapshotUse, scene_ops};
use crate::protocol::{Op, TileRecord};
use crate::raster_route::{Routed, Routing};
use crate::scene::{ActivePlace, Fresh, Scene};
use crate::scene_adjust::EditPlace;
use crate::smudge::{Smudges, Wanted, reached};
use crate::waiting::Waiting;

/// One artwork version's scene and its ops on the GPU, reused while the version stays.
pub(super) struct SceneCache {
    pub(super) artwork: Artwork,
    /// The in-progress stroke's place, or while none is drawn the next one's (S56).
    pub(super) active: ActivePlace,
    /// Shared with the raster worker drawing a view tier's tiles from it (S60).
    pub(super) scene: Arc<Scene>,
    pub(super) ops: SceneOps,
    /// The adjustment whose values the last version changed, and its layer's snapshot.
    previewed: Option<EditPlace>,
    /// How far its last two values change pixels beyond its layer's content, in tiles.
    spread: u32,
    /// The kept plan's tiles near the layer's content at a spread, and that plan's serial.
    near: Option<(u32, u64, Vec<TileRecord>)>,
    snapshot: Option<PreviewSnapshot>,
    /// The plan of the last evaluation without an in-progress stroke, and what it binned.
    plan: Option<(PlanKey, Plan)>,
    /// What each item costs at an output pixel (S53, `cost.rs`).
    pub(super) costs: Vec<f32>,
    /// Names this scene among every scene built or changed in place: what a view tile's fill
    /// was binned from (S53).
    pub(super) serial: u64,
    /// Waiting tiles binned for the frames' background work, not all evaluated yet (S53).
    pub(super) batch: Option<Batch>,
}

/// Waiting tiles binned together (S53): their plan, and those not evaluated yet with what each
/// is estimated to cost, in the order they are taken.
pub(super) struct Batch {
    pub(super) plan: Plan,
    pub(super) left: std::collections::VecDeque<(TileRecord, f64)>,
}

/// What a kept plan was binned for.
#[derive(PartialEq)]
struct PlanKey {
    regions: Vec<PixelRect>,
    output: Output,
    resume: Option<Range<usize>>,
}

/// What one evaluation into a target cost (S48): the time taken to bin the plan on the CPU, the
/// tiles it evaluated and the op entries their lists held, which the GPU walks per pixel (S63:
/// only a scene routed to the evaluator is evaluated so).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EvaluationWork {
    pub(crate) planned: Duration,
    pub(crate) tiles: usize,
    pub(crate) entries: usize,
}

impl EvaluationWork {
    /// What evaluating `tiles` costs, planned in `planned`.
    pub(super) fn of(tiles: &[TileRecord], planned: Duration) -> Self {
        Self {
            planned,
            tiles: tiles.len(),
            entries: tiles.iter().map(|t| t.count as usize).sum(),
        }
    }
}

/// How one evaluation reads the scene: its plan, its fresh ops, and the snapshot it resumes
/// the previewed layer from.
pub(super) struct Reading<'a> {
    pub(super) ops: &'a SceneOps,
    pub(super) plan: &'a Plan,
    pub(super) fresh: Vec<Op>,
    pub(super) snapshot: Option<SnapshotUse<'a>>,
    /// The plan's tiles near the previewed layer's content, when it resumes from a snapshot.
    pub(super) only: Option<&'a [TileRecord]>,
}

impl CanvasTarget {
    /// The scene of `artwork` with the in-progress stroke's place `active`, built and uploaded
    /// only when either differs from the last call (changed in place when only a previewed
    /// adjustment's values did), with every smudge and filter it draws prepared.
    pub(super) fn scene(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        active: ActivePlace,
    ) -> Result<&SceneCache, RenderError> {
        let same =
            |cache: &SceneCache| cache.artwork.same_version(artwork) && cache.active == active;
        if self.scene.as_ref().is_some_and(same) {
            return self.cached();
        }
        // A preview's values change between versions while no stroke is drawn (in place: while
        // no raster worker holds the scene).
        let retune = self
            .scene
            .as_ref()
            .filter(|cache| cache.active == active && Arc::strong_count(&cache.scene) == 1)
            .and_then(|cache| retuned(&cache.artwork, artwork));
        if let Some(retune) = retune {
            if let Err(error) = self.retune(core, artwork, &retune) {
                self.scene = None;
                return Err(error);
            }
            return self.cached();
        }
        let (scene, ops) = match self.built(core, artwork, &active) {
            Ok(built) => built,
            Err(error) => {
                self.scene = None;
                return Err(error);
            }
        };
        self.scene = Some(SceneCache {
            artwork: artwork.clone(),
            active,
            costs: item_costs(&scene),
            serial: next_serial(),
            scene: Arc::new(scene),
            ops,
            previewed: None,
            spread: 0,
            near: None,
            snapshot: None,
            plan: None,
            batch: None,
        });
        self.cached()
    }

    /// A new scene of `artwork` with the place `active` on the GPU, its smudges and filters
    /// prepared. The strokes' spans the scene before drew are copied from it (S56), which leaves
    /// that scene without its items: on an error the caller keeps no scene. A scene the raster
    /// worker still holds (S60) gives none: they are made anew, the same.
    fn built(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        active: &ActivePlace,
    ) -> Result<(Scene, SceneOps), RenderError> {
        let started = Instant::now();
        let known = self.scene.as_ref().map(|cache| cache.scene.edges.clone());
        let earlier = (self.scene.as_mut())
            .and_then(|cache| Arc::get_mut(&mut cache.scene))
            .map(Scene::take_earlier);
        let known = (known.unwrap_or_default(), earlier.unwrap_or_default());
        let mut scene = Scene::build_knowing(artwork, Some(active), known)?;
        if self.reference {
            scene.routing = Routing::Evaluated(Routed::default());
        }
        let patches = self.scene.as_ref().and_then(|cache| {
            let (old, new) = (&cache.scene.patches, &scene.patches);
            let same = old.len() == new.len() && old.iter().zip(new).all(|(a, b)| a.shares(b));
            same.then(|| cache.ops.patches.clone())
        });
        let ops = scene_ops(&core.gpu, &scene, patches);
        crate::profile::scene_built(started.elapsed());
        // Its filters' results now; the residuals near what each evaluation evaluates (S51).
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        smudges.prepare(core, (&scene, &ops), artwork, (filters, &Wanted::Near(&[])))?;
        Ok((scene, ops))
    }

    fn cached(&self) -> Result<&SceneCache, RenderError> {
        self.scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))
    }

    /// Evaluates `all` (target pixels) of the scene built last for `request` at `output`, with
    /// the in-progress stroke's `fresh` ops (`reset`: its coverage starts over; S63: only a scene
    /// routed to the evaluator draws one here). Given `now` (one frame of the screen, S49), the
    /// tiles meeting it and then what one frame's work allows: the others wait (`pending`), and the
    /// regions evaluated are returned.
    pub(super) fn draw(
        &mut self,
        core: &Core,
        (request, (output, reset)): (&FrameRequest<'_>, (Output, bool)),
        fresh: Fresh,
        (all, now): (&[PixelRect], Option<&[PixelRect]>),
    ) -> Result<Option<Vec<PixelRect>>, RenderError> {
        let size = self.size();
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let started = Instant::now();
        let previewing = fresh.ops.is_empty() && output == Output::CANVAS;
        let tools = (&mut self.smudges, &mut self.filters);
        prepare_early(core, tools, (cache, request.artwork), previewing)?;
        let residuals = self.smudges.residuals();
        let bindings = (&mut self.bindings, residuals);
        let readied = cache.ready(core, bindings, fresh, (size, output, all))?;
        let cache = &*cache;
        let reading = cache.reading(readied)?;
        let tiles = reading.only.unwrap_or(&reading.plan.tiles);
        let work = core.evaluate.frame_work();
        let waiting = (&mut self.waiting, &mut self.blank);
        let (only, evaluated) = frame_tiles(tiles, (now, work), waiting);
        let only = only.as_deref().or(reading.only);
        let tiles = only.unwrap_or(&reading.plan.tiles);
        self.work = EvaluationWork::of(tiles, started.elapsed());
        if self.work.tiles == 0 && !reset {
            return Ok(evaluated);
        }
        // S51: the residuals near the tiles evaluated now.
        let near = reached(tiles, output.scale);
        let filters = (&mut self.filters, &Wanted::Near(&near));
        let scene = (&*cache.scene, &cache.ops);
        self.smudges
            .prepare(core, scene, request.artwork, filters)?;
        let residuals = self.smudges.residuals();
        let paper = request.background;
        core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops: reading.ops,
                fresh: &reading.fresh,
                plan: reading.plan,
                only,
                output,
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                reset,
                residuals,
                base: Affine::IDENTITY,
                snapshot: reading.snapshot,
                timing: None,
            },
        )?;
        Ok(evaluated)
    }
}

#[path = "evaluator_retune.rs"]
mod retune;
use retune::next_serial;

/// S51: the residuals a preview's snapshot reads before the frame's own evaluation: its layer
/// whole while it takes its snapshot (`previewing`).
fn prepare_early(
    core: &Core,
    (smudges, filters): (&mut Smudges, &mut Filters),
    (cache, artwork): (&SceneCache, &Artwork),
    previewing: bool,
) -> Result<(), RenderError> {
    let Some(items) = cache.preview_items(previewing) else {
        return Ok(());
    };
    let scene = (&*cache.scene, &cache.ops);
    smudges.prepare(core, scene, artwork, (filters, &Wanted::Items(items)))
}

/// The tiles of `tiles` this evaluation takes: given `now` (one frame of a preview, S35, S49),
/// those meeting it and then what `work` allows, the others left `waiting` (a `blank` target's
/// shown as paper meanwhile), with the regions evaluated; else all of them.
fn frame_tiles(
    tiles: &[TileRecord],
    (now, work): (Option<&[PixelRect]>, u64),
    (waiting, blank): (&mut Waiting, &mut bool),
) -> (Option<Vec<TileRecord>>, Option<Vec<PixelRect>>) {
    let share = now.and_then(|now| frame_share(tiles, now, work));
    let taken = match share {
        Some((mut chosen, left)) => {
            waiting.add(&tile_rects(&left));
            let mut evaluated = now.unwrap_or_default().to_vec();
            evaluated.extend(tile_rects(&chosen));
            // A new target's waiting tiles show the paper meanwhile, not undefined pixels.
            if *blank {
                chosen.extend(paper_only(&left));
            }
            (Some(chosen), Some(evaluated))
        }
        None => (None, None),
    };
    *blank = false;
    taken
}

#[path = "evaluator_reading.rs"]
mod reading;

impl SceneCache {
    /// Its version only gives an adjustment new values (S35): its frames evaluate the preview's
    /// tiles as S49 did, not as background work.
    pub(super) const fn previewing(&self) -> bool {
        self.previewed.is_some()
    }

    /// Bytes of the scene's ops on the GPU, and of the previewed layer's snapshot (S49).
    pub(super) fn bytes(&self) -> [u64; 2] {
        let ops = &self.ops;
        let scene = ops.ops().size() + ops.masks.size() + ops.patches.size();
        [
            scene,
            self.snapshot.as_ref().map_or(0, PreviewSnapshot::bytes),
        ]
    }

    /// The plan of the last evaluation without an in-progress stroke (S55's benchmark).
    pub(super) fn kept_plan(&self) -> Option<&Plan> {
        self.plan.as_ref().map(|(_, plan)| plan)
    }

    /// Lets the previewed layer's snapshot go; the next value takes it again, within the room.
    pub(super) fn drop_snapshot(&mut self) {
        self.snapshot = None;
    }

    /// Heap bytes of the scene (its items, mask words and smudge and filter items) and of the
    /// plan kept (its tiles and op lists), for the harness's long session (S51).
    pub(super) fn heap_bytes(&self) -> [u64; 2] {
        let scene = &self.scene;
        let items = scene.items.capacity() * size_of::<crate::scene::Item>()
            + scene.masks.capacity() * 4
            + scene.smudges.capacity() * size_of::<crate::scene::SmudgeItem>()
            + scene.filters.capacity() * size_of::<crate::filter::FilterItem>();
        let plan = self.plan.as_ref().map_or(0, |(_, plan)| {
            plan.tiles.capacity() * size_of::<TileRecord>() + plan.indices.capacity() * 4
        });
        [items as u64, plan as u64]
    }

    /// The serial of the plan kept (`Plan::serial`; 0: none), for tests.
    #[cfg(test)]
    pub(super) fn plan_serial(&self) -> u64 {
        self.plan.as_ref().map_or(0, |(_, plan)| plan.serial)
    }
}

#[cfg(test)]
#[path = "evaluator_scene_tests.rs"]
mod tests;
