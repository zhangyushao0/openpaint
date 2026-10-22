//! The raster core's slot passes (`canvas/drawing.rs`): a drawing keeps, per
//! output tile, a snapshot of the region's float targets as the units before the next stroke
//! leave them and, while a stroke is drawn, its coverage so far, in slots of pages of its own. A
//! region's targets are set from the snapshots of the tiles it meets instead of drawing those
//! units (`Restore`), a snapshot is taken from the targets after drawing them (`Keep`), and the
//! stroke is laid in one pass of the atlas (`Kept`): its kept coverage read from the slots into
//! its regions (cleared first), the spans settled since and its tail laid over it, then the
//! regions holding the kept coverage and the settled spans copied back into the slots. Copies of
//! float texels and blending in draw order, so every value is the one a region drawn whole from
//! the paper holds.

use std::ops::Range;

use super::frame::{begin, clear, coverage};
use super::{RasterTargets, Target};
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{AtlasCopy, CoverageDraw, SlotAt, SlotCopy};
use crate::readback::{extent, place};

/// The pages a plan's slot passes read and write: a drawing's snapshot pages (the float targets'
/// format) and its coverage pages (the atlas's format, as large as the atlas) with the bind group
/// reading each, and a tile of nothing in the atlas's format.
#[derive(Clone, Copy, Default)]
pub(crate) struct Pages<'a> {
    pub(crate) snapshots: &'a [Target],
    pub(crate) coverage: &'a [Target],
    pub(crate) reads: &'a [wgpu::BindGroup],
    pub(crate) nothing: Option<&'a Target>,
}

/// What `Pass::Kept` draws and copies: per page its kept coverage's instances, the spans' draws,
/// the copies back into the slots.
type KeptPass<'a> = (
    &'a [(usize, Range<u32>)],
    &'a [CoverageDraw],
    &'a [AtlasCopy],
);

impl Pages<'_> {
    /// No pages: a plan without slot passes.
    pub(crate) const NONE: Self = Self {
        snapshots: &[],
        coverage: &[],
        reads: &[],
        nothing: None,
    };
}

impl RasterTargets {
    /// The scratch targets `copies` name set from their snapshot slots (`restore`), or kept into
    /// them.
    pub(super) fn snapshot_copies(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        (copies, restore): (&[SlotCopy], bool),
        pages: Pages<'_>,
    ) {
        for copy in copies {
            let (Some(target), Some(page)) = (
                self.scratch[copy.target.index()].as_ref(),
                pages.snapshots.get(copy.slot.page),
            ) else {
                continue;
            };
            let slot = place(&page.texture, slot_pixel(copy.slot, copy.offset));
            let region = place(&target.texture, copy.at);
            let (from, to) = if restore {
                (slot, region)
            } else {
                (region, slot)
            };
            encoder.copy_texture_to_texture(from, to, extent(copy.size));
        }
    }

    /// One pass of the atlas, cleared first: the kept coverage `held` (per page, its instances)
    /// read from the slots, then `draws` over it; then the atlas's regions `keep` copied into the
    /// slots.
    pub(super) fn kept(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RasterPipelines,
        (held, draws, keep): KeptPass<'_>,
        pages: Pages<'_>,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = begin(encoder, &self.atlas.view, clear(None), timestamps);
        self.bind(&mut pass, None);
        let atlas = self.atlas();
        pass.set_scissor_rect(0, 0, atlas[0], atlas[1]);
        pass.set_pipeline(&pipelines.kept);
        for (page, instances) in held {
            let Some(reads) = pages.reads.get(*page) else {
                continue;
            };
            pass.set_bind_group(1, reads, &[]);
            pass.draw(0..4, instances.clone());
        }
        coverage(&mut pass, pipelines, draws);
        drop(pass);
        for copy in keep {
            let Some(page) = pages.coverage.get(copy.slot.page) else {
                continue;
            };
            let from = place(&self.atlas.texture, copy.at);
            let to = place(&page.texture, slot_pixel(copy.slot, copy.offset));
            encoder.copy_texture_to_texture(from, to, extent(copy.size));
        }
    }
}

/// Kept coverage slots `slots` set to nothing.
pub(super) fn zero(encoder: &mut wgpu::CommandEncoder, slots: &[SlotAt], pages: Pages<'_>) {
    let Some(nothing) = pages.nothing else {
        return;
    };
    for slot in slots {
        let Some(page) = pages.coverage.get(slot.page) else {
            continue;
        };
        let to = place(&page.texture, slot.at);
        let size = [nothing.texture.width(), nothing.texture.height()];
        encoder.copy_texture_to_texture(nothing.texture.as_image_copy(), to, extent(size));
    }
}

/// A slot's pixel `offset` from its top-left, in its page.
fn slot_pixel(slot: SlotAt, offset: [u32; 2]) -> [u32; 2] {
    [slot.at[0] + offset[0], slot.at[1] + offset[1]]
}
