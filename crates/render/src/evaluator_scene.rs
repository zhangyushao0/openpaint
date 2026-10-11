//! A canvas target's scene: built and uploaded once per artwork version, changed in place when a
//! version only gives a previewed adjustment new values (S35, `preview.rs`), with the plan it
//! was last evaluated by and the previewed layer's snapshot kept as long as both stay valid.

use std::ops::Range;
use std::time::{Duration, Instant};

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::active_snapshot::{self, Kept};
use crate::affine::Affine;
use crate::bounded::{frame_share, paper_only, tile_rects};
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::preview::{PreviewSnapshot, Retune, retuned};
use crate::program::{Bindings, Dispatch, SceneOps, SnapshotUse, scene_ops};
use crate::protocol::{Op, TileRecord, encode};
use crate::scene::{ActivePlace, Fresh, Scene};
use crate::scene_adjust::EditPlace;

/// One artwork version's scene and its ops on the GPU, reused while the version stays.
pub(super) struct SceneCache {
    pub(super) artwork: Artwork,
    pub(super) active: Option<ActivePlace>,
    pub(super) scene: Scene,
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
}

/// The in-progress stroke's layer items a plan leaves unbinned, and the fresh op that resumes
/// them from its snapshot (`Plan::over_snapshot`).
type Skipped = (Range<usize>, u32);

/// What a kept plan was binned for.
#[derive(PartialEq)]
struct PlanKey {
    regions: Vec<PixelRect>,
    output: Output,
    resume: Option<Range<usize>>,
}

/// What one evaluation into a target cost (S48): the time taken to bin the plan on the CPU (and
/// to bring a snapshot it resumes from up to date), the tiles it evaluated and the op entries
/// their lists held, which the GPU walks per pixel; and for the in-progress stroke the tiles of
/// its layer's snapshot it took (evaluating the layer's ops into them) and those it laid
/// strokes committed since into (`active_snapshot.rs`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvaluationWork {
    pub planned: Duration,
    pub tiles: usize,
    pub entries: usize,
    pub taken: usize,
    pub carried: usize,
}

impl EvaluationWork {
    /// What evaluating `tiles` costs, planned in `planned`, with what the snapshot `kept`.
    pub(super) fn of(tiles: &[TileRecord], planned: Duration, [taken, carried]: Kept) -> Self {
        Self {
            planned,
            tiles: tiles.len(),
            entries: tiles.iter().map(|t| t.count as usize).sum(),
            taken,
            carried,
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
        active: Option<ActivePlace>,
    ) -> Result<&SceneCache, RenderError> {
        let same =
            |cache: &SceneCache| cache.artwork.same_version(artwork) && cache.active == active;
        if self.scene.as_ref().is_some_and(same) {
            return self.cached();
        }
        let retune = self
            .scene
            .as_ref()
            .filter(|cache| active.is_none() && cache.active.is_none())
            .and_then(|cache| retuned(&cache.artwork, artwork));
        if let Some(retune) = retune {
            if let Err(error) = self.retune(core, artwork, &retune) {
                self.scene = None;
                return Err(error);
            }
            return self.cached();
        }
        let known = self.scene.as_ref().map(|cache| cache.scene.edges.clone());
        let scene = Scene::build_knowing(artwork, active.as_ref(), known.unwrap_or_default())?;
        let patches = self.scene.as_ref().and_then(|cache| {
            let (old, new) = (&cache.scene.patches, &scene.patches);
            let same = old.len() == new.len() && old.iter().zip(new).all(|(a, b)| a.shares(b));
            same.then(|| cache.ops.patches.clone())
        });
        let ops = scene_ops(&core.gpu, &scene, patches);
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        smudges.prepare(core, (&scene, &ops), artwork, filters)?;
        self.scene = Some(SceneCache {
            artwork: artwork.clone(),
            active,
            scene,
            ops,
            previewed: None,
            spread: 0,
            near: None,
            snapshot: None,
            plan: None,
        });
        self.cached()
    }

    fn cached(&self) -> Result<&SceneCache, RenderError> {
        self.scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))
    }

    /// Evaluates `all` (target pixels) of the scene built last for `request` at `output`, with
    /// the in-progress stroke's `fresh` ops (`reset`: its coverage starts over), its layer
    /// resumed from the snapshot of the place `held` when given (S48). Given `now` (one frame of
    /// the screen, S49), the tiles meeting it and then what one frame's work allows: the others
    /// wait (`pending`), and the regions evaluated are returned.
    pub(super) fn draw(
        &mut self,
        core: &Core,
        (request, (output, reset)): (&FrameRequest<'_>, (Output, bool)),
        (fresh, held): (Fresh, Option<ActivePlace>),
        (all, now): (&[PixelRect], Option<&[PixelRect]>),
    ) -> Result<Option<Vec<PixelRect>>, RenderError> {
        let size = self.size();
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let residuals = self.smudges.residuals();
        let started = Instant::now();
        let (held, kept) = match held {
            Some(place) => {
                let evaluating = ((&cache.scene, &cache.ops), (&mut self.bindings, residuals));
                let drawing = (request.artwork, &place);
                active_snapshot::prepare(&mut self.held, core, evaluating, drawing, all)?
            }
            None => (None, [0; 2]),
        };
        let bindings = (&mut self.bindings, residuals);
        let reading = cache.reading(core, bindings, fresh, (size, output, all), held)?;
        let tiles = reading.only.unwrap_or(&reading.plan.tiles);
        let share = now.and_then(|now| frame_share(tiles, now, core.evaluate.frame_work()));
        let (only, evaluated) = match share {
            Some((mut chosen, left)) => {
                self.pending = tile_rects(&left);
                let mut evaluated = now.unwrap_or_default().to_vec();
                evaluated.extend(tile_rects(&chosen));
                // A new target's waiting tiles show the paper meanwhile, not undefined pixels.
                if self.blank {
                    chosen.extend(paper_only(&left));
                }
                (Some(chosen), Some(evaluated))
            }
            None => (None, None),
        };
        self.blank = false;
        let only = only.as_deref().or(reading.only);
        let tiles = only.unwrap_or(&reading.plan.tiles);
        self.work = EvaluationWork::of(tiles, started.elapsed(), kept);
        if self.work.tiles == 0 && !reset {
            return Ok(evaluated);
        }
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
            },
        )?;
        Ok(evaluated)
    }

    /// Gives the kept scene `retune`'s values: the ops and tables that changed rewritten on the
    /// GPU, a filter's result prepared anew.
    fn retune(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        retune: &Retune,
    ) -> Result<(), RenderError> {
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let changed = cache.scene.retune(retune)?;
        let queue = &core.gpu.queue;
        for item in changed.items {
            let op = [cache.scene.items[item].op()];
            queue.write_buffer(
                &cache.ops.ops,
                (item * size_of::<Op>()) as u64,
                &encode(&op),
            );
        }
        for words in changed.words {
            let at = (words.start * 4) as u64;
            queue.write_buffer(&cache.ops.masks, at, &encode(&cache.scene.masks[words]));
        }
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        smudges.prepare(core, (&cache.scene, &cache.ops), artwork, filters)?;
        cache.artwork = artwork.clone();
        cache.spread = retune.spread;
        if cache.previewed != Some(retune.place) {
            cache.previewed = Some(retune.place);
            cache.snapshot = None;
        }
        Ok(())
    }
}

impl SceneCache {
    /// How the evaluation of `regions` at `output` with `fresh` reads this scene: the
    /// in-progress stroke's evaluations bin their own plan each time, over its layer's snapshot
    /// `held` (S48: those items and the snapshot) when there is one; others keep it, and a
    /// previewed adjustment's layer resumes from its snapshot (taken first through the target's
    /// `bindings`, with the arena and in-progress residual `residuals`).
    pub(super) fn reading<'a>(
        &'a mut self,
        core: &Core,
        (bindings, residuals): (&mut Bindings, [&wgpu::Buffer; 2]),
        fresh: Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
        held: Option<(Range<usize>, SnapshotUse<'a>)>,
    ) -> Result<Reading<'a>, RenderError> {
        let previewing = fresh.ops.is_empty() && output == Output::CANVAS;
        if let Some(place) = self.previewed.filter(|_| previewing)
            && self.snapshot.as_ref().is_none_or(|s| s.place != place)
        {
            let scene = (&self.scene, &self.ops);
            self.snapshot = PreviewSnapshot::take(core, scene, (bindings, residuals), place)?;
        }
        let resume = self
            .snapshot
            .as_ref()
            .filter(|s| previewing && self.previewed == Some(s.place))
            .map(|s| s.prefix.clone());
        let (fresh, over) = match held.filter(|_| !fresh.ops.is_empty()) {
            Some((prefix, snapshot)) => {
                let (fresh, at) = fresh.resuming();
                (fresh, Some((prefix, at, snapshot)))
            }
            None => (fresh, None),
        };
        let skipped = over.as_ref().map(|(prefix, at, _)| (prefix.clone(), *at));
        self.plan_for(&fresh, (size, output, regions), (resume.clone(), skipped));
        let plan = self
            .plan
            .as_ref()
            .map(|(_, plan)| plan)
            .ok_or(RenderError::Invalid("no plan was built"))?;
        let stale = |(spread, serial, _): &(u32, u64, Vec<TileRecord>)| {
            *spread != self.spread || *serial != plan.serial
        };
        if resume.is_some() && self.near.as_ref().is_none_or(stale) {
            self.near = Some((self.spread, plan.serial, plan.near_resumed(self.spread)));
        }
        Ok(Reading {
            ops: &self.ops,
            plan,
            fresh: if resume.is_some() {
                vec![Op::resume()]
            } else {
                fresh.ops
            },
            snapshot: match over {
                Some((_, _, snapshot)) => Some(snapshot),
                None => resume
                    .as_ref()
                    .and(self.snapshot.as_ref())
                    .map(|snapshot| snapshot.reading(false)),
            },
            only: resume
                .and(self.near.as_ref())
                .map(|(_, _, tiles)| tiles.as_slice()),
        })
    }

    /// Bins the plan for `regions` at `output`, unless the plan kept was binned for them and
    /// nothing is fresh: resuming a previewed layer (`resume`), over the in-progress stroke's
    /// layer snapshot (`over`: its items and the fresh op resuming them), or every item.
    fn plan_for(
        &mut self,
        fresh: &Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
        (resume, over): (Option<Range<usize>>, Option<Skipped>),
    ) {
        let key = PlanKey {
            regions: regions.to_vec(),
            output,
            resume: resume.clone(),
        };
        if fresh.ops.is_empty() && self.plan.as_ref().is_some_and(|(kept, _)| *kept == key) {
            return;
        }
        let target = (size, output);
        let plan = match (resume, over) {
            (Some(prefix), _) => Plan::resuming(&self.scene, target, regions, prefix),
            (None, Some(over)) => Plan::over_snapshot(&self.scene, fresh, target, regions, over),
            (None, None) => Plan::build(&self.scene, fresh, target, regions),
        };
        self.plan = Some((key, plan));
    }

    /// Bytes of the scene's ops on the GPU, and of the previewed layer's snapshot (S49).
    pub(super) fn bytes(&self) -> [u64; 2] {
        let ops = &self.ops;
        let scene = ops.ops.size() + ops.masks.size() + ops.patches.size();
        [
            scene,
            self.snapshot.as_ref().map_or(0, PreviewSnapshot::bytes),
        ]
    }

    /// Lets the previewed layer's snapshot go; the next value takes it again, within the room.
    pub(super) fn drop_snapshot(&mut self) {
        self.snapshot = None;
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
