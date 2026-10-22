//! A canvas target's smudge and filter memory under the GPU memory budget: what it holds, the
//! arena cut down to the blocks the scene shows (the rest is derived again from the artwork if a
//! later version shows it), and the residuals shown kept within a room, the least recently needed
//! going first, never those the last draw needed.

use std::collections::HashSet;

use super::{Keep, Key, Smudges, binding_limit, key};
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
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
        let scratch = (tools.scratch.as_ref()).map_or(0, |run| run.working.buffer().size());
        self.arena.bytes() + material + kernels + working + scratch + tools.empty.size()
    }

    /// Bytes of the arena's buffer alone.
    pub(crate) fn arena_bytes(&self) -> u64 {
        self.arena.bytes()
    }

    /// Bytes of the arena's blocks the last draw did not need: what its room may cut.
    pub(crate) fn spare_bytes(&self) -> u64 {
        self.arena.spare_words(&self.arena.last_needed()) * 4
    }

    /// Keeps in the arena only the blocks `scene` shows, within the room (the least recently
    /// needed residuals going first, never one the last draw needed), in a buffer just large
    /// enough and, with `material`, lets the material texture go (made again when a smudge or
    /// filter needs it); returns the bytes given back.
    ///
    /// # Errors
    /// `Admission` as `prepare`.
    pub(crate) fn shrink(
        &mut self,
        core: &Core,
        scene: &Scene,
        (filters, material): (&mut Filters, bool),
    ) -> Result<u64, RenderError> {
        let before = self.bytes();
        let keys = keys_of(scene, filters);
        let shown: HashSet<Key> = keys.iter().copied().collect();
        let needed = self.arena.last_needed();
        let keep = Keep {
            shown: &shown,
            needed: &needed,
            room: self.room,
            limit: binding_limit(core).min(self.limit),
        };
        self.arena.shrink(&core.gpu, &keep)?;
        filters.retain(&self.arena);
        if material {
            // The material holds no run's tiles and no filter's pixels any more.
            self.tools.material = None;
            self.tools.scratch = None;
            self.tools.owner = u64::MAX;
        }
        Ok(before.saturating_sub(self.bytes()))
    }
}
