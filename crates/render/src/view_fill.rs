//! A view tier's work in one frame: which ready tiles the canvas's changes reach (drawn in
//! place or made to wait, `view_raster.rs`), and logging how long a fill took.

use std::time::Instant;

use crate::geom::PixelRect;
use crate::view_atlas::Tier;
use crate::view_tier::ViewWork;

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

/// Needed tiles being drawn whole, from the first frame that missed one until none is
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
