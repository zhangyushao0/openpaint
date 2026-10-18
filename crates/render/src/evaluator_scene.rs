//! A canvas target's scene: built and uploaded once per artwork version, changed in place when a
//! version only gives an adjustment new values (S35's previews: `preview.rs`, every value drawn
//! anew since S65) or appends strokes on the place (S64, `scene_append.rs`); and, for a
//! reference (S65: the evaluator's one use), the plan it was evaluated by.

use std::sync::Arc;
use std::time::Instant;

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::preview::retuned;
use crate::program::{Dispatch, SceneOps, scene_ops};
use crate::protocol::TileRecord;
use crate::scene::{ActivePlace, Scene};
use crate::smudge::{Wanted, reached};

/// One artwork version's scene and its ops on the GPU, reused while the version stays.
pub(super) struct SceneCache {
    pub(super) artwork: Artwork,
    /// The in-progress stroke's place, or while none is drawn the next one's (S56).
    pub(super) active: ActivePlace,
    /// Shared with the raster worker drawing tiles from it (S60).
    pub(super) scene: Arc<Scene>,
    pub(super) ops: SceneOps,
    /// A reference's plan, kept for its benchmark.
    plan: Option<Plan>,
    /// Names this scene among every scene built or changed in place (S53).
    pub(super) serial: u64,
}

impl CanvasTarget {
    /// The scene of `artwork` with the in-progress stroke's place `active`, built and uploaded
    /// only when either differs from the last call (changed in place when only an adjustment's
    /// values did, or strokes were appended on the place), with every filter it draws prepared.
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
        let changed = match retune {
            Some(retune) => self.retune(core, artwork, &retune).map(|()| true),
            // S64: the strokes a lift commits on the place, taken in place.
            None => self.append(core, artwork, &active),
        };
        match changed {
            Ok(true) => return self.cached(),
            Ok(false) => {}
            Err(error) => {
                self.scene = None;
                return Err(error);
            }
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
            serial: next_serial(),
            scene: Arc::new(scene),
            ops,
            plan: None,
        });
        self.cached()
    }

    /// A new scene of `artwork` with the place `active` on the GPU, its filters prepared. The
    /// strokes' spans the scene before drew are copied from it (S56), which leaves that scene
    /// without its items: on an error the caller keeps no scene. A scene the raster worker still
    /// holds (S60) gives none: they are made anew, the same.
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
        let scene = Scene::build_knowing(artwork, Some(active), known)?;
        let patches = self.scene.as_ref().and_then(|cache| {
            let (old, new) = (&cache.scene.patches, &scene.patches);
            let same = old.len() == new.len() && old.iter().zip(new).all(|(a, b)| a.shares(b));
            same.then(|| cache.ops.patches.clone())
        });
        let ops = scene_ops(&core.gpu, &scene, patches);
        crate::profile::scene_built(started.elapsed());
        // Its filters' results now; the residuals near what each draw draws (S51).
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        smudges.prepare(core, (&scene, &ops), artwork, (filters, &Wanted::Near(&[])))?;
        Ok((scene, ops))
    }

    fn cached(&self) -> Result<&SceneCache, RenderError> {
        self.scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))
    }

    /// A reference's evaluation of `whole` (its pixels) of the scene built last for `request` at
    /// `output`, by the evaluator: binned, the residuals it reads made present, dispatched.
    pub(super) fn draw(
        &mut self,
        core: &Core,
        (request, output): (&FrameRequest<'_>, Output),
        whole: &PixelRect,
    ) -> Result<(), RenderError> {
        let size = self.size();
        let bindings = (self.bindings.as_mut()).ok_or(RenderError::Invalid("no reference"))?;
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let plan = Plan::build(&cache.scene, (size, output), std::slice::from_ref(whole));
        core.evaluate.tally.binned(plan.indices.len());
        // S51: the residuals near the tiles evaluated.
        let near = reached(&plan.tiles, output.scale);
        let filters = (&mut self.filters, &Wanted::Near(&near));
        let scene = (&*cache.scene, &cache.ops);
        self.smudges
            .prepare(core, scene, request.artwork, filters)?;
        let paper = request.background;
        core.evaluate.dispatch(
            &core.gpu,
            bindings,
            &Dispatch {
                ops: &cache.ops,
                fresh: &[],
                plan: &plan,
                output,
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                residuals: self.smudges.residuals(),
                base: Affine::IDENTITY,
                timing: None,
            },
        )?;
        cache.plan = Some(plan);
        Ok(())
    }
}

#[path = "evaluator_retune.rs"]
mod retune;
use retune::next_serial;

impl SceneCache {
    /// Bytes of the scene's ops on the GPU (S49).
    pub(super) fn bytes(&self) -> u64 {
        let ops = &self.ops;
        ops.ops().size() + ops.masks.size() + ops.patches.size()
    }

    /// A reference's plan (S55's benchmark).
    pub(super) fn kept_plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    /// Heap bytes of the scene (its items, mask words and smudge and filter items) and of the
    /// plan kept (its tiles and op lists), for the harness's long session (S51).
    pub(super) fn heap_bytes(&self) -> [u64; 2] {
        let scene = &self.scene;
        let items = scene.items.capacity() * size_of::<crate::scene::Item>()
            + scene.masks.capacity() * 4
            + scene.smudges.capacity() * size_of::<crate::scene::SmudgeItem>()
            + scene.filters.capacity() * size_of::<crate::filter::FilterItem>();
        let plan = self.plan.as_ref().map_or(0, |plan| {
            plan.tiles.capacity() * size_of::<TileRecord>() + plan.indices.capacity() * 4
        });
        [items as u64, plan as u64]
    }
}
