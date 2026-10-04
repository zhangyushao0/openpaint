//! A view tier's work in one frame (S32): evaluating the canvas's changes in place or making
//! the tiles they reach wait, filling missing tiles, and logging how long a fill took.

use std::time::Instant;

use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::view_atlas::Tier;
use crate::view_needed::Needed;
use crate::view_tier::ViewWork;

/// View tiles evaluated whole in one frame at most.
const FILL_TILES: usize = 4;
/// Output pixels of changes to ready tiles evaluated in place in one frame at most; a larger
/// change makes the tiles it touches wait to be evaluated whole.
const EDIT_PIXELS: i64 = 4 * 256 * 256;

/// The ready tiles of `tier` the canvas pixels `changed` reach, each with the output pixels
/// reached.
pub(crate) fn touched(tier: &Tier, changed: &[PixelRect]) -> Vec<([u32; 2], PixelRect)> {
    let mut parts = Vec::new();
    for rect in changed {
        let placed = tier.map.place(rect);
        for (tile, part) in tier.map.meeting(&placed) {
            if tier.is_ready(tile) && !part.is_empty() {
                parts.push((tile, part));
            }
        }
    }
    parts
}

/// The target pixels to evaluate again for `changed`, when they are few; else the tiles they
/// touch wait to be evaluated whole.
pub(crate) fn in_place(
    tier: &mut Tier,
    changed: &[PixelRect],
    work: &mut ViewWork,
) -> Vec<PixelRect> {
    let parts = touched(tier, changed);
    let area: i64 = parts
        .iter()
        .map(|(_, part)| {
            i64::from(part.max[0] - part.min[0]) * i64::from(part.max[1] - part.min[1])
        })
        .sum();
    let mut tiles: Vec<[u32; 2]> = parts.iter().map(|(tile, _)| *tile).collect();
    tiles.sort_unstable();
    tiles.dedup();
    if area > EDIT_PIXELS {
        for tile in &tiles {
            tier.set_ready(*tile, false);
        }
        work.dropped = tiles;
        return Vec::new();
    }
    work.updated = tiles;
    parts
        .iter()
        .filter_map(|(tile, part)| {
            let slot = tier.map.slot(*tile)?;
            Some(tier.map.to_target(*tile, slot, part))
        })
        .collect()
}

/// Drops tiles beyond the keep ring, then gives missing needed tiles slots and returns their
/// target pixels, at most `FILL_TILES` of them, marked ready.
pub(crate) fn fill(
    core: &Core,
    tier: &mut Tier,
    needed: &Needed,
    work: &mut ViewWork,
) -> Result<Vec<PixelRect>, RenderError> {
    let held: Vec<[u32; 2]> = tier.map.held().map(|(tile, _)| tile).collect();
    for tile in &held {
        if !needed.keeps(*tile) {
            tier.release(*tile);
        }
    }
    let missing = needed
        .ordered()
        .filter(|tile| tier.map.slot(*tile).is_none())
        .count() as u32;
    let mut regions = Vec::new();
    for tile in needed.ordered() {
        if work.filled.len() >= FILL_TILES {
            break;
        }
        if tier.is_ready(tile) {
            continue;
        }
        if tier.map.slot(tile).is_none() {
            let spare = tier
                .map
                .held()
                .map(|(held, _)| held)
                .find(|held| !needed.needs(*held));
            if let (None, Some(spare)) = (tier.map.free(), spare) {
                tier.release(spare);
            }
            if !tier.hold(core, tile, missing)? {
                continue;
            }
        }
        let Some(slot) = tier.map.slot(tile) else {
            continue;
        };
        regions.push(tier.map.to_target(tile, slot, &tier.map.tile_rect(tile)));
        tier.set_ready(tile, true);
        work.filled.push(tile);
    }
    Ok(regions)
}

/// Needed tiles being evaluated whole, from the first frame that missed one until none is
/// missing: logged then, as the time a pan or a new tier took to fill in.
pub(crate) struct Filling {
    started: Instant,
    frames: u32,
    tiles: usize,
}

impl Filling {
    pub(crate) fn step(filling: Option<Self>, work: &ViewWork) -> Option<Self> {
        let mut filling = match filling {
            Some(filling) => filling,
            None if !work.filled.is_empty() => Self {
                started: Instant::now(),
                frames: 0,
                tiles: 0,
            },
            None => return None,
        };
        filling.frames += 1;
        filling.tiles += work.filled.len();
        if work.waiting > 0 {
            return Some(filling);
        }
        // A pan that one frame fills is no news; longer fills are what a pan or pinch costs.
        let level = if filling.frames > 1 {
            log::Level::Info
        } else {
            log::Level::Debug
        };
        log::log!(
            level,
            "view tier {}x filled {} tiles in {} ms over {} frames",
            work.tier.unwrap_or(0),
            filling.tiles,
            filling.started.elapsed().as_millis(),
            filling.frames
        );
        None
    }
}
