//! Bounded dispatches (S49, DECISIONS D76). A GPU runs one dispatch until it ends; past the
//! driver's time limit the device is lost. On the tablet that happened when an artwork of 300
//! Textured Round strokes over one place opened: the whole canvas went to the GPU at once and ran
//! for seconds. So every evaluation's tiles are cut into runs whose estimated work stays within a
//! bound, each run dispatched and submitted on its own, in order. A pixel's value never depends on
//! which tiles are evaluated with it (D16), so a bounded evaluation equals one dispatch bit for bit.
//!
//! The estimate counts what the evaluator walks: every op a tile lists, and the tile itself, each
//! costed as the costliest op measured on the tablet, a Textured Round span: 33 ns a pixel on the
//! Adreno 735 (S48: 53 ms for 6,330 entries in 42 tiles), 8.4 µs over a tile's 256 pixels.
//! `DISPATCH_WORK` entries are about 100 ms there, well inside the time limit (seconds); every
//! other op costs less, so real dispatches are shorter.
//!
//! The screen's canvas takes at most `FRAME_DISPATCHES` runs a frame (`evaluate_frame`): tiles
//! beyond wait for the next frames, so a large change (an artwork opening, an undo, a layer edit)
//! fills in over frames while taps and navigation go on; the in-progress stroke's tiles never wait.

use crate::geom::PixelRect;
use crate::protocol::{TILE, TileRecord};

/// The most work one dispatch takes: op entries listed in its tiles, plus one a tile.
pub const DISPATCH_WORK: u64 = 12_000;

/// Runs of `DISPATCH_WORK` the screen's canvas evaluates in one frame at most.
pub(crate) const FRAME_DISPATCHES: u64 = 2;

/// What evaluating `tile` costs: the ops it lists, and the tile itself.
pub(crate) fn tile_work(tile: &TileRecord) -> u64 {
    u64::from(tile.count) + 1
}

/// `tiles` cut in order into runs of at most `most` work; a tile above it alone is a run of its
/// own (its 256 pixels cannot be cut). No tiles: one empty run, so a dispatch still clears what it
/// must.
pub(crate) fn runs(tiles: &[TileRecord], most: u64) -> Vec<&[TileRecord]> {
    let mut runs = Vec::new();
    let (mut start, mut work) = (0, 0);
    for (index, tile) in tiles.iter().enumerate() {
        let cost = tile_work(tile);
        if index > start && work + cost > most {
            runs.push(&tiles[start..index]);
            (start, work) = (index, 0);
        }
        work += cost;
    }
    runs.push(&tiles[start..]);
    runs
}

/// The tiles of `tiles` one frame evaluates when they cost more than `most`: every tile meeting
/// `now` (the in-progress stroke's and what must show at once), then the others in order while
/// the work lasts, one at least; and the tiles left. `None` when every tile fits.
pub(crate) fn frame_share(
    tiles: &[TileRecord],
    now: &[PixelRect],
    most: u64,
) -> Option<(Vec<TileRecord>, Vec<TileRecord>)> {
    let total: u64 = tiles.iter().map(tile_work).sum();
    if total <= most {
        return None;
    }
    let urgent = |tile: &TileRecord| {
        let at = rect(tile);
        now.iter().any(|region| !region.intersect(&at).is_empty())
    };
    let (mut chosen, mut later): (Vec<TileRecord>, Vec<TileRecord>) =
        tiles.iter().partition(|tile| urgent(tile));
    let mut work: u64 = chosen.iter().map(tile_work).sum();
    let mut taken = 0;
    for tile in &later {
        let cost = tile_work(tile);
        if taken > 0 && work + cost > most {
            break;
        }
        work += cost;
        taken += 1;
    }
    chosen.extend(later.drain(..taken));
    Some((chosen, later))
}

/// `tiles` listing no op: evaluated, each shows the paper alone (S49: what a new target shows
/// where its tiles still wait, instead of undefined pixels).
pub(crate) fn paper_only(tiles: &[TileRecord]) -> impl Iterator<Item = TileRecord> + '_ {
    tiles.iter().map(|tile| TileRecord { count: 0, ..*tile })
}

/// The target pixels of `tiles` (on a target at one pixel per output pixel), joined where they
/// touch along a row.
pub(crate) fn tile_rects(tiles: &[TileRecord]) -> Vec<PixelRect> {
    merged(tiles.iter().map(rect).collect())
}

/// A tile's target pixels.
fn rect(tile: &TileRecord) -> PixelRect {
    let [x, y] = tile.origin.map(|v| v as i32);
    let side = TILE as i32;
    PixelRect {
        min: [x, y],
        max: [x + side, y + side],
    }
}

/// `rects` (whole tiles) joined where they touch along a row.
fn merged(mut rects: Vec<PixelRect>) -> Vec<PixelRect> {
    rects.sort_unstable_by_key(|rect| (rect.min[1], rect.min[0]));
    let mut out: Vec<PixelRect> = Vec::with_capacity(rects.len());
    for rect in rects {
        match out.last_mut() {
            Some(last) if last.min[1] == rect.min[1] && last.max[0] == rect.min[0] => {
                last.max[0] = rect.max[0];
            }
            _ => out.push(rect),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(x: u32, y: u32, count: u32) -> TileRecord {
        TileRecord {
            origin: [x * TILE, y * TILE],
            output: [x * TILE, y * TILE],
            start: 0,
            count,
        }
    }

    #[test]
    fn runs_stay_within_the_bound_and_keep_every_tile_in_order() {
        let tiles: Vec<TileRecord> = (0..10).map(|x| tile(x, 0, 9)).collect();
        let cut = runs(&tiles, 30);
        assert_eq!(
            cut.iter().map(|run| run.len()).collect::<Vec<_>>(),
            [3, 3, 3, 1]
        );
        assert!(cut.concat() == tiles);
        // A tile costlier than the bound alone; nothing: one empty run.
        let heavy = [tile(0, 0, 2), tile(1, 0, 100), tile(2, 0, 2)];
        assert_eq!(runs(&heavy, 30).len(), 3);
        assert_eq!(runs(&[], 30), [&[] as &[TileRecord]]);
    }

    #[test]
    fn a_frame_takes_the_urgent_tiles_then_what_its_work_allows() {
        let tiles: Vec<TileRecord> = (0..8).map(|x| tile(x, 0, 9)).collect();
        assert!(frame_share(&tiles, &[], 80).is_none(), "everything fits");
        let stroke = PixelRect {
            min: [7 * TILE as i32 + 2, 3],
            max: [7 * TILE as i32 + 5, 6],
        };
        let (chosen, left) = frame_share(&tiles, &[stroke], 30).expect("too much for a frame");
        // The stroke's tile first, then two more within 30.
        assert_eq!(chosen.len(), 3);
        assert_eq!(chosen[0].origin, [7 * TILE, 0]);
        let left = tile_rects(&left);
        assert_eq!(left.len(), 1, "the tiles left join along their row");
        assert_eq!(left[0].min, [2 * TILE as i32, 0]);
        assert_eq!(left[0].max, [7 * TILE as i32, TILE as i32]);
        assert!(paper_only(&tiles).all(|tile| tile.count == 0));
        // Over the work with nothing urgent, a frame still takes one tile.
        let heavy = [tile(0, 0, 100), tile(1, 0, 100)];
        let (chosen, left) = frame_share(&heavy, &[], 30).expect("too much");
        assert_eq!((chosen.len(), left.len()), (1, 1));
    }
}
