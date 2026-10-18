//! Bounded dispatches (S49, DECISIONS D76). A GPU runs one dispatch until it ends; past the
//! driver's time limit the device is lost. On the tablet that happened when an artwork of 300
//! Textured Round strokes over one place opened: the whole canvas went to the GPU at once and ran
//! for seconds. So every evaluation's tiles are cut into runs whose estimated work stays within a
//! bound, each run dispatched and submitted on its own, in order. A pixel's value never depends on
//! which tiles are evaluated with it (D82), so a bounded evaluation equals one dispatch bit for bit.
//! Since S65 only the evaluator's reference images are evaluated so (`evaluator.rs`).
//!
//! The estimate counts what the evaluator walks: every op a tile lists, and the tile itself, each
//! costed as the costliest op measured on the tablet, a Textured Round span: 33 ns a pixel on the
//! Adreno 735 (S48: 53 ms for 6,330 entries in 42 tiles), 8.4 µs over a tile's 256 pixels.
//! `DISPATCH_WORK` entries are about 100 ms there, well inside the time limit (seconds); every
//! other op costs less, so real dispatches are shorter.

use crate::protocol::TileRecord;

/// The most work one dispatch takes: op entries listed in its tiles, plus one a tile.
pub const DISPATCH_WORK: u64 = 12_000;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::TILE;

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
}
