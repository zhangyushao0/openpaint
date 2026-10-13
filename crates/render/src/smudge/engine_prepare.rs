//! Making the residuals an evaluation needs present in the arena (S51): which of them, and which
//! tiles of each, `engine_needs.rs` says; here they are evaluated in scene order, whole when the
//! device binds them all, else only the tiles needed, and the arena keeps within its room by
//! dropping the residuals needed least lately.

use std::collections::HashSet;

use openpaint_core::Artwork;

use super::needs::{self, Need, Wanted};
use super::{
    Active, Keep, Key, Smudges, TileSet, binding_limit, budget, dabs, key, material, node_words,
};
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::program::SceneOps;
use crate::scene::{Scene, SmudgeItem};

impl Smudges {
    /// Makes what `wanted` needs of `scene`'s smudges (built from `artwork`, uploaded as `ops`)
    /// and every filter present in the arena, in scene order, with what those evaluated anew
    /// read, and writes where each block starts into the scene's base words (0 for a residual
    /// not held: no evaluation reads it). A just committed smudge takes over the in-progress
    /// residual when it is that stroke on the same layer content.
    ///
    /// # Errors
    /// `Admission` when the needed blocks exceed what the device binds; `Invalid` for a canvas
    /// the device cannot hold.
    pub(crate) fn prepare(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        artwork: &Artwork,
        (filters, wanted): (&mut Filters, &Wanted<'_>),
    ) -> Result<(), RenderError> {
        let keys = budget::keys_of(scene, filters);
        let limit = binding_limit(core).min(self.limit);
        let (needs, whole) = self.needs(scene, artwork, (&keys, wanted), limit);
        let filtered = keys[scene.smudges.len()..].iter();
        let needed: HashSet<Key> = (keys.iter().zip(&needs))
            .filter(|(_, need)| **need != Need::No)
            .map(|(key, _)| *key)
            .chain(filtered.copied())
            .collect();
        let shown: HashSet<Key> = keys.iter().copied().collect();
        let keep = Keep {
            shown: &shown,
            needed: &needed,
            room: self.room,
            limit,
        };
        if !self.framed {
            self.arena.next();
        }
        for key in &needed {
            self.arena.touch(key);
        }
        let size = [artwork.canvas().width(), artwork.canvas().height()];
        let smudges = scene.smudges.iter().map(|item| item.item);
        let mut order: Vec<(usize, usize)> = smudges
            .chain(scene.filters.iter().map(|item| item.item))
            .enumerate()
            .map(|(index, item)| (item, index))
            .collect();
        order.sort_unstable();
        // The blocks already present, for the materials evaluated below to read.
        self.write_bases(core, ops, scene.bases_at, &keys);
        for (_, index) in order {
            match scene.smudges.get(index) {
                Some(item) => {
                    let Some(tiles) = self.missing(&keys[index], &needs[index], whole) else {
                        continue;
                    };
                    self.prepare_smudge(core, (scene, ops), artwork, (item, tiles), &keep)?;
                }
                None if self.arena.base(&keys[index]).is_some() => continue,
                None => {
                    let item = &scene.filters[index - scene.smudges.len()];
                    let material = material(&mut self.tools.material, core, size)?;
                    let held = self.tools.owner == 0;
                    let tools = (&mut self.arena, material, &self.tools.empty, held);
                    let place = (item, keys[index]);
                    filters.prepare(core, (scene, ops), place, tools, &keep)?;
                    // The material holds the filter's pixels now.
                    self.tools.owner = 0;
                }
            }
            self.write_bases(core, ops, scene.bases_at, &keys);
        }
        self.write_bases(core, ops, scene.bases_at, &keys);
        filters.retain(&self.arena);
        Ok(())
    }

    /// What `wanted` needs of each smudge, with what those evaluated anew read, and whether the
    /// residuals evaluated anew go whole (`fits`); when even their parts do not fit beside the
    /// residuals held whole of which only a part is needed, those go too, to be made again in
    /// part with what they read.
    pub(super) fn needs(
        &mut self,
        scene: &Scene,
        artwork: &Artwork,
        (keys, wanted): (&[Key], &Wanted<'_>),
        limit: u64,
    ) -> (Vec<Need>, bool) {
        let mut needs = needs::needs(scene, wanted);
        // A just committed smudge takes the in-progress residual whole, before it ends.
        for (need, item) in needs.iter_mut().zip(&scene.smudges) {
            let adopts = |active: &Active| active.adopts(item, artwork);
            if item.newest && self.active.as_ref().is_some_and(adopts) {
                *need = Need::All;
            }
        }
        self.reads.keep(scene);
        needs::close(scene, (keys, &self.arena, &mut self.reads), &mut needs);
        let [whole, parts] = self.fits(scene, (keys, &needs), limit);
        if !parts {
            let cut: Vec<&Key> = (keys.iter().zip(&needs))
                .filter(|(key, need)| matches!(need, Need::Part(_)) && self.arena.whole(key))
                .map(|(key, _)| key)
                .collect();
            for key in &cut {
                self.arena.forget(key);
            }
            if !cut.is_empty() {
                needs::close(scene, (keys, &self.arena, &mut self.reads), &mut needs);
            }
        }
        (needs, whole)
    }

    /// The tiles a residual needed but not held is to be made with (`None`: whole): what it
    /// needs with what it held before, or whole when everything goes whole; `None` outside when
    /// it is not needed or held as needed.
    fn missing(&self, key: &Key, need: &Need, whole: bool) -> Option<Option<TileSet>> {
        let tiles = need.tiles()?;
        if self.arena.holds(key, tiles) {
            return None;
        }
        Some(tiles.filter(|_| !whole).map(|tiles| {
            let mut joined = tiles.clone();
            if let Some(held) = self.arena.part(key) {
                joined.join(held);
            }
            joined
        }))
    }

    /// The arena keeps within `room` bytes when it can from now on (S51).
    pub(crate) fn set_room(&mut self, room: u64) {
        self.room = room;
    }

    /// The room the arena keeps within when it can, in bytes.
    pub(crate) const fn room(&self) -> u64 {
        self.room
    }

    /// A new frame begins (after the budget's step on the arena): what its evaluations need
    /// counts as needed together, most recently.
    pub(crate) fn next_frame(&mut self) {
        self.framed = true;
        self.arena.next();
    }

    /// The arena binds at most `bytes` from now on, as a device with that limit would.
    pub(crate) fn set_limit(&mut self, bytes: u64) {
        self.limit = bytes;
    }

    /// A finished smudge's residual, from the in-progress one when it adopts it, else cold,
    /// placed whole or as its `tiles`.
    fn prepare_smudge(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        artwork: &Artwork,
        (item, tiles): (&SmudgeItem, Option<TileSet>),
        keep: &Keep<'_>,
    ) -> Result<(), RenderError> {
        let size = [artwork.canvas().width(), artwork.canvas().height()];
        let range = (item.before.clone(), item.to_canvas);
        let adopted = self.active.take_if(|active| active.adopts(item, artwork));
        let run = match adopted {
            Some(mut active) => {
                let mut new = Vec::new();
                let samples = item.stroke.samples();
                active
                    .walker
                    .advance(&active.style, (samples, true), &mut new);
                let scene = (scene, ops, range);
                self.tools
                    .extend(core, scene, &self.arena, &mut active.run, &new)?;
                active.run
            }
            None => {
                core.evaluate.tally.derived();
                let mut run = self.tools.cold_run(core, size);
                let all = dabs(item.stroke.style(), item.stroke.samples());
                let scene = (scene, ops, range);
                self.tools
                    .extend(core, scene, &self.arena, &mut run, &all)?;
                run
            }
        };
        let key = key(item.layer, &item.stroke);
        let place = (&run.working, tiles);
        self.arena
            .place(&core.gpu, (key, &item.stroke), place, keep)?;
        // Its buffers serve the next cold evaluation.
        self.tools.scratch = Some(run);
        Ok(())
    }

    /// Whether the residuals `needs` asks to be evaluated anew fit the device's binding `limit`
    /// beside the needed blocks it holds: whole, and in their needed parts. Whole when they do,
    /// so a later evaluation near the rest of one finds it; the room is kept by dropping what is
    /// not needed.
    fn fits(&mut self, scene: &Scene, (keys, needs): (&[Key], &[Need]), limit: u64) -> [bool; 2] {
        let tile_words = node_words(1);
        let [mut whole, mut parts] = [0, 0];
        for ((item, key), need) in scene.smudges.iter().zip(keys).zip(needs) {
            match need.tiles() {
                Some(tiles) if !self.arena.holds(key, tiles) => {
                    let all = self.reads.of(item, scene.canvas).len() as u64 * tile_words;
                    whole += all;
                    parts += tiles.map_or(all, |tiles| tiles.0.len() as u64 * tile_words);
                }
                Some(_) => {
                    whole += self.arena.words(key);
                    parts += self.arena.words(key);
                }
                None => {}
            }
        }
        [whole, parts].map(|words| words.saturating_mul(4) <= limit)
    }
}
