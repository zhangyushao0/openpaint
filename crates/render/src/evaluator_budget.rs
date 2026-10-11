//! A canvas target's GPU memory (S49, DECISIONS D76): what its derived caches hold, and two of the
//! budget's steps on it: its snapshots kept within a room (beyond it a frame evaluates every op,
//! as past the device's limits) and its arena cut down to the blocks the scene shows. Neither
//! changes a pixel: both are evaluated again from the artwork when needed.

use super::CanvasTarget;
use crate::active_snapshot::ActiveSnapshot;
use crate::budget::CacheBytes;
use crate::core::Core;
use crate::error::RenderError;

impl CanvasTarget {
    /// What it holds on the GPU now: the canvas itself (its texture, the in-progress stroke's
    /// textures, the evaluator's lists, the scene's ops), its snapshots and its arena. View tiers
    /// and pictures are not its own.
    #[must_use]
    pub fn bytes(&self) -> CacheBytes {
        let [width, height] = self.size();
        let texture = u64::from(width) * u64::from(height) * 4;
        let [scene, preview] = self.scene.as_ref().map_or([0, 0], |cache| cache.bytes());
        let held = self.held.as_ref().map_or(0, ActiveSnapshot::bytes);
        CacheBytes {
            canvas: texture + self.bindings.bytes() + self.view.size() + scene,
            snapshots: held + preview,
            arena: self.smudges.bytes() + self.filters.bytes(),
            ..CacheBytes::default()
        }
    }

    /// Its snapshots grow to `room` bytes at most from now on; when they hold more, they are let
    /// go now. Returns the bytes given back.
    pub fn set_snapshot_room(&mut self, room: u64) -> u64 {
        self.bindings.snapshot_room = room;
        let held = self.bytes().snapshots;
        if held <= room {
            return 0;
        }
        self.held = None;
        if let Some(cache) = self.scene.as_mut() {
            cache.drop_snapshot();
        }
        held
    }

    /// Keeps in its arena only the blocks the last scene shows, in a buffer just large enough,
    /// and lets the material texture go; returns the bytes given back.
    ///
    /// # Errors
    /// `Admission` when the blocks shown pass what the device binds (they fit before).
    pub fn shrink_arena(&mut self, core: &Core) -> Result<u64, RenderError> {
        let Some(cache) = self.scene.as_ref() else {
            return Ok(0);
        };
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        smudges.shrink(core, (&cache.scene, &cache.ops), filters)
    }
}
