//! The screen canvas's tiles that wait to be drawn (its 256-px tiles,
//! `canvas_raster.rs`), taken in the order the screen needs them: the tiles the view shows,
//! nearest the screen's middle first, then the others by their distance from it; with no view
//! known (tests, the harness's own targets), rows from the top.

use crate::geom::PixelRect;

/// Where the screen looks on the canvas, in canvas pixels: its middle and what it shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Focus {
    pub middle: [f32; 2],
    pub shown: PixelRect,
}

impl Focus {
    /// Where the tile `rect` comes in the order the screen needs tiles: those it shows first,
    /// then by the distance of the tile's middle from its middle.
    pub(crate) fn order(&self, rect: &PixelRect) -> (bool, u64) {
        let middle = [0, 1].map(|axis| (rect.min[axis] + rect.max[axis]) as f32 / 2.0);
        let [dx, dy] = [0, 1].map(|axis| middle[axis] - self.middle[axis]);
        let hidden = rect.intersect(&self.shown).is_empty();
        (hidden, (dx * dx + dy * dy) as u64)
    }
}

/// The middle moves this many canvas pixels before the order is made again.
const RESORT: f32 = 48.0;

#[derive(Debug, Default)]
pub(crate) struct Waiting {
    /// A tile's side in pixels.
    side: u32,
    columns: u32,
    rows: u32,
    /// Per tile, row-major: it waits.
    marked: Vec<bool>,
    count: usize,
    /// Every waiting tile, in the order they are taken (and tiles taken since, skipped).
    order: Vec<u32>,
    /// The view the order was made for (`Some(None)`: none), `None` when tiles came since.
    sorted: Option<Option<Focus>>,
}

impl Waiting {
    /// No tile of `side` pixels of a canvas of `size` pixels waits.
    pub(crate) fn of([width, height]: [u32; 2], side: u32) -> Self {
        let (columns, rows) = (width.div_ceil(side), height.div_ceil(side));
        Self {
            side,
            columns,
            rows,
            marked: vec![false; (columns * rows) as usize],
            ..Self::default()
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The tiles meeting `rects` wait too.
    pub(crate) fn add(&mut self, rects: &[PixelRect]) {
        for rect in rects {
            for tile in self.tiles(rect) {
                if !self.marked[tile as usize] {
                    self.marked[tile as usize] = true;
                    self.count += 1;
                    self.order.push(tile);
                    self.sorted = None;
                }
            }
        }
    }

    /// The tiles meeting `rects` no longer wait: they were drawn.
    pub(crate) fn remove(&mut self, rects: &[PixelRect]) {
        for rect in rects {
            for tile in self.tiles(rect) {
                if std::mem::replace(&mut self.marked[tile as usize], false) {
                    self.count -= 1;
                }
            }
        }
        if self.count == 0 {
            self.order.clear();
        }
    }

    /// Every waiting tile as rectangles (joined along rows), none waiting any more.
    pub(crate) fn take_all(&mut self) -> Vec<PixelRect> {
        let mut rects: Vec<PixelRect> = Vec::new();
        for tile in 0..self.marked.len() as u32 {
            if !self.marked[tile as usize] {
                continue;
            }
            let rect = self.rect(tile);
            match rects.last_mut() {
                Some(last) if last.min[1] == rect.min[1] && last.max[0] == rect.min[0] => {
                    last.max[0] = rect.max[0];
                }
                _ => rects.push(rect),
            }
        }
        self.marked.fill(false);
        self.count = 0;
        self.order.clear();
        rects
    }

    /// The first waiting tile in the order `focus` gives them that `skip` does not pass over
    /// (one the raster worker draws); it keeps waiting until `remove`d.
    pub(crate) fn first(
        &mut self,
        focus: Option<Focus>,
        skip: impl Fn(&PixelRect) -> bool,
    ) -> Option<PixelRect> {
        self.ordered(focus);
        (self.order.iter())
            .map(|tile| self.rect(*tile))
            .find(|rect| !skip(rect))
    }

    /// The order made again when the view moved far enough, taken tiles left out.
    fn ordered(&mut self, focus: Option<Focus>) {
        let moved = match (self.sorted, focus) {
            (Some(Some(before)), Some(now)) => {
                let [dx, dy] = [0, 1].map(|axis| now.middle[axis] - before.middle[axis]);
                dx.hypot(dy) > RESORT || before.shown != now.shown
            }
            (Some(before), now) => before.is_some() != now.is_some(),
            (None, _) => true,
        };
        if moved {
            self.sort(focus);
        }
        let marked = &self.marked;
        self.order.retain(|tile| marked[*tile as usize]);
    }

    /// The waiting tiles in the order `focus` gives them.
    fn sort(&mut self, focus: Option<Focus>) {
        let marked = &self.marked;
        self.order.retain(|tile| marked[*tile as usize]);
        self.order.sort_unstable();
        self.order.dedup();
        if let Some(focus) = focus {
            let (columns, side) = (self.columns, self.side);
            let key = |tile: &u32| focus.order(&rect(tile, (columns, side)));
            self.order.sort_by_cached_key(key);
        }
        self.sorted = Some(focus);
    }

    /// Tile `tile`'s pixels.
    fn rect(&self, tile: u32) -> PixelRect {
        rect(&tile, (self.columns, self.side))
    }

    /// The tiles (row-major numbers) meeting `rect`.
    fn tiles(&self, rect: &PixelRect) -> impl Iterator<Item = u32> + use<> {
        let side = self.side as i32;
        let span = |min: i32, max: i32, count: u32| {
            if max <= min {
                return 0..0;
            }
            let first = (min.max(0) / side) as u32;
            let end = ((max.max(0) + side - 1) / side) as u32;
            first.min(count)..end.min(count)
        };
        let columns = span(rect.min[0], rect.max[0], self.columns);
        let rows = span(rect.min[1], rect.max[1], self.rows);
        let width = self.columns;
        rows.flat_map(move |row| columns.clone().map(move |column| row * width + column))
    }
}

/// The pixels of tile `tile` (row-major) of `columns` tiles a row, each `side` pixels a side.
fn rect(tile: &u32, (columns, side): (u32, u32)) -> PixelRect {
    let [x, y] = [tile % columns, tile / columns].map(|at| (at * side) as i32);
    let side = side as i32;
    PixelRect {
        min: [x, y],
        max: [x + side, y + side],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min: [i32; 2], max: [i32; 2]) -> PixelRect {
        PixelRect { min, max }
    }

    #[test]
    fn tiles_wait_once_and_come_nearest_the_screens_middle_first() {
        let mut waiting = Waiting::of([100, 50], 16);
        waiting.add(&[rect([0, 0], [100, 50]), rect([10, 10], [20, 20])]);
        assert_eq!(waiting.count, 7 * 4, "each tile once");
        // Without a view, rows from the top; a tile skipped is passed over.
        let first = rect([0, 0], [16, 16]);
        assert_eq!(waiting.first(None, |_| false), Some(first));
        assert_eq!(
            waiting.first(None, |tile| *tile == first),
            Some(rect([16, 0], [32, 16]))
        );
        let focus = Focus {
            middle: [90.0, 40.0],
            shown: rect([64, 32], [100, 50]),
        };
        let near = rect([80, 32], [96, 48]);
        assert_eq!(
            waiting.first(Some(focus), |_| false),
            Some(near),
            "the shown tile nearest the middle"
        );
        waiting.remove(&[rect([80, 32], [81, 33])]);
        let next = waiting.first(Some(focus), |_| false).expect("one waits");
        assert!(
            next != near && !next.intersect(&focus.shown).is_empty(),
            "taken tiles are skipped"
        );
        let all = waiting.take_all();
        assert_eq!(all.len(), 5, "joined along each row, but for the gap taken");
        assert!(waiting.is_empty() && waiting.first(Some(focus), |_| false).is_none());
    }
}
