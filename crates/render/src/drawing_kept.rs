//! The kept coverage of the stroke being drawn (`drawing_frame.rs`):
//! every frame notes the output tiles its spans reach (two slots each, taken when they reach a
//! tile first) and which 32-px cells of them the spans settled since reach; the frame's regions
//! lay those spans over what the slots hold and copy the result back into them
//! (`raster_plan_kept.rs`), so after the frame the slots hold those cells too.

use super::{CanvasTarget, Onto, Pen};
use crate::core::Core;
use crate::drawing::Drawing;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::view_tiles::{TIER_TILE, meeting};

impl CanvasTarget {
    /// The tiles `onto` draws that the stroke's spans reach given their kept coverage, and the
    /// cells the spans settled since reach in each; the cells the last frame laid are held now.
    pub(super) fn note_kept(
        &mut self,
        core: &Core,
        drawing: &mut Drawing,
        (pen, onto): (&Pen, &Onto<'_>),
    ) -> Result<(), RenderError> {
        if pen.smudge.is_some() {
            return Ok(());
        }
        for kept in drawing.kept.values_mut() {
            kept.held |= std::mem::take(&mut kept.laid);
        }
        let atlas = self.raster_atlas(core)?;
        let format = core.raster_pipelines()?.atlas;
        let output = Output::at(onto.scale, [0, 0])?;
        let size = self.size().map(|side| side * onto.scale);
        for (index, (_, _, bounds)) in pen.settled.iter().chain(&pen.tail).enumerate() {
            for (tile, part) in meeting(output.place(&bounds.pixels()), size) {
                if bounds.is_empty() || (onto.place)(tile).is_none() {
                    continue;
                }
                let kept = drawing.kept(core, tile, (format, atlas))?;
                if index < pen.settled.len() {
                    kept.laid |= cells(&part, tile.map(|at| at * TIER_TILE));
                }
            }
        }
        Ok(())
    }
}

/// The 32-px cells of the output tile at `origin` that output pixels `part` (inside it) meet.
fn cells(part: &PixelRect, origin: [u32; 2]) -> u64 {
    let cell = 32;
    let [x0, y0] = [0, 1].map(|axis| (part.min[axis] - origin[axis] as i32) / cell);
    let [x1, y1] = [0, 1].map(|axis| (part.max[axis] - origin[axis] as i32 - 1) / cell);
    let mut cells = 0;
    for y in y0.max(0)..=y1.min(7) {
        for x in x0.max(0)..=x1.min(7) {
            cells |= 1 << (y * 8 + x);
        }
    }
    cells
}
