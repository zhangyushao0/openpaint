//! Snapshots taken ahead (S63, DECISIONS D90; `drawing.rs`): while nothing touches the screen and
//! nothing of the canvas waits, the raster worker takes the snapshots the 1x canvas's tiles lack
//! (and, once its tiles are ready, those of the view tier's shown tiles), or lays the strokes
//! committed since onto those that lack them, the tiles the screen shows nearest its middle first,
//! a few at a time, each into slots the drawing holds for it until it is answered (D87's pacing:
//! dropped unsubmitted once a hand arrives), within the snapshots' room in the GPU memory budget.
//! An answer is kept unless a change reached the tile, or the frame loop brought its snapshot up
//! first; so the first frames of a stroke find what lies under it and take no snapshot. What a
//! stroke's frames draw with (a coverage page, the scratch targets) is made ahead too.

use std::sync::Arc;

use super::{CanvasTarget, paper};
use crate::core::Core;
use crate::drawing::{Drawing, Taking, slot_copies, tile_rect};
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::output::Output;
use crate::raster::SceneBuffers;
use crate::raster_plan::{Scratch, SlotAt, SlotCopy};
use crate::smudge::around_output;
use crate::view_raster_jobs::{JobInto, RasterJob, RasterOrder, SnapshotJob};
use crate::view_tiles::TIER_TILE;

/// Snapshots with the raster worker at most, per drawing.
pub(crate) const TAKING: usize = 4;

impl CanvasTarget {
    /// Hands the worker the next snapshots the canvas's tiles want, while nothing touches the
    /// screen and no stroke is drawn.
    pub(super) fn snapshots_ahead(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<(), RenderError> {
        if !core.pace().idle() || self.drawing.tracking() || self.drawing.base.is_none() {
            return Ok(());
        }
        let mut drawing = std::mem::take(&mut self.drawing);
        let made = self.made_ahead(core, &mut drawing);
        let wanted = self.wanted_snapshots(&drawing);
        let others = self.bytes().snapshots;
        let handed = made
            .and_then(|()| self.hand_snapshots(core, request, &mut drawing, (1, &wanted, others)));
        self.drawing = drawing;
        handed
    }

    /// A coverage page of `drawing`'s and every scratch target made now, when they are not.
    pub(crate) fn made_ahead(
        &mut self,
        core: &Core,
        drawing: &mut Drawing,
    ) -> Result<(), RenderError> {
        let atlas = self.raster_atlas(core)?;
        let format = core.raster_pipelines()?.atlas;
        drawing.coverage_ahead(core, (format, atlas))?;
        let group = (drawing.base.as_ref()).is_some_and(|base| base.restored.group);
        match self.raster.targets.as_mut() {
            Some(targets) => targets.make_every(core, group),
            None => Ok(()),
        }
    }

    /// Hands the worker snapshots of `drawing`'s output tiles `tiles` (an output at `scale`), in
    /// order while the snapshots' room holds them besides `others` bytes and the residuals they
    /// read are present.
    pub(crate) fn hand_snapshots(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        drawing: &mut Drawing,
        (scale, tiles, others): (u32, &[[u32; 2]], u64),
    ) -> Result<(), RenderError> {
        let Some(base) = drawing.base.clone() else {
            return Ok(());
        };
        let size = self.size().map(|side| side * scale);
        for &tile in tiles {
            let rect = tile_rect(tile, size);
            let near = [around_output(&rect, scale)];
            if !self.room_for(drawing, others) || !self.residuals_ready(core, request, &near)? {
                break;
            }
            self.reach(core, request.artwork, &near)?;
            let missing = || RenderError::Invalid("no scene was built");
            let held = self.held().ok_or_else(missing)?;
            let cache = self.scene.as_ref().ok_or_else(missing)?;
            let format = core.raster_pipelines()?.layer;
            let slots = drawing.snapshot_slots(core, (&base.restored.targets(), format))?;
            let side = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]).unsigned_abs());
            let under = drawing.under.get(&tile).cloned();
            let snapshot = SnapshotJob {
                from: (under.as_ref()).map(|under| (under.at, copies(&under.slots, side))),
                to: base.at,
                restored: base.restored,
                keep: copies(&slots, side),
                pages: drawing.snapshot_pages(),
            };
            let id = self.raster.next_id();
            let job = RasterJob {
                id,
                scene: Arc::clone(&cache.scene),
                buffers: SceneBuffers::of(&cache.ops, held),
                output: Output::at(scale, rect.min.map(i32::unsigned_abs))?,
                size: side,
                paper: paper(request),
                into: JobInto::Snapshot(Box::new(snapshot)),
            };
            self.raster.orders.push(RasterOrder::Draw(Box::new(job)));
            drawing.taking.push(Taking {
                id,
                tile,
                from: under.map(|under| under.made),
                epoch: base.epoch,
                slots,
                at: base.at,
            });
        }
        Ok(())
    }

    /// The tiles whose snapshot `drawing` (the canvas's) wants taken next, as many as may be
    /// taken at once: those the screen shows first, nearest its middle (rows from the top
    /// without a view).
    fn wanted_snapshots(&self, drawing: &Drawing) -> Vec<[u32; 2]> {
        let [columns, rows] = self.size().map(|side| side.div_ceil(TIER_TILE));
        let all = (0..rows).flat_map(|row| (0..columns).map(move |column| [column, row]));
        let mut wanted = drawing.wanted(all, usize::MAX);
        if let Some(focus) = self.focus {
            let key = |tile: &[u32; 2]| {
                let rect = tile_rect(*tile, self.size());
                let middle = [0, 1].map(|axis| (rect.min[axis] + rect.max[axis]) as f32 / 2.0);
                let [dx, dy] = [0, 1].map(|axis| middle[axis] - focus.middle[axis]);
                let hidden = rect.intersect(&focus.shown).is_empty();
                (hidden, (dx * dx + dy * dy) as u64)
            };
            wanted.sort_by_cached_key(key);
        }
        wanted.truncate(TAKING.saturating_sub(drawing.taking.len()));
        wanted
    }

    /// Whether the canvas's snapshots under the next stroke are being taken ahead or wait to be
    /// (within the snapshots' room): frames are wanted until none is (`CanvasGpu` asks the view
    /// tier's too).
    #[must_use]
    pub fn taking_ahead(&self) -> bool {
        let drawing = &self.drawing;
        let others = self.bytes().snapshots - drawing.bytes();
        drawing.base.is_some()
            && (!drawing.taking.is_empty()
                || (self.room_for(drawing, others) && !self.wanted_snapshots(drawing).is_empty()))
    }

    /// One more snapshot of `drawing`'s keeps the snapshots within the room the GPU memory budget
    /// gives them, `others` bytes held besides.
    pub(crate) fn room_for(&self, drawing: &Drawing, others: u64) -> bool {
        let targets = (drawing.base.as_ref()).map_or(0, |base| base.restored.targets().len());
        let more = drawing.bytes_for(targets);
        others + drawing.bytes() + more <= self.snapshot_room
    }
}

/// A snapshot's targets as slot copies of a whole tile of `size` pixels.
fn copies(slots: &[(Scratch, SlotAt)], size: [u32; 2]) -> Vec<SlotCopy> {
    slot_copies(slots, ([0, 0], [0, 0], size))
}
