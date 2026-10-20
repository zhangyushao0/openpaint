//! A canvas target's GPU memory (S49, DECISIONS D76): what its derived caches hold, and the
//! budget's steps on it: its snapshots kept within a room (beyond it a stroke draws from the
//! layer's start), its arena cut down to the blocks the scene shows, and (S51) its residuals
//! within a room, with the residuals a draw reads made present near what it draws. None changes a
//! pixel: each is made again from the artwork when needed.

use openpaint_core::{Artwork, StrokeTool};

use super::CanvasTarget;
use crate::budget::CacheBytes;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::smudge::{Held, Wanted};
use crate::stroke::stroke_region;

impl CanvasTarget {
    /// What it holds on the GPU now: the canvas itself (its texture and view, the scene's ops),
    /// its snapshots and its arena. Its raster targets, view tiers and pictures are not its own.
    #[must_use]
    pub fn bytes(&self) -> CacheBytes {
        let [width, height] = self.size();
        let texture = u64::from(width) * u64::from(height) * 4;
        let scene = self.scene.as_ref().map_or(0, |cache| cache.bytes());
        CacheBytes {
            canvas: texture + self.view.size() + scene,
            snapshots: self.drawing.bytes(),
            arena: self.smudges.bytes() + self.filters.bytes(),
            ..CacheBytes::default()
        }
    }

    /// Heap bytes of the scene it keeps between frames (items, mask words): what the harness's
    /// long session reports (S51).
    #[must_use]
    pub fn heap_bytes(&self) -> u64 {
        self.scene.as_ref().map_or(0, |cache| cache.heap_bytes())
    }

    /// Its snapshots grow to `room` bytes at most from now on; when they hold more, they are let
    /// go now (a stroke's drawing only once no stroke is drawn, S63). Returns the bytes given
    /// back.
    pub fn set_snapshot_room(&mut self, room: u64) -> u64 {
        self.snapshot_room = room;
        if self.bytes().snapshots <= room {
            return 0;
        }
        self.drawing.let_go()
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
        smudges.shrink(core, &cache.scene, (filters, true))
    }

    /// Its arena binds at most `bytes` from now on, as on a device whose storage binding limit
    /// that is (tests: the tablet binds 128 MiB): residuals are then kept in part, near what is
    /// evaluated (S51).
    pub fn set_arena_limit(&mut self, bytes: u64) {
        self.smudges.set_limit(bytes);
    }

    /// The budget's step on its arena at the start of a frame (S51): it keeps within `room` bytes
    /// from now on; when it holds more, the residuals the scene shows go now, the least recently
    /// needed first, never one the last frame needed; the frame's evaluations keep within it as
    /// they can (a residual they need is evaluated again, near what they evaluate), what they
    /// need kept together. Returns the bytes given back.
    ///
    /// # Errors
    /// As `shrink_arena`.
    pub fn set_arena_room(&mut self, core: &Core, room: u64) -> Result<u64, RenderError> {
        self.smudges.set_room(room);
        // Only when a quarter of the room can go: the blocks the last frame needed stay anyway,
        // and moving the arena for less each frame costs more than it gives back.
        let over = self.smudges.arena_bytes() > room && self.smudges.spare_bytes() >= room / 4;
        let freed = match self.scene.as_ref().filter(|_| over) {
            Some(cache) => {
                let (smudges, filters) = (&mut self.smudges, &mut self.filters);
                smudges.shrink(core, &cache.scene, (filters, false))?
            }
            None => 0,
        };
        self.smudges.next_frame();
        Ok(freed)
    }

    /// After an evaluation outside the screen's frames that took every waiting tile at once (a
    /// pick, the gallery picture), its arena back within its room at once (S51): what that
    /// evaluation needed counts as needed before now, the least recently needed going first.
    /// Returns the bytes given back.
    ///
    /// # Errors
    /// As `shrink_arena`.
    pub fn settle_arena(&mut self, core: &Core) -> Result<u64, RenderError> {
        self.smudges.next_frame();
        let room = self.smudges.room();
        self.set_arena_room(core, room)
    }

    /// Makes the residuals read near the canvas pixels `near` (a view tier's tiles, `reached`,
    /// `around_output`) present in the arena, for a draw of the scene the last evaluation drew
    /// (S51).
    ///
    /// # Errors
    /// As `Smudges::prepare`.
    pub(crate) fn reach(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        near: &[PixelRect],
    ) -> Result<(), RenderError> {
        let Some(cache) = self.scene.as_ref() else {
            return Ok(());
        };
        let filters = (&mut self.filters, &Wanted::Near(near));
        let scene = (&*cache.scene, &cache.ops);
        self.smudges.prepare(core, scene, artwork, filters)
    }

    /// The residuals of the scene the last evaluation drew as the arena holds them now, for the
    /// raster core's draws of it (S61, D88).
    pub(crate) fn held(&self) -> Option<Held> {
        let cache = self.scene.as_ref()?;
        Some(self.smudges.held(&cache.scene, &self.filters))
    }

    /// Extends the in-progress smudge `request` draws (`drawn`: its layer shows) by its new
    /// contacts, the earlier residuals near it present first (S51), or ends the one there was;
    /// returns the pixels to evaluate again when its residual started over.
    pub(super) fn smudged(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        drawn: bool,
    ) -> Result<Option<PixelRect>, RenderError> {
        let smudging = request
            .active
            .filter(|active| drawn && active.style().tool() == StrokeTool::Smudge);
        let Some(active) = smudging else {
            self.smudges.end_active();
            return Ok(None);
        };
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let scene = (&*cache.scene, &cache.ops);
        let near: Vec<PixelRect> = stroke_region(active.style(), active.samples(), 0)
            .into_iter()
            .collect();
        let filters = (&mut self.filters, &Wanted::Near(&near));
        self.smudges
            .prepare(core, scene, request.artwork, filters)?;
        let scene = (scene.0, scene.1, &self.filters);
        self.smudges.advance(core, scene, request.artwork, active)
    }
}
