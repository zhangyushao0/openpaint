//! A kept scene changed in place: given an adjustment's new values (S35's previews, `preview.rs`),
//! or the strokes a version appends on its place (S64, `scene_append.rs`); and the serials naming
//! each scene built or changed.

use std::sync::Arc;
use std::time::Instant;

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::preview::Retune;
use crate::protocol::{Op, encode};
use crate::scene::ActivePlace;
use crate::scene_append::{appended, appended_over};
use crate::smudge::Wanted;

impl CanvasTarget {
    /// Gives the kept scene, of the place `active`, the strokes `artwork` appends on that place
    /// to the version it was built for, in place (its ops in a buffer of their own, the scene's
    /// before copied on the GPU); whether it did. It does not when the version changed otherwise,
    /// the raster worker holds the scene, or the scene does not take them (`Scene::append`).
    pub(super) fn append(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        active: &ActivePlace,
    ) -> Result<bool, RenderError> {
        let place = (active.layer, active.on_mask);
        let Some(cache) = (self.scene.as_mut()).filter(|cache| {
            cache.active == *active && appended_over(&cache.artwork, artwork, place)
        }) else {
            return Ok(false);
        };
        let strokes = appended(&cache.artwork, artwork, place);
        let Some(scene) = Arc::get_mut(&mut cache.scene).filter(|_| !strokes.is_empty()) else {
            return Ok(false);
        };
        let started = Instant::now();
        let Some(inserted) = scene.append(&strokes) else {
            return Ok(false);
        };
        cache.ops = cache.ops.inserted(&core.gpu, scene, inserted);
        crate::profile::scene_built(started.elapsed());
        core.evaluate.tally.appended();
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        let scene = (&*cache.scene, &cache.ops);
        smudges.prepare(core, scene, artwork, (filters, &Wanted::Near(&[])))?;
        cache.artwork = artwork.clone();
        cache.serial = next_serial();
        Ok(true)
    }

    /// Gives the kept scene `retune`'s values: the ops and tables that changed rewritten on the
    /// GPU, a filter's result prepared anew.
    pub(super) fn retune(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        retune: &Retune,
    ) -> Result<(), RenderError> {
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let scene = Arc::get_mut(&mut cache.scene).ok_or(RenderError::Invalid(
            "a scene the raster worker holds is not retuned",
        ))?;
        let changed = scene.retune(retune)?;
        let queue = &core.gpu.queue;
        for item in changed.items {
            let op = [cache.scene.items[item].op()];
            queue.write_buffer(
                &cache.ops.ops(),
                (item * size_of::<Op>()) as u64,
                &encode(&op),
            );
        }
        for words in changed.words {
            let at = (words.start * 4) as u64;
            queue.write_buffer(&cache.ops.masks, at, &encode(&cache.scene.masks[words]));
        }
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        let scene = (&*cache.scene, &cache.ops);
        smudges.prepare(core, scene, artwork, (filters, &Wanted::Near(&[])))?;
        cache.artwork = artwork.clone();
        cache.serial = next_serial();
        Ok(())
    }
}

/// The next `SceneCache::serial`.
pub(super) fn next_serial() -> u64 {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}
