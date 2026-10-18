//! `Plan`: a scene binned into the tiles of the regions being evaluated. Each tile lists, in
//! scene order, only the ops that can reach it. An op that cannot reach a pixel leaves it
//! exactly unchanged (zero coverage, or compositing nothing), so a pixel's value never depends
//! on which regions are evaluated together: incremental evaluation equals cold evaluation.

use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::affine::Affine;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::protocol::{FRESH, TILE, TileRecord};
use crate::scene::Scene;

#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub(crate) tiles: Vec<TileRecord>,
    /// Scene item indices, or fresh op indices with the `FRESH` bit; each tile owns
    /// `start..start + count`.
    pub(crate) indices: Vec<u32>,
    /// The most moved selections any tile sets aside at once (`MOVE_DEPTH` at most).
    pub(crate) deepest: u32,
    /// Names this plan among every plan built, so a dispatch uploads a kept one once (0: none).
    pub(crate) serial: u64,
}

/// The last `Plan::serial` given.
static SERIAL: AtomicU64 = AtomicU64::new(0);

impl Plan {
    /// Regions (target pixels) are clipped to the target of `size` pixels, which lies at
    /// `output`, and widened to whole tiles; ops reach the tiles their canvas bounds cover
    /// there.
    pub(crate) fn build(
        scene: &Scene,
        (size, output): ([u32; 2], Output),
        regions: &[PixelRect],
    ) -> Self {
        let grid = Grid::new(size, output, regions);
        let (lists, deepest) = Binner::new(&grid, None).bin(&scene.items, 0);
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

        let grid = Grid::new(size, Output::CANVAS, regions);
        let view = (!to_canvas.is_identity()).then(|| to_canvas.inverse());
        let binner = Binner::new(&grid, view);
        let (mut lists, deepest) = binner.bin(&scene.items[items], first);
        for list in &mut lists {
            list.push(FRESH);
        }
        Self::flatten(&grid, lists, deepest)
    }

    fn flatten(grid: &Grid, lists: Vec<Vec<u32>>, deepest: u32) -> Self {
        let mut plan = Self {
            deepest,
            serial: SERIAL.fetch_add(1, Ordering::Relaxed) + 1,
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

/// The target's tile grid, how it lies in the output, and which tiles are being evaluated (a
/// dense slot each).
struct Grid {
    columns: usize,
    output: Output,
    /// Per grid tile, row major: its slot, or `None` when not evaluated.
    slot_of: Vec<Option<usize>>,
    /// Per slot: the tile's top-left pixel, and the output pixel it holds.
    origins: Vec<[u32; 2]>,
    outputs: Vec<[u32; 2]>,
    /// The columns and rows the evaluated tiles lie in: an item is looked up only there, so
    /// binning a few tiles costs what reaches them, not what reaches the whole target (S53).
    within: [std::ops::Range<usize>; 2],
    /// The canvas pixels the evaluated tiles' output pixels lie in, widened by one (as `place`
    /// widens), `[x0, y0, x1, y1]`: an item whose canvas bounds miss them is passed over at once.
    reach: [f32; 4],
}

impl Grid {
    fn new(size: [u32; 2], output: Output, regions: &[PixelRect]) -> Self {
        let columns = size[0].div_ceil(TILE) as usize;
        let rows = size[1].div_ceil(TILE) as usize;
        let mut grid = Self {
            columns,
            output,
            slot_of: vec![None; columns * rows],
            origins: Vec::new(),
            outputs: Vec::new(),
            within: [0..columns, 0..rows],
            reach: [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        };
        let mut low = [usize::MAX; 2];
        let mut high = [0; 2];
        for region in regions {
            for tile in grid.tiles(region) {
                if grid.slot_of[tile].is_none() {
                    grid.slot_of[tile] = Some(grid.origins.len());
                    let (column, row) = (tile % columns, tile / columns);
                    low = [low[0].min(column), low[1].min(row)];
                    high = [high[0].max(column + 1), high[1].max(row + 1)];
                    let origin = [column as u32 * TILE, row as u32 * TILE];
                    grid.origins.push(origin);
                    grid.outputs
                        .push([0, 1].map(|axis| output.origin[axis] + origin[axis]));
                }
            }
        }
        grid.within = [low[0]..high[0], low[1]..high[1]];
        let scale = output.scale as f32;
        for output in &grid.outputs {
            let [x, y] = output.map(|at| at as f32 / scale);
            let side = TILE as f32 / scale;
            let [x0, y0, x1, y1] = grid.reach;
            grid.reach = [
                x0.min(x - 1.0),
                y0.min(y - 1.0),
                x1.max(x + side + 1.0),
                y1.max(y + side + 1.0),
            ];
        }
        grid
    }

    /// Whether canvas bounds `b` can reach an evaluated tile at all (a quick test; `slots` says
    /// which).
    fn may_reach(&self, b: &[f32; 4]) -> bool {
        let r = &self.reach;
        b[0] <= r[2] && b[2] >= r[0] && b[1] <= r[3] && b[3] >= r[1]
    }

    /// Grid tiles (row-major indices) that meet `rect`, among the columns and rows `within`.
    fn tiles(&self, rect: &PixelRect) -> impl Iterator<Item = usize> + use<> {
        let span = |min: i32, max: i32, within: &std::ops::Range<usize>| {
            if max <= min {
                return 0..0;
            }
            let tile = i64::from(TILE);
            let first = (i64::from(min).max(0) / tile) as usize;
            let end = (i64::from(max).max(0) + tile - 1) / tile;
            first.max(within.start)..(end as usize).min(within.end)
        };
        let columns = span(rect.min[0], rect.max[0], &self.within[0]);
        let rows = span(rect.min[1], rect.max[1], &self.within[1]);
        let width = self.columns;
        rows.flat_map(move |row| columns.clone().map(move |column| row * width + column))
    }

    /// Slots of the evaluated tiles that meet the canvas pixels `rect`.
    fn slots(&self, rect: &PixelRect) -> Vec<usize> {
        let target = self.output.place(rect);
        self.tiles(&target)
            .filter_map(|tile| self.slot_of[tile])
            .collect()
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
