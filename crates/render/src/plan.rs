//! `Plan`: a scene binned into the tiles of the regions being evaluated. Each tile lists, in
//! scene order, only the ops that can reach it. An op that cannot reach a pixel leaves it
//! exactly unchanged (zero coverage, or compositing nothing), so a pixel's value never depends
//! on which regions are evaluated together: incremental evaluation equals cold evaluation.

use crate::geom::PixelRect;
use crate::protocol::{FRESH, TILE, TileRecord};
use crate::scene::{Fresh, Item, Scene};

#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub(crate) tiles: Vec<TileRecord>,
    /// Scene item indices, or fresh op indices with the `FRESH` bit; each tile owns
    /// `start..start + count`.
    pub(crate) indices: Vec<u32>,
}

impl Plan {
    /// Regions are clipped to the target of `size` pixels and widened to whole tiles. At the
    /// scene's `Active` place every tile lists the fresh segments that reach it and then the
    /// fresh paint op: the in-progress stroke's earlier coverage may lie in any tile.
    pub(crate) fn build(
        scene: &Scene,
        fresh: &Fresh,
        size: [u32; 2],
        regions: &[PixelRect],
    ) -> Self {
        let grid = Grid::new(size, regions);
        let mut lists = vec![Vec::new(); grid.origins.len()];
        let mut stroke = Touched::new(lists.len());
        let mut layer = Touched::new(lists.len());
        let mut fresh_lists = Some(grid.bin(fresh));
        for (index, item) in scene.items.iter().enumerate() {
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            match item {
                Item::Segment { bounds, .. } => {
                    for slot in grid.slots(&bounds.pixels()) {
                        lists[slot].push(index);
                        stroke.touch(slot);
                    }
                }
                Item::EndStroke(_) => {
                    for slot in stroke.close() {
                        lists[slot].push(index);
                        layer.touch(slot);
                    }
                }
                Item::Active => {
                    let binned = fresh_lists.take().unwrap_or_default();
                    for (slot, mut list) in binned.into_iter().enumerate() {
                        list.push(FRESH);
                        lists[slot].append(&mut list);
                        layer.touch(slot);
                    }
                }
                Item::EndLayer(_) => {
                    for slot in layer.close() {
                        lists[slot].push(index);
                    }
                }
            }
        }
        Self::flatten(&grid.origins, lists)
    }

    fn flatten(origins: &[[u32; 2]], lists: Vec<Vec<u32>>) -> Self {
        let mut plan = Self::default();
        for (origin, list) in origins.iter().zip(lists) {
            let start = u32::try_from(plan.indices.len()).unwrap_or(u32::MAX);
            let count = u32::try_from(list.len()).unwrap_or(u32::MAX);
            plan.tiles.push(TileRecord {
                origin: *origin,
                start,
                count,
            });
            plan.indices.extend(list);
        }
        plan
    }
}

/// The target's tile grid and which tiles are being evaluated (a dense slot each).
struct Grid {
    columns: usize,
    rows: usize,
    /// Per grid tile, row major: its slot, or `None` when not evaluated.
    slot_of: Vec<Option<usize>>,
    /// Per slot: the tile's top-left pixel.
    origins: Vec<[u32; 2]>,
}

impl Grid {
    fn new(size: [u32; 2], regions: &[PixelRect]) -> Self {
        let columns = size[0].div_ceil(TILE) as usize;
        let rows = size[1].div_ceil(TILE) as usize;
        let mut grid = Self {
            columns,
            rows,
            slot_of: vec![None; columns * rows],
            origins: Vec::new(),
        };
        for region in regions {
            for tile in grid.tiles(region) {
                if grid.slot_of[tile].is_none() {
                    grid.slot_of[tile] = Some(grid.origins.len());
                    let (column, row) = (tile % columns, tile / columns);
                    grid.origins.push([column as u32 * TILE, row as u32 * TILE]);
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

    /// Slots of the evaluated tiles that meet `rect`.
    fn slots(&self, rect: &PixelRect) -> impl Iterator<Item = usize> + '_ {
        self.tiles(rect).filter_map(|tile| self.slot_of[tile])
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

/// The slots one stroke or layer has reached so far, each listed once.
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

    /// The touched slots in first-touch order; starts a new epoch.
    fn close(&mut self) -> Vec<usize> {
        self.epoch += 1;
        std::mem::take(&mut self.slots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Bounds;
    use crate::protocol::Op;

    fn segment(bounds: [f32; 4]) -> Item {
        Item::Segment {
            op: Op::segment([0.0; 2], [0.0; 2], [1.0; 2]),
            bounds: Bounds(bounds),
        }
    }

    fn scene() -> Scene {
        Scene {
            items: vec![
                segment([2.0, 2.0, 20.0, 4.0]),
                Item::EndStroke(Op::end_stroke([0.0; 3], 1.0)),
                segment([40.0, 2.0, 41.0, 3.0]),
                Item::EndStroke(Op::end_stroke([0.0; 3], 1.0)),
                Item::EndLayer(Op::end_layer(1.0)),
            ],
        }
    }

    fn lists(plan: &Plan) -> Vec<([u32; 2], Vec<u32>)> {
        let list = |tile: &TileRecord| {
            let range = tile.start as usize..(tile.start + tile.count) as usize;
            (tile.origin, plan.indices[range].to_vec())
        };
        plan.tiles.iter().map(list).collect()
    }

    #[test]
    fn tiles_list_only_the_ops_that_reach_them_in_scene_order() {
        let none = Fresh::default();
        let plan = Plan::build(&scene(), &none, [48, 20], &[PixelRect::full([48, 20])]);
        let expected = [
            ([0, 0], vec![0, 1, 4]),
            ([16, 0], vec![0, 1, 4]),
            ([32, 0], vec![2, 3, 4]),
            ([0, 16], vec![]),
            ([16, 16], vec![]),
            ([32, 16], vec![]),
        ];
        assert_eq!(lists(&plan), expected);
    }

    #[test]
    fn regions_are_clipped_widened_to_tiles_and_deduplicated() {
        let regions = [
            PixelRect {
                min: [-50, -50],
                max: [3, 3],
            },
            PixelRect {
                min: [1, 1],
                max: [2, 2],
            },
            PixelRect {
                min: [47, 19],
                max: [900, 900],
            },
            PixelRect {
                min: [5, 5],
                max: [5, 9],
            },
        ];
        let plan = Plan::build(&scene(), &Fresh::default(), [48, 20], &regions);
        let expected = [([0, 0], vec![0, 1, 4]), ([32, 16], vec![])];
        assert_eq!(lists(&plan), expected);
    }

    #[test]
    fn every_tile_paints_the_active_stroke_after_the_fresh_segments_that_reach_it() {
        let scene = Scene {
            items: vec![
                segment([2.0, 2.0, 4.0, 4.0]),
                Item::EndStroke(Op::end_stroke([0.0; 3], 1.0)),
                Item::Active,
                Item::EndLayer(Op::end_layer(1.0)),
            ],
        };
        let fresh = Fresh {
            ops: vec![Op::end_active([0.0; 3], 1.0); 2],
            bounds: vec![Bounds([20.0, 2.0, 22.0, 3.0])],
        };
        let plan = Plan::build(&scene, &fresh, [48, 16], &[PixelRect::full([48, 16])]);
        let expected = [
            ([0, 0], vec![0, 1, FRESH, 3]),
            ([16, 0], vec![FRESH | 1, FRESH, 3]),
            ([32, 0], vec![FRESH, 3]),
        ];
        assert_eq!(lists(&plan), expected);
    }
}
