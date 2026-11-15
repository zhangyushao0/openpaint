//! A drawing's kept coverage slots (`drawing/`): per output tile the
//! stroke being drawn reaches, two slots of atlas-sized coverage pages (its coverage or density,
//! and a pencil's contact), taken when the stroke reaches the tile first and set to nothing by
//! that frame (`take_taken`); each page with a bind group the atlas pass reads it through, and a
//! tile of nothing in its format to set slots from.

use super::{Drawing, Kept, Target};
use crate::core::Core;
use crate::error::RenderError;
use crate::raster_plan::SlotAt;
use crate::readback::target_texture;
use crate::view_tiles::TIER_TILE;

impl Drawing {
    /// Tile `tile`'s kept coverage, two slots of `atlas`-sized pages of `format` taken when it has
    /// none (holding no kept spans; the frame sets them to nothing first: `take_taken`).
    pub(crate) fn kept(
        &mut self,
        core: &Core,
        tile: [u32; 2],
        size: (wgpu::TextureFormat, [u32; 2]),
    ) -> Result<&mut Kept, RenderError> {
        if !self.kept.contains_key(&tile) {
            let slots = [
                self.coverage_slot(core, size)?,
                self.coverage_slot(core, size)?,
            ];
            self.taken.extend(slots);
            let none = Kept {
                slots,
                held: 0,
                laid: 0,
            };
            self.kept.insert(tile, none);
        }
        (self.kept.get_mut(&tile)).ok_or(RenderError::Invalid("a kept tile just taken"))
    }

    /// The coverage slots taken since the last call: a frame sets them to nothing first.
    pub(crate) fn take_taken(&mut self) -> Vec<SlotAt> {
        std::mem::take(&mut self.taken)
    }

    /// A coverage page of `atlas` pixels and `format` made now when there is none (ahead,
    /// so a stroke's first frame allocates nothing: making one took up to 20 ms on the tablet).
    pub(crate) fn coverage_ahead(
        &mut self,
        core: &Core,
        size: (wgpu::TextureFormat, [u32; 2]),
    ) -> Result<(), RenderError> {
        if self.coverage.pages.is_empty() {
            let slot = self.coverage_slot(core, size)?;
            self.coverage.free(slot);
        }
        Ok(())
    }

    /// A free coverage slot, on a page of `atlas` pixels and `format` made (with the bind group
    /// reading it) when none is left.
    fn coverage_slot(
        &mut self,
        core: &Core,
        (format, atlas): (wgpu::TextureFormat, [u32; 2]),
    ) -> Result<SlotAt, RenderError> {
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let slot = self.coverage.take(core, (atlas, format, usage))?;
        if self.nothing.is_none() {
            let tile = ([TIER_TILE; 2], format);
            self.nothing = Some(Target::of(target_texture(&core.gpu, tile, usage)?));
        }
        let layout = &core.raster_pipelines()?.layouts[1];
        for page in &self.coverage.pages[self.reads.len()..] {
            let read = wgpu::BindingResource::TextureView(&page.view);
            let entries: Vec<wgpu::BindGroupEntry<'_>> = (0..3)
                .map(|binding| wgpu::BindGroupEntry {
                    binding,
                    resource: read.clone(),
                })
                .collect();
            let group = core
                .gpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kept coverage"),
                    layout,
                    entries: &entries,
                });
            self.reads.push(group);
        }
        Ok(slot)
    }
}
