//! A canvas target's scene: built and uploaded once per artwork version, changed in place when a
//! version only gives a previewed adjustment new values (S35, `preview.rs`), with the plan it
//! was last evaluated by and the previewed layer's snapshot kept as long as both stay valid.

use std::ops::Range;

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::preview::{Retune, Snapshot, retuned};
use crate::program::{Bindings, SceneOps, scene_ops};
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
    snapshot: Option<Snapshot>,
    /// The plan of the last evaluation without an in-progress stroke, and what it binned.
    plan: Option<(PlanKey, Plan)>,
}

/// What a kept plan was binned for.
#[derive(PartialEq)]
struct PlanKey {
    regions: Vec<PixelRect>,
    output: Output,
    resume: Option<Range<usize>>,
}

/// How one evaluation reads the scene: its plan, its fresh ops, and the snapshot it resumes
/// the previewed layer from.
pub(super) struct Reading<'a> {
    pub(super) ops: &'a SceneOps,
    pub(super) plan: &'a Plan,
    pub(super) fresh: Vec<Op>,
    pub(super) snapshot: Option<&'a Snapshot>,
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
    /// in-progress stroke's evaluations bin their own plan each time; others keep it, and a
    /// previewed adjustment's layer resumes from its snapshot (taken first through the target's
    /// `bindings`, with the arena and in-progress residual `residuals`).
    pub(super) fn reading(
        &mut self,
        core: &Core,
        (bindings, residuals): (&mut Bindings, [&wgpu::Buffer; 2]),
        fresh: Fresh,
        (size, output, regions): ([u32; 2], Output, &[PixelRect]),
    ) -> Result<Reading<'_>, RenderError> {
        let previewing = fresh.ops.is_empty() && output == Output::CANVAS;
        if let Some(place) = self.previewed.filter(|_| previewing)
            && self.snapshot.as_ref().is_none_or(|s| s.place != place)
        {
            let scene = (&self.scene, &self.ops);
            self.snapshot = Snapshot::take(core, scene, (bindings, residuals), place)?;
        }
        let resume = self
            .snapshot
            .as_ref()
            .filter(|s| previewing && self.previewed == Some(s.place))
            .map(|s| s.prefix.clone());
        let key = PlanKey {
            regions: regions.to_vec(),
            output,
            resume: resume.clone(),
        };
        if !fresh.ops.is_empty() || self.plan.as_ref().is_none_or(|(kept, _)| *kept != key) {
            let plan = match resume.clone() {
                Some(prefix) => Plan::resuming(&self.scene, (size, output), regions, prefix),
                None => Plan::build(&self.scene, &fresh, (size, output), regions),
            };
            self.plan = Some((key, plan));
        }
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
            snapshot: resume.as_ref().and(self.snapshot.as_ref()),
            only: resume
                .and(self.near.as_ref())
                .map(|(_, _, tiles)| tiles.as_slice()),
        })
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
