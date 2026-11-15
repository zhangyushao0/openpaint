//! A drawing's snapshots following the artwork (`drawing.rs`). A version with
//! strokes appended on the place a stroke lays leaves every snapshot as it is: the units before
//! the place keep their numbers; one the strokes do not reach (by what an undo of them would draw
//! again, `changed_regions`) stands before the new place's unit too, one they reach lays them when
//! it is next used. Any other change lets go the snapshots of the tiles it reaches and those still
//! lacking strokes (whose units may have moved), and the rest stand before the new place's unit; a
//! new place (another layer, its mask, a clipping chain begun or ended) lets every snapshot go.

use openpaint_core::{Artwork, LayerId};

use super::{Base, Drawing};
use crate::change::changed_regions;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster_plan::Restored;
use crate::scene_append::appended_over;
use crate::view_tiles::TIER_TILE;

impl Drawing {
    /// The drawing (of an output at `output`) follows `now`, whose scene lays a stroke on `place`
    /// at unit `at` (`None`: no such unit or the layer hidden: every snapshot goes); `dirty`, the
    /// canvas pixels a change reached (`None`: unknown, every snapshot goes on a change).
    pub(crate) fn follow(
        &mut self,
        now: &Artwork,
        place: (LayerId, bool),
        at: Option<(usize, Restored)>,
        (dirty, output): (Option<&[PixelRect]>, Output),
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
        // What the strokes appended can change: the snapshots standing at the old place that
        // they do not reach stand at the new one too.
        let placed = |rects: &[PixelRect]| -> Vec<PixelRect> {
            rects.iter().map(|rect| output.place(rect)).collect()
        };
        let reached = appended.then(|| placed(&changed_regions(&base.artwork, now)));
        *base = Base {
            epoch: base.epoch + u64::from(!appended),
            ..fresh(base.epoch)
        };
        if let Some(reached) = reached {
            for (tile, under) in &mut self.under {
                if *under.stands.end() == old && !meets(*tile, &reached) {
                    under.stands = *under.stands.start()..=at;
                }
            }
            return;
        }
        let dirty = dirty.map(placed);
        // The stroke tracked (its commit names no regions) reached its kept tiles.
        let gone: Vec<[u32; 2]> = (self.under.iter())
            .filter(|(tile, under)| {
                *under.stands.end() < old
                    || self.kept.contains_key(*tile)
                    || dirty.as_ref().is_none_or(|dirty| meets(**tile, dirty))
            })
            .map(|(tile, _)| *tile)
            .collect();
        for tile in gone {
            self.forget(tile);
        }
        for under in self.under.values_mut() {
            under.stands = at..=at;
        }
    }
}

/// Whether output tile `tile` meets any of the output pixels `rects`.
pub(crate) fn meets(tile: [u32; 2], rects: &[PixelRect]) -> bool {
    let min = tile.map(|at| (at * TIER_TILE) as i32);
    let side = TIER_TILE as i32;
    let rect = PixelRect {
        min,
        max: [min[0] + side, min[1] + side],
    };
    rects.iter().any(|dirty| !dirty.intersect(&rect).is_empty())
}
