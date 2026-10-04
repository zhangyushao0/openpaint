//! `Plan`: a scene binned into the tiles of the regions being evaluated. Each tile lists, in
//! scene order, only the ops that can reach it. An op that cannot reach a pixel leaves it
//! exactly unchanged (zero coverage, or compositing nothing), so a pixel's value never depends
//! on which regions are evaluated together: incremental evaluation equals cold evaluation.

use std::ops::Range;

use crate::affine::Affine;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::protocol::{FRESH, TILE, TileRecord};
use crate::scene::{Fresh, Scene};
use crate::view_tiles::TileMap;

#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub(crate) tiles: Vec<TileRecord>,
    /// Scene item indices, or fresh op indices with the `FRESH` bit; each tile owns
    /// `start..start + count`.
    pub(crate) indices: Vec<u32>,
    /// The most moved selections any tile sets aside at once (`MOVE_DEPTH` at most).
    pub(crate) deepest: u32,
}

impl Plan {
    /// Regions (target pixels) are clipped to the target of `size` pixels, which lies at
    /// `output`, and widened to whole tiles; ops reach the tiles their canvas bounds cover
    /// there. At the scene's `Active` place every tile lists the fresh segments that reach it
    /// and then the fresh paint op: the in-progress stroke's earlier coverage may lie in any
    /// tile.
    pub(crate) fn build(
        scene: &Scene,
        fresh: &Fresh,
        (size, output): ([u32; 2], Output),
        regions: &[PixelRect],
    ) -> Self {
        Self::laid_out(scene, fresh, (size, Layout::Linear(output)), regions)
    }

    /// As `build`, for a target laid out as `layout` (a view tier's tiles, S32).
    pub(crate) fn laid_out(
        scene: &Scene,
        fresh: &Fresh,
        (size, layout): ([u32; 2], Layout<'_>),
        regions: &[PixelRect],
    ) -> Self {
        let grid = Grid::new(size, layout, regions);
        let (lists, deepest) = Binner::new(&grid, None).bin(&scene.items, 0, fresh);
        Self::flatten(&grid, lists, deepest)
    }

    /// One layer's pixels before a smudge: the scene's `items` (that layer's ops before it),
    /// then in every tile the evaluation's fresh op 0, which ends the layer at full opacity.
    /// The smudge's pixels lie on the canvas by `to_canvas` (under transform records), so the
    /// target's pixels are those and the items' bounds are mapped back to them.
    pub(crate) fn material(
        scene: &Scene,
        (items, to_canvas): (Range<usize>, Affine),
        size: [u32; 2],
        regions: &[PixelRect],
    ) -> Self {
        let first = u32::try_from(items.start).unwrap_or(u32::MAX);
        let layout = Layout::Linear(Output::CANVAS);
        let grid = Grid::new(size, layout, regions);
        let view = (!to_canvas.is_identity()).then(|| to_canvas.inverse());
        let binner = Binner::new(&grid, view);
        let (mut lists, deepest) = binner.bin(&scene.items[items], first, &Fresh::default());
        for list in &mut lists {
            list.push(FRESH);
        }
        Self::flatten(&grid, lists, deepest)
    }

    fn flatten(grid: &Grid, lists: Vec<Vec<u32>>, deepest: u32) -> Self {
        let mut plan = Self {
            deepest,
            ..Self::default()
        };
        for ((origin, output), list) in grid.origins.iter().zip(&grid.outputs).zip(lists) {
            let start = u32::try_from(plan.indices.len()).unwrap_or(u32::MAX);
            let count = u32::try_from(list.len()).unwrap_or(u32::MAX);
            plan.tiles.push(TileRecord {
                origin: *origin,
                output: *output,
                start,
                count,
            });
            plan.indices.extend(list);
        }
        plan
    }
}

/// How a target's pixels hold output pixels.
#[derive(Clone, Copy)]
pub(crate) enum Layout<'a> {
    /// One rectangle of the output: the target's pixel `p` holds output pixel `origin + p`
    /// (the canvas, an export's piece).
    Linear(Output),
    /// A view tier's tiles, each in a slot of the target (S32).
    Tiles(&'a TileMap),
}

/// The target's tile grid, how it lies in the output, and which tiles are being evaluated (a
/// dense slot each).
struct Grid<'a> {
    columns: usize,
    rows: usize,
    layout: Layout<'a>,
    /// Per grid tile, row major: its slot, or `None` when not evaluated.
    slot_of: Vec<Option<usize>>,
    /// Per slot: the tile's top-left pixel, and the output pixel it holds.
    origins: Vec<[u32; 2]>,
    outputs: Vec<[u32; 2]>,
}

impl<'a> Grid<'a> {
    fn new(size: [u32; 2], layout: Layout<'a>, regions: &[PixelRect]) -> Self {
        let columns = size[0].div_ceil(TILE) as usize;
        let rows = size[1].div_ceil(TILE) as usize;
        let mut grid = Self {
            columns,
            rows,
            layout,
            slot_of: vec![None; columns * rows],
            origins: Vec::new(),
            outputs: Vec::new(),
        };
        for region in regions {
            for tile in grid.tiles(region) {
                if grid.slot_of[tile].is_none() {
                    grid.slot_of[tile] = Some(grid.origins.len());
                    let (column, row) = (tile % columns, tile / columns);
                    let origin = [column as u32 * TILE, row as u32 * TILE];
                    grid.origins.push(origin);
                    grid.outputs.push(match layout {
                        Layout::Linear(output) => {
                            [0, 1].map(|axis| output.origin[axis] + origin[axis])
                        }
                        Layout::Tiles(map) => map.output_of(origin),
                    });
                }
            }
        }
        grid
    }

    /// Grid tiles (row-major indices) that meet `rect`.
    fn tiles(&self, rect: &PixelRect) -> impl Iterator<Item = usize> + use<> {
        let span = |min: i32, max: i32, count: usize| {
            if max <= min {
                return 0..0;
            }
            let tile = i64::from(TILE);
            let first = (i64::from(min).max(0) / tile) as usize;
            let end = (i64::from(max).max(0) + tile - 1) / tile;
            first..(end as usize).min(count)
        };
        let columns = span(rect.min[0], rect.max[0], self.columns);
        let rows = span(rect.min[1], rect.max[1], self.rows);
        let width = self.columns;
        rows.flat_map(move |row| columns.clone().map(move |column| row * width + column))
    }

    /// Slots of the evaluated tiles that meet the canvas pixels `rect`.
    fn slots(&self, rect: &PixelRect) -> Vec<usize> {
        let mut slots = Vec::new();
        let mut add = |target: &PixelRect| {
            slots.extend(self.tiles(target).filter_map(|tile| self.slot_of[tile]));
        };
        match self.layout {
            Layout::Linear(output) => add(&output.place(rect)),
            Layout::Tiles(map) => map.targets(&map.place(rect), &mut add),
        }
        slots
    }

    /// Per slot, the fresh segments that reach it, as `FRESH` indices in order.
    fn bin(&self, fresh: &Fresh) -> Vec<Vec<u32>> {
        let mut lists = vec![Vec::new(); self.origins.len()];
        for (index, bounds) in (1..).zip(&fresh.bounds) {
            for slot in self.slots(&bounds.pixels()) {
                lists[slot].push(FRESH | index);
            }
        }
        lists
    }
}

/// The slots one stroke, layer or move has reached so far, each listed once.
struct Touched {
    /// Per slot, the epoch it was last touched in.
    marks: Vec<u32>,
    slots: Vec<usize>,
    epoch: u32,
}

impl Touched {
    fn new(slots: usize) -> Self {
        Self {
            marks: vec![0; slots],
            slots: Vec::new(),
            epoch: 1,
        }
    }

    fn touch(&mut self, slot: usize) {
        if self.marks[slot] != self.epoch {
            self.marks[slot] = self.epoch;
            self.slots.push(slot);
        }
    }

    fn has(&self, slot: usize) -> bool {
        self.marks[slot] == self.epoch
    }

    /// The touched slots in first-touch order; starts a new epoch.
    fn close(&mut self) -> Vec<usize> {
        self.epoch += 1;
        std::mem::take(&mut self.slots)
    }
}

#[path = "plan_bin.rs"]
mod bin;
use bin::Binner;

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
