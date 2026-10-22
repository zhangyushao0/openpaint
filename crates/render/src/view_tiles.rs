//! A view tier's tiles: the canvas's output at the tier's scale cut into squares of
//! `TIER_TILE` output pixels from the canvas's top-left, and the slots of the tier's target (an
//! atlas of such squares, `across` to a row) holding some of them. A tier's tiles are drawn into
//! their slots through it; the blit finds a tile's slot through its index.

use crate::geom::PixelRect;
use crate::output::Output;

/// Output pixels along a view tile's side.
pub(crate) const TIER_TILE: u32 = 256;

/// Which slot holds which view tile.
#[derive(Debug)]
pub(crate) struct TileMap {
    output: Output,
    /// The canvas's output in pixels, and in view tiles across and down.
    size: [u32; 2],
    tiles: [u32; 2],
    /// Slots in a row of the target, and rows.
    across: u32,
    rows: u32,
    /// Per view tile, row-major: its slot.
    slot_of: Vec<Option<u32>>,
    /// Per slot, row-major: the view tile it holds.
    held: Vec<Option<[u32; 2]>>,
}

impl TileMap {
    /// A map of the output of a `canvas` of pixels at `scale`, into `rows` rows of `across`
    /// slots, all free.
    pub(crate) fn new(canvas: [u32; 2], output: Output, (across, rows): (u32, u32)) -> Self {
        let size = canvas.map(|side| side * output.scale);
        let tiles = size.map(|side| side.div_ceil(TIER_TILE));
        Self {
            output,
            size,
            tiles,
            across,
            rows,
            slot_of: vec![None; (tiles[0] * tiles[1]) as usize],
            held: vec![None; (across * rows) as usize],
        }
    }

    pub(crate) fn scale(&self) -> u32 {
        self.output.scale
    }

    /// View tiles across and down the canvas's output.
    pub(crate) fn tiles(&self) -> [u32; 2] {
        self.tiles
    }

    /// Slots across and down the target.
    pub(crate) fn slots(&self) -> (u32, u32) {
        (self.across, self.rows)
    }

    /// The slot holding view tile `tile`.
    pub(crate) fn slot(&self, [column, row]: [u32; 2]) -> Option<u32> {
        let tiles = self.tiles;
        (column < tiles[0] && row < tiles[1])
            .then(|| self.slot_of[(row * tiles[0] + column) as usize])
            .flatten()
    }

    /// Every view tile a slot holds, with its slot.
    pub(crate) fn held(&self) -> impl Iterator<Item = ([u32; 2], u32)> + '_ {
        (0..)
            .zip(&self.held)
            .filter_map(|(slot, tile)| Some(((*tile)?, slot)))
    }

    /// A free slot, if any.
    pub(crate) fn free(&self) -> Option<u32> {
        (0..)
            .zip(&self.held)
            .find_map(|(slot, tile)| tile.is_none().then_some(slot))
    }

    /// Puts view tile `tile` into the free slot `slot`.
    pub(crate) fn hold(&mut self, tile: [u32; 2], slot: u32) {
        debug_assert!(self.held[slot as usize].is_none() && self.slot(tile).is_none());
        self.held[slot as usize] = Some(tile);
        self.slot_of[(tile[1] * self.tiles[0] + tile[0]) as usize] = Some(slot);
    }

    /// Frees the slot holding `tile`.
    pub(crate) fn release(&mut self, tile: [u32; 2]) {
        if let Some(slot) = self.slot(tile) {
            self.held[slot as usize] = None;
            self.slot_of[(tile[1] * self.tiles[0] + tile[0]) as usize] = None;
        }
    }

    /// `rows` more rows of free slots below the others.
    pub(crate) fn grow(&mut self, rows: u32) {
        self.rows += rows;
        self.held.resize((self.across * self.rows) as usize, None);
    }

    /// The target pixel at a slot's top-left.
    pub(crate) fn slot_origin(&self, slot: u32) -> [u32; 2] {
        [slot % self.across, slot / self.across].map(|at| at * TIER_TILE)
    }

    /// A view tile's output pixels, cut to the canvas's output.
    pub(crate) fn tile_rect(&self, tile: [u32; 2]) -> PixelRect {
        tile_rect(tile, self.size)
    }

    /// The output pixels holding the centres of the canvas pixels `rect` (as `Output::place`).
    pub(crate) fn place(&self, rect: &PixelRect) -> PixelRect {
        self.output.place(rect)
    }

    /// The view tiles meeting the output pixels `rect`, each with that part of `rect`.
    pub(crate) fn meeting(&self, rect: &PixelRect) -> impl Iterator<Item = ([u32; 2], PixelRect)> {
        meeting(*rect, self.size)
    }

    /// Output pixels `part` of `tile` as the target pixels of its `slot`.
    pub(crate) fn to_target(&self, tile: [u32; 2], slot: u32, part: &PixelRect) -> PixelRect {
        let origin = self.slot_origin(slot);
        let shift = |axis: usize| origin[axis] as i32 - (tile[axis] * TIER_TILE) as i32;
        let [x, y] = [shift(0), shift(1)];
        PixelRect {
            min: [part.min[0] + x, part.min[1] + y],
            max: [part.max[0] + x, part.max[1] + y],
        }
    }
}

/// Output tile `tile`'s pixels inside an output of `size` pixels.
pub(crate) fn tile_rect(tile: [u32; 2], size: [u32; 2]) -> PixelRect {
    let min = tile.map(|at| at * TIER_TILE);
    PixelRect {
        min: min.map(|at| at as i32),
        max: [0, 1].map(|axis| (min[axis] + TIER_TILE).min(size[axis]) as i32),
    }
}

/// The output tiles meeting output pixels `rect` inside an output of `size` pixels, each with
/// that part of `rect`.
pub(crate) fn meeting(
    rect: PixelRect,
    size: [u32; 2],
) -> impl Iterator<Item = ([u32; 2], PixelRect)> {
    let rect = rect.intersect(&PixelRect::full(size));
    let side = TIER_TILE as i32;
    let span = |axis: usize| {
        if rect.is_empty() {
            return 0..0;
        }
        rect.min[axis] / side..(rect.max[axis] + side - 1) / side
    };
    let (columns, rows) = (span(0), span(1));
    rows.flat_map(move |row| columns.clone().map(move |column| [column, row]))
        .map(move |[column, row]| {
            let min = [column * side, row * side];
            let tile = PixelRect {
                min,
                max: [min[0] + side, min[1] + side],
            };
            ([column, row].map(i32::unsigned_abs), rect.intersect(&tile))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_go_into_slots_and_rects_follow_them() {
        let output = Output::at(4, [0, 0]).expect("4x");
        let mut map = TileMap::new([100, 70], output, (2, 1));
        assert_eq!(map.tiles(), [2, 2], "400 x 280 output pixels");
        assert_eq!(map.free(), Some(0));
        map.hold([1, 1], 0);
        map.hold([0, 1], 1);
        assert_eq!(map.free(), None);
        assert_eq!((map.slot([1, 1]), map.slot([0, 0])), (Some(0), None));
        // The bottom right tile is 144 x 24 output pixels.
        let rect = map.tile_rect([1, 1]);
        assert_eq!((rect.min, rect.max), ([256, 256], [400, 280]));
        // Output pixel 300, 260 lies 44, 4 into tile 1, 1, held in slot 0.
        let part = PixelRect {
            min: [300, 260],
            max: [301, 261],
        };
        let slot = map.slot([1, 1]).expect("held");
        assert_eq!(
            map.to_target([1, 1], slot, &part),
            PixelRect {
                min: [44, 4],
                max: [45, 5]
            }
        );
        map.release([1, 1]);
        assert_eq!(map.free(), Some(0));
        map.grow(1);
        assert_eq!((map.slots(), map.slot_origin(3)), ((2, 2), [256, 256]));
        let meeting: Vec<_> = map
            .meeting(&PixelRect {
                min: [250, -5],
                max: [900, 10],
            })
            .map(|(tile, _)| tile)
            .collect();
        assert_eq!(meeting, [[0, 0], [1, 0]], "cut to the output");
    }
}
