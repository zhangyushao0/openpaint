//! The raster core's slot passes (S63, DECISIONS D90; `drawing.rs`): a drawing keeps, per output
//! tile, a snapshot of the region's float targets as the units before the next stroke leave them
//! and, while a stroke is drawn, its coverage so far, in slots of pages of its own. A region's
//! targets are set from the snapshots of the tiles it meets instead of drawing those units
//! (`Restore`), a snapshot is taken from the targets after drawing them (`Keep`), the stroke's
//! kept coverage is copied into its atlas region (cleared first) before its tail is laid there
//! (`Kept`), and the spans that settled are laid into the kept coverage's slots (`Cover`, a page
//! cleared first when its slots were freed): copies of float texels and blending in draw order,
//! so every value is the one a region drawn whole from the paper holds.

use super::frame::{begin, clear, coverage};
use super::{RasterTargets, Target};
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{AtlasCopy, CoverageDraw, SlotAt, SlotCopy};

/// The pages a plan's slot passes read and write: a drawing's snapshot pages (the float targets'
/// format) and its coverage pages (the atlas's format, as large as the atlas).
#[derive(Clone, Copy, Default)]
pub(crate) struct Pages<'a> {
    pub(crate) snapshots: &'a [Target],
    pub(crate) coverage: &'a [Target],
}

impl Pages<'_> {
    /// No pages: a plan without slot passes.
    pub(crate) const NONE: Self = Self {
        snapshots: &[],
        coverage: &[],
    };
}

impl RasterTargets {
    /// The scratch targets `copies` name set from their snapshot slots.
    pub(super) fn restore(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        copies: &[SlotCopy],
        pages: Pages<'_>,
    ) {
        for copy in copies {
            let (Some(target), Some(page)) = (
                self.scratch[copy.target as usize].as_ref(),
                pages.snapshots.get(copy.slot.page),
            ) else {
                continue;
            };
            let from = place(&page.texture, slot_pixel(copy.slot, copy.offset));
            encoder.copy_texture_to_texture(
                from,
                place(&target.texture, copy.at),
                extent(copy.size),
            );
        }
    }

    /// The scratch targets `copies` name kept into their snapshot slots.
    pub(super) fn keep(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        copies: &[SlotCopy],
        pages: Pages<'_>,
    ) {
        for copy in copies {
            let (Some(target), Some(page)) = (
                self.scratch[copy.target as usize].as_ref(),
                pages.snapshots.get(copy.slot.page),
            ) else {
                continue;
            };
            let to = place(&page.texture, slot_pixel(copy.slot, copy.offset));
            let from = place(&target.texture, copy.at);
            encoder.copy_texture_to_texture(from, to, extent(copy.size));
        }
    }

    /// The atlas cleared, a stroke's kept coverage copied into its regions there (`copies`; what
    /// no copy covers holds nothing), then its tail `draws` laid there over it.
    pub(super) fn kept(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RasterPipelines,
        (copies, draws): (&[AtlasCopy], &[CoverageDraw]),
        pages: Pages<'_>,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        drop(begin(encoder, &self.atlas.view, clear(None), None));
        for copy in copies {
            let Some(page) = pages.coverage.get(copy.slot.page) else {
                continue;
            };
            let from = place(&page.texture, slot_pixel(copy.slot, copy.offset));
            let to = place(&self.atlas.texture, copy.to);
            encoder.copy_texture_to_texture(from, to, extent(copy.size));
        }
        let mut pass = begin(encoder, &self.atlas.view, wgpu::LoadOp::Load, timestamps);
        self.bind(&mut pass, None);
        coverage(&mut pass, pipelines, draws);
    }

    /// Spans' pieces and stamps `draws` laid into coverage page `page`'s slots (their scissors),
    /// the page cleared first when `cleared` (its slots were freed since it was laid into).
    pub(super) fn cover(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RasterPipelines,
        (page, cleared, draws): (usize, bool, &[CoverageDraw]),
        pages: Pages<'_>,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let Some(page) = pages.coverage.get(page) else {
            return;
        };
        let load = if cleared {
            clear(None)
        } else {
            wgpu::LoadOp::Load
        };
        let mut pass = begin(encoder, &page.view, load, timestamps);
        self.bind(&mut pass, None);
        coverage(&mut pass, pipelines, draws);
    }
}

/// A slot's pixel `offset` from its top-left, in its page.
fn slot_pixel(slot: SlotAt, offset: [u32; 2]) -> [u32; 2] {
    [slot.at[0] + offset[0], slot.at[1] + offset[1]]
}

fn place(texture: &wgpu::Texture, [x, y]: [u32; 2]) -> wgpu::TexelCopyTextureInfo<'_> {
    wgpu::TexelCopyTextureInfo {
        origin: wgpu::Origin3d { x, y, z: 0 },
        ..texture.as_image_copy()
    }
}

fn extent([width, height]: [u32; 2]) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}
