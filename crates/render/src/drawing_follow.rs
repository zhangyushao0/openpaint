//! A drawing's snapshots following the artwork (S63, DECISIONS D90; `drawing.rs`). A version with
//! strokes appended on the place a stroke lays leaves every snapshot as it is: the units before
//! the place keep their numbers, and a snapshot standing before an earlier unit lays the strokes
//! since when it is next used. Any other change lets go the snapshots of the tiles it reaches and
//! those still lacking strokes (whose units may have moved), and the rest stand before the new
//! place's unit; a new place (another layer, its mask, a clipping chain begun or ended) lets every
//! snapshot go.

use openpaint_core::{Artwork, LayerId};

use super::{Base, Drawing};
use crate::geom::PixelRect;
use crate::raster_plan::Restored;
use crate::scene_append::appended_over;
use crate::view_tiles::TIER_TILE;

impl Drawing {
    /// The drawing follows `now`, whose scene lays a stroke on `place` at unit `at` (`None`: no
    /// such unit or the layer hidden: every snapshot goes); `dirty`,
    /// the output pixels a change reached (`None`: unknown, every snapshot goes on a change).
    pub(crate) fn follow(
        &mut self,
        now: &Artwork,
        place: (LayerId, bool),
        at: Option<(usize, Restored)>,
        dirty: Option<&[PixelRect]>,
    ) {
        let Some((at, restored)) = at else {
            self.forget_all();
            self.base = None;
            return;
        };
        let fresh = |epoch| Base {
            artwork: now.clone(),
            place,
            at,
            restored,
            epoch,
        };
        let Some(base) = &mut self.base else {
            self.forget_all();
            self.base = Some(fresh(0));
            return;
        };
        if base.place != place || base.restored != restored {
            let epoch = base.epoch + 1;
            self.forget_all();
            self.base = Some(fresh(epoch));
            return;
        }
        if base.artwork.same_version(now) {
            return;
        }
        let appended = appended_over(&base.artwork, now, place);
        let old = base.at;
        *base = Base {
            epoch: base.epoch + u64::from(!appended),
            ..fresh(base.epoch)
        };
        if appended {
            return;
        }
        // The stroke tracked (its commit names no regions) reached its kept tiles.
        let gone: Vec<[u32; 2]> = (self.under.iter())
            .filter(|(tile, under)| {
                under.at < old
                    || self.kept.contains_key(*tile)
                    || dirty.is_none_or(|dirty| meets(**tile, dirty))
            })
            .map(|(tile, _)| *tile)
            .collect();
        for tile in gone {
            self.forget(tile);
        }
        for under in self.under.values_mut() {
            under.at = at;
        }
    }
}

/// Whether output tile `tile` meets any of the output pixels `rects`.
fn meets(tile: [u32; 2], rects: &[PixelRect]) -> bool {
    let min = tile.map(|at| (at * TIER_TILE) as i32);
    let side = TIER_TILE as i32;
    let rect = PixelRect {
        min,
        max: [min[0] + side, min[1] + side],
    };
    rects.iter().any(|dirty| !dirty.intersect(&rect).is_empty())
}
