//! A canvas target's smudge and filter memory under the GPU memory budget (S49, D76): what it
//! holds, and the third step of keeping within the budget, the arena cut down to the blocks the
//! scene shows (the rest is evaluated again from the artwork if a later version shows it).

use std::collections::HashSet;

use super::{Key, Smudges, binding_limit, key};
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::program::SceneOps;
use crate::scene::Scene;
use crate::smudge::Material;
use crate::smudge::kernels::KernelBuffers;

/// The arena keys of `scene`'s smudges, then of its filters (numbering results not known yet).
pub(super) fn keys_of(scene: &Scene, filters: &mut Filters) -> Vec<Key> {
    let mut keys: Vec<Key> = scene
        .smudges
        .iter()
        .map(|item| key(item.layer, &item.stroke))
        .collect();
    keys.extend(filters.keys(&scene.filters));
    keys
}

impl Smudges {
    /// Bytes of the arena, the material texture, the kernels' buffers and the residuals.
    pub(crate) fn bytes(&self) -> u64 {
        let tools = &self.tools;
        let material = tools.material.as_ref().map_or(0, Material::bytes);
        let kernels = tools.kernels.as_ref().map_or(0, KernelBuffers::bytes);
        let working = self
            .active
            .as_ref()
            .map_or(0, |active| active.run.working.buffer().size());
        self.arena.bytes() + material + kernels + working + tools.empty.size()
    }

    /// Keeps in the arena only the blocks `scene` (uploaded as `ops`) shows, in a buffer just
    /// large enough, writes their new bases and lets the material texture go (made again when a
    /// smudge or filter needs it); returns the bytes given back.
    ///
    /// # Errors
    /// `Admission` as `prepare`.
    pub(crate) fn shrink(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        filters: &mut Filters,
    ) -> Result<u64, RenderError> {
        let before = self.bytes();
        let keys = keys_of(scene, filters);
        let live: HashSet<Key> = keys.iter().copied().collect();
        self.arena.shrink(&core.gpu, &live, binding_limit(core))?;
        self.write_bases(core, ops, scene.bases_at, &keys);
        filters.retain(&self.arena);
        // The material holds no run's tiles and no filter's pixels any more.
        self.tools.material = None;
        self.tools.owner = u64::MAX;
        Ok(before.saturating_sub(self.bytes()))
    }
}
