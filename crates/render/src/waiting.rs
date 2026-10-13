//! The screen canvas's tiles that wait to be evaluated (S53): whole 16-px tiles no frame's
//! background budget reached yet, taken in the order the screen needs them: the tiles the view
//! shows, nearest the screen's middle first, then the others by their distance from it; with no
//! view known (tests, the harness's own targets), rows from the top.

use crate::geom::PixelRect;
use crate::protocol::TILE;

/// Where the screen looks on the canvas, in canvas pixels: its middle and what it shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Focus {
    pub middle: [f32; 2],
    pub shown: PixelRect,
}

/// The middle moves this many canvas pixels before the order is made again.
const RESORT: f32 = 48.0;

#[derive(Debug, Default)]
pub(crate) struct Waiting {
    columns: u32,
    rows: u32,
    /// Per tile, row-major: it waits.
    marked: Vec<bool>,
    count: usize,
    /// Every waiting tile, in the order they are taken (and tiles taken since, skipped).
    order: Vec<u32>,
    /// The view the order was made for (`Some(None)`: none), `None` when tiles came since.
    sorted: Option<Option<Focus>>,
    /// Per tile: it waits put aside, for a smudge residual being evaluated over frames (S53).
    deferred: Vec<bool>,
    put_aside: usize,
}

impl Waiting {
    /// No tile of a canvas of `size` pixels waits.
    pub(crate) fn new([width, height]: [u32; 2]) -> Self {
        let (columns, rows) = (width.div_ceil(TILE), height.div_ceil(TILE));
        Self {
            columns,
            rows,
            marked: vec![false; (columns * rows) as usize],
            deferred: vec![false; (columns * rows) as usize],
            ..Self::default()
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The waiting tiles meeting `rects` are put aside: `next` passes them over until `resume`.
    pub(crate) fn defer(&mut self, rects: &[PixelRect]) {
        for rect in rects {
            for tile in self.tiles(rect) {
                let at = tile as usize;
                if self.marked[at] && !std::mem::replace(&mut self.deferred[at], true) {
                    self.put_aside += 1;
                }
            }
        }
    }

    /// Whether tiles wait put aside.
    pub(crate) fn has_deferred(&self) -> bool {
        self.put_aside > 0
    }

    /// The tiles put aside, as rectangles joined along rows.
    pub(crate) fn deferred_rects(&self) -> Vec<PixelRect> {
        self.rects_where(|at| self.deferred[at])
    }

    /// Every tile put aside is taken in order again.
    pub(crate) fn resume(&mut self) {
        self.deferred.fill(false);
        self.put_aside = 0;
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

    /// The tiles meeting `rects` no longer wait: they were evaluated (a plan evaluates whole
    /// tiles).
    pub(crate) fn remove(&mut self, rects: &[PixelRect]) {
        for rect in rects {
            for tile in self.tiles(rect) {
                let at = tile as usize;
                if std::mem::replace(&mut self.marked[at], false) {
                    self.count -= 1;
                }
                if std::mem::replace(&mut self.deferred[at], false) {
                    self.put_aside -= 1;
                }
            }
        }
        if self.count == 0 {
            self.order.clear();
        }
    }

    /// Every waiting tile as rectangles (joined along rows), none waiting any more.
    pub(crate) fn take_all(&mut self) -> Vec<PixelRect> {
        let rects = self.rects_where(|at| self.marked[at]);
        self.marked.fill(false);
        self.count = 0;
        self.order.clear();
        self.resume();
        rects
    }

    /// The tiles `keep` keeps (by row-major number), as rectangles joined along rows.
    fn rects_where(&self, keep: impl Fn(usize) -> bool) -> Vec<PixelRect> {
        let mut rects: Vec<PixelRect> = Vec::new();
        for tile in 0..self.marked.len() as u32 {
            if !keep(tile as usize) {
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
        rects
    }

    /// The first `most` waiting tiles in the order `focus` gives them, but those put aside, as
    /// rectangles; they keep waiting until `remove`d.
    pub(crate) fn next(&mut self, focus: Option<Focus>, most: usize) -> Vec<PixelRect> {
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
        (self.order.iter())
            .filter(|tile| !self.deferred[**tile as usize])
            .take(most)
            .map(|tile| self.rect(*tile))
            .collect()
    }

    /// The waiting tiles in the order `focus` gives them.
    fn sort(&mut self, focus: Option<Focus>) {
        let marked = &self.marked;
        self.order.retain(|tile| marked[*tile as usize]);
        self.order.sort_unstable();
        self.order.dedup();
        if let Some(focus) = focus {
            let columns = self.columns;
            let key = |tile: &u32| {
                let [x, y] = [tile % columns, tile / columns].map(|at| (at * TILE) as i32);
                let side = TILE as i32;
                let rect = PixelRect {
                    min: [x, y],
                    max: [x + side, y + side],
                };
                let middle = [x, y].map(|at| (at + side / 2) as f32);
                let [dx, dy] = [0, 1].map(|axis| middle[axis] - focus.middle[axis]);
                let hidden = rect.intersect(&focus.shown).is_empty();
                (hidden, (dx * dx + dy * dy) as u64)
            };
            self.order.sort_by_cached_key(key);
        }
        self.sorted = Some(focus);
    }

    /// Tile `tile`'s pixels.
    fn rect(&self, tile: u32) -> PixelRect {
        let [x, y] = [tile % self.columns, tile / self.columns].map(|at| (at * TILE) as i32);
        let side = TILE as i32;
        PixelRect {
            min: [x, y],
            max: [x + side, y + side],
        }
    }

    /// The tiles (row-major numbers) meeting `rect`.
    fn tiles(&self, rect: &PixelRect) -> impl Iterator<Item = u32> + use<> {
        let side = TILE as i32;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min: [i32; 2], max: [i32; 2]) -> PixelRect {
        PixelRect { min, max }
    }

    #[test]
    fn tiles_wait_once_and_come_nearest_the_screens_middle_first() {
        let mut waiting = Waiting::new([100, 50]);
        waiting.add(&[rect([0, 0], [100, 50]), rect([10, 10], [20, 20])]);
        assert_eq!(waiting.count, 7 * 4, "each tile once");
        // Without a view, rows from the top.
        assert_eq!(
            waiting.next(None, 2),
            [rect([0, 0], [16, 16]), rect([16, 0], [32, 16])]
        );
        let focus = Focus {
            middle: [90.0, 40.0],
            shown: rect([64, 32], [100, 50]),
        };
        let first = waiting.next(Some(focus), 3);
        assert_eq!(
            first[0],
            rect([80, 32], [96, 48]),
            "the shown tile nearest the middle"
        );
        assert!(first.iter().all(|r| !r.intersect(&focus.shown).is_empty()));
        waiting.remove(&[rect([80, 32], [81, 33])]);
        assert_eq!(
            waiting.next(Some(focus), 1)[0],
            first[1],
            "taken tiles are skipped"
        );
        // Put aside for a residual: passed over until resumed, still waiting.
        waiting.defer(&[first[1]]);
        assert!(waiting.has_deferred() && !waiting.is_empty());
        assert_eq!(waiting.deferred_rects(), [first[1]]);
        assert_eq!(waiting.next(Some(focus), 1)[0], first[2]);
        waiting.resume();
        assert_eq!(
            waiting.next(Some(focus), 1)[0],
            first[1],
            "taken in order again"
        );
        let all = waiting.take_all();
        assert_eq!(all.len(), 5, "joined along each row, but for the gap taken");
        assert!(waiting.is_empty() && waiting.next(Some(focus), 5).is_empty());
    }
}
