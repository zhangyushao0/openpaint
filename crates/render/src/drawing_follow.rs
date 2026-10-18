//! A drawing's snapshots following the artwork (S63, DECISIONS D90; `drawing.rs`). A version with
//! strokes appended on the place a stroke lays leaves every snapshot as it is: the units before
//! the place keep their numbers, and a snapshot standing before an earlier unit lays the strokes
//! since when it is next used. Any other change lets go the snapshots of the tiles it reaches and
//! those still lacking strokes (whose units may have moved), and the rest stand before the new
//! place's unit; a new place (another layer, its mask, a clipping chain begun or ended) lets every
//! snapshot go.

use openpaint_core::{Artwork, LayerId, StrokeList};

use super::{Base, Drawing};
use crate::geom::PixelRect;
use crate::raster_plan::Restored;
use crate::view_tiles::TIER_TILE;

impl Drawing {
    /// The drawing follows `now`, whose scene lays a stroke on `place` at unit `at` (`None`: no
    /// such unit, the layer hidden or the scene the evaluator's: every snapshot goes); `dirty`,
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

/// Whether `now` is `base` with strokes appended on the paint or mask `place` names and nothing
/// else changed: what lies under the place is what the snapshots taken over `base` hold, and the
/// strokes appended.
pub(crate) fn appended_over(
    base: &Artwork,
    now: &Artwork,
    (layer, on_mask): (LayerId, bool),
) -> bool {
    let appended = |old: &StrokeList, new: &StrokeList| {
        new.len() > old.len()
            && old.last().is_none_or(|last| {
                (new.get(old.len() - 1)).is_some_and(|same| same.shares_samples(last))
            })
    };
    if base.canvas() != now.canvas()
        || base.groups() != now.groups()
        || base.layers().len() != now.layers().len()
    {
        return false;
    }
    base.layers().iter().zip(now.layers()).all(|(old, new)| {
        let drawn = old.id() == layer;
        let masks = match (old.mask(), new.mask()) {
            (None, None) => true,
            (Some(a), Some(b)) if drawn && on_mask => {
                a.enabled() == b.enabled()
                    && a.inverted() == b.inverted()
                    && a.edits() == b.edits()
                    && appended(a.strokes(), b.strokes())
            }
            (Some(a), Some(b)) => a.enabled() == b.enabled() && a.same_grey(b),
            _ => false,
        };
        // Edits compare shared masks and patches first: a commit keeps them.
        let paint = if drawn && !on_mask {
            old.edits() == new.edits() && appended(old.strokes(), new.strokes())
        } else {
            old.same_paint(new)
        };
        old.id() == new.id() && old.props() == new.props() && masks && paint
    })
}
