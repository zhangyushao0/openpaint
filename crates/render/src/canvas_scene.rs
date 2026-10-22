//! A canvas target's scene: built and uploaded once per artwork version, changed in place when a
//! version only gives an adjustment new values (a preview's, `preview.rs`) or appends strokes on
//! the place (`scene_append.rs`).

use std::sync::Arc;
use std::time::Instant;

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::canvas::NO_SCENE;
use crate::core::Core;
use crate::error::RenderError;
use crate::preview::retuned;
use crate::scene::{ActivePlace, Scene};
use crate::scene_ops::{SceneOps, scene_ops};

/// One artwork version's scene and its ops on the GPU, reused while the version stays.
pub(super) struct SceneCache {
    pub(super) artwork: Artwork,
    /// The in-progress stroke's place, or while none is drawn the next one's.
    pub(super) active: ActivePlace,
    /// Shared with the raster worker drawing tiles from it.
    pub(super) scene: Arc<Scene>,
    pub(super) ops: SceneOps,
    /// Names this scene among every scene built or changed in place.
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
            // The strokes a lift commits on the place, taken in place.
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
        });
        self.cached()
    }

    /// A new scene of `artwork` with the place `active` on the GPU, its filters prepared. The
    /// strokes' spans the scene before drew are copied from it, which leaves that scene
    /// without its items: on an error the caller keeps no scene. A scene the raster worker still
    /// holds gives none: they are made anew, the same.
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
        // Its filters' results now; the residuals near what each draw draws.
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        smudges.prepare(core, (&scene, &ops), artwork, (filters, &[]))?;
        Ok((scene, ops))
    }

    fn cached(&self) -> Result<&SceneCache, RenderError> {
        self.scene.as_ref().ok_or(NO_SCENE)
    }
}

#[path = "canvas_retune.rs"]
mod retune;
use retune::next_serial;

impl SceneCache {
    /// Bytes of the scene's ops on the GPU.
    pub(super) fn bytes(&self) -> u64 {
        let ops = &self.ops;
        ops.ops().size() + ops.masks.size() + ops.patches.size()
    }

    /// Heap bytes of the scene: its items, mask words and smudge and filter items.
    pub(super) fn heap_bytes(&self) -> u64 {
        let scene = &self.scene;
        let items = scene.items.capacity() * size_of::<crate::scene::Item>()
            + scene.masks.capacity() * 4
            + scene.smudges.capacity() * size_of::<crate::scene::SmudgeItem>()
            + scene.filters.capacity() * size_of::<crate::filter::FilterItem>();
        items as u64
    }
}
