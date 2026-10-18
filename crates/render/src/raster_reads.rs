//! What a raster region's draws read (group 1, `raster.rs`): three float textures, by the pass
//! drawing them. A stroke's tiles read the atlas; a layer's end the layer's paint (or the chain, a
//! level, the group's backdrop) with the mask and the backdrop; a smudge, an adjustment or a filter
//! the copy it rewrites from; the finish the canvas. S65: the targets read vary with the levels
//! and the group a region uses, so each bind group is made the first time a region reads through
//! it, and all go when a scratch target is made.

use super::{RasterTargets, Region};
use crate::core::Core;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{Pass, Scratch};

/// What one texture of a draw's group reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Read {
    Atlas,
    Scratch(Scratch),
    /// A texture of one texel, where a draw reads nothing.
    Unread,
}

const UNREAD: Read = Read::Unread;

/// A stroke's tiles: the atlas.
pub(super) const ATLAS: [Read; 3] = [Read::Atlas, UNREAD, UNREAD];
/// A rewrite: the copy in the backdrop.
pub(super) const COPIED: [Read; 3] = [Read::Scratch(Scratch::Backdrop), UNREAD, UNREAD];
/// The finish: the canvas.
pub(super) const CANVAS: [Read; 3] = [Read::Scratch(Scratch::Canvas), UNREAD, UNREAD];

/// An end reading `reads` with the mask and the backdrop.
pub(super) const fn ending(reads: Scratch) -> [Read; 3] {
    [
        Read::Scratch(reads),
        Read::Scratch(Scratch::Mask),
        Read::Scratch(Scratch::Backdrop),
    ]
}

/// What `pass` reads through group 1, when it draws.
fn read_by(pass: &Pass) -> Option<[Read; 3]> {
    match pass {
        Pass::Draw { .. } => Some(ATLAS),
        Pass::Rewrite { .. } => Some(COPIED),
        Pass::End { reads, .. } => Some(ending(*reads)),
        _ => None,
    }
}

/// The bind groups made so far, by what they read.
#[derive(Default)]
pub(super) struct Reads {
    made: Vec<([Read; 3], wgpu::BindGroup)>,
}

impl Reads {
    pub(super) fn get(&self, key: [Read; 3]) -> Option<&wgpu::BindGroup> {
        (self.made.iter()).find_map(|(made, group)| (*made == key).then_some(group))
    }
}

impl RasterTargets {
    /// Every bind group `regions` read through, made when it is not.
    pub(super) fn make_reads(
        &mut self,
        core: &Core,
        pipelines: &RasterPipelines,
        regions: &[Region<'_>],
    ) {
        for region in regions {
            let passes = region.plan.passes.iter().filter_map(read_by);
            let finish = region.finish.map(|_| CANVAS);
            for key in passes.chain(finish) {
                if self.reads.get(key).is_none() {
                    let group = self.read_group(core, pipelines, key);
                    self.reads.made.push((key, group));
                }
            }
        }
    }

    fn read_group(
        &self,
        core: &Core,
        pipelines: &RasterPipelines,
        key: [Read; 3],
    ) -> wgpu::BindGroup {
        let views = key.map(|read| match read {
            Read::Atlas => &self.atlas.view,
            Read::Scratch(at) => self.view(at),
            Read::Unread => &self.unread.view,
        });
        let entries: Vec<wgpu::BindGroupEntry<'_>> = (0..)
            .zip(views)
            .map(|(binding, view)| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        core.gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raster reads"),
                layout: &pipelines.layouts[1],
                entries: &entries,
            })
    }

    /// Scratch target `at`'s view, or the unread texel's while it is not made.
    pub(super) fn view(&self, at: Scratch) -> &wgpu::TextureView {
        &self.scratch[at.index()]
            .as_ref()
            .unwrap_or(&self.unread)
            .view
    }
}
