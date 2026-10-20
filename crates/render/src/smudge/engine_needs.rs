//! Which smudge residuals a draw needs in the arena (S51), and which tiles of each: those
//! reaching the pixels it draws, near them; and, for a residual that must be derived anew, the
//! earlier residuals' tiles its material reads (its own tiles). Filter results are always needed
//! (their material reads their whole layer); a filter made anew needs every residual before it
//! whole, and under a transform record a residual's tiles lie in its content's pixels, so it is
//! needed whole too.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use openpaint_core::Stroke;

use super::{Arena, Key, TileSet};
use super::{dabs, key, read_tiles};
use crate::geom::PixelRect;
use crate::scene::{Scene, SmudgeItem};

/// The residual tiles each smudge's contacts read, kept per stroke (S53): walking a stroke's
/// contacts again for every evaluation that asks what it needs cost the screen's frames their CPU
/// time. Each entry keeps its stroke alive, so its key names it alone.
#[derive(Default)]
pub(crate) struct Reads(HashMap<Key, (Stroke, Arc<BTreeSet<u32>>)>);

impl Reads {
    /// The residual tiles `item`'s contacts read on a canvas of `size` pixels.
    pub(crate) fn of(&mut self, item: &SmudgeItem, size: [u32; 2]) -> Arc<BTreeSet<u32>> {
        let stroke = &item.stroke;
        let (_, read) = (self.0).entry(key(item.layer, stroke)).or_insert_with(|| {
            let all = dabs(stroke.style(), stroke.samples());
            (stroke.clone(), Arc::new(read_tiles(&all, size)))
        });
        Arc::clone(read)
    }

    /// Forgets the strokes `scene` no longer shows, once there are many.
    pub(crate) fn keep(&mut self, scene: &Scene) {
        if self.0.len() <= 2 * scene.smudges.len() + 64 {
            return;
        }
        let shown: std::collections::HashSet<Key> = (scene.smudges.iter())
            .map(|item| key(item.layer, &item.stroke))
            .collect();
        self.0.retain(|key, _| shown.contains(key));
    }
}

/// What an evaluation needs present in the arena.
#[derive(Debug, Clone)]
pub(crate) enum Wanted<'a> {
    /// The residuals reaching these canvas pixels, near them.
    Near(&'a [PixelRect]),
}

/// How much of one residual an evaluation needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Need {
    No,
    Part(TileSet),
    All,
}

impl Need {
    fn and(&mut self, other: Self) {
        let joined = match (std::mem::replace(self, Self::No), other) {
            (Self::All, _) | (_, Self::All) => Self::All,
            (Self::Part(mut a), Self::Part(b)) => {
                a.join(&b);
                Self::Part(a)
            }
            (Self::No, need) | (need, Self::No) => need,
        };
        *self = joined;
    }

    /// The tiles needed (`None`: all of them), or `None` when it is not needed at all.
    pub(crate) fn tiles(&self) -> Option<Option<&TileSet>> {
        match self {
            Self::No => None,
            Self::Part(tiles) => Some(Some(tiles)),
            Self::All => Some(None),
        }
    }
}

/// Canvas pixels around what is drawn whose residual tiles are held too: a margin beyond the
/// bilinear read's pixel (`around_output`), so what the next tiles of a view or a stroke nearby
/// read is at hand.
const NEAR: i32 = 17;

/// `rect` widened by `by` pixels each way.
fn widened(rect: &PixelRect, by: i32) -> PixelRect {
    PixelRect {
        min: rect.min.map(|value| value.saturating_sub(by)),
        max: rect.max.map(|value| value.saturating_add(by)),
    }
}

/// The part of `item` near the canvas pixels `near` on a canvas of `size` pixels.
fn near(item: &SmudgeItem, near: &PixelRect, size: [u32; 2]) -> Need {
    let met = near.intersect(&item.canvas);
    if met.is_empty() {
        return Need::No;
    }
    if !item.to_canvas.is_identity() {
        return Need::All;
    }
    Need::Part(TileSet::of(&met, size))
}

/// The canvas pixels a raster region of an output at `scale` reads (its output pixels `rect`):
/// those its pixels' centres lie in, widened by one for a bilinear read (S61).
pub(crate) fn around_output(rect: &PixelRect, scale: u32) -> PixelRect {
    let scale = i32::try_from(scale.max(1)).unwrap_or(1);
    PixelRect {
        min: rect.min.map(|at| at.div_euclid(scale) - 1),
        max: rect
            .max
            .map(|at| at.saturating_add(scale - 1).div_euclid(scale) + 1),
    }
}

/// What `wanted` needs of each of `scene`'s smudges, before what residuals evaluated anew read.
pub(crate) fn needs(scene: &Scene, wanted: &Wanted<'_>) -> Vec<Need> {
    let Wanted::Near(regions) = wanted;
    let item_need = |item: &SmudgeItem| {
        let mut need = Need::No;
        for region in *regions {
            need.and(near(item, &widened(region, NEAR), scene.canvas));
        }
        need
    };
    scene.smudges.iter().map(item_need).collect()
}

/// Adds to `needs` what the residuals and filter results `arena` lacks read when evaluated anew
/// (keys in `keys`, smudges then filters, as `keys_of` gives them): later ones first, so what an
/// earlier residual is needed for is known before it is looked at.
pub(crate) fn close(
    scene: &Scene,
    (keys, arena, reads): (&[Key], &Arena, &mut Reads),
    needs: &mut [Need],
) {
    let smudges = &scene.smudges;
    for (filter, key) in scene.filters.iter().zip(&keys[smudges.len()..]) {
        if arena.base(key).is_none() {
            for (need, earlier) in needs.iter_mut().zip(smudges) {
                if filter.before.contains(&earlier.item) {
                    *need = Need::All;
                }
            }
        }
    }
    for index in (0..smudges.len()).rev() {
        let Some(tiles) = needs[index].tiles() else {
            continue;
        };
        if arena.holds(&keys[index], tiles) {
            continue;
        }
        let item = &smudges[index];
        let moved = !item.to_canvas.is_identity();
        // Its material is evaluated over the tiles its contacts read.
        let read = reads.of(item, scene.canvas);
        for (need, earlier) in needs[..index].iter_mut().zip(smudges) {
            if !item.before.contains(&earlier.item) {
                continue;
            }
            if moved || !earlier.to_canvas.is_identity() {
                need.and(Need::All);
                continue;
            }
            let around = TileSet::of(&widened(&earlier.canvas, NEAR), scene.canvas);
            let shared: TileSet = TileSet(read.intersection(&around.0).copied().collect());
            if !shared.0.is_empty() {
                need.and(Need::Part(shared));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_join_and_all_wins_and_tiles_count_row_major() {
        let size = [100, 70];
        let rect = |min: [i32; 2], max: [i32; 2]| PixelRect { min, max };
        let a = TileSet::of(&rect([31, 0], [33, 1]), size);
        assert_eq!(a.0.iter().copied().collect::<Vec<_>>(), [0, 1]);
        let b = TileSet::of(&rect([64, 64], [65, 65]), size);
        assert_eq!(
            b.0.iter().copied().collect::<Vec<_>>(),
            [4 * 2 + 2],
            "row 2, column 2"
        );
        let mut need = Need::Part(a.clone());
        need.and(Need::Part(b));
        assert_eq!(need.tiles().flatten().map(|tiles| tiles.0.len()), Some(3));
        need.and(Need::All);
        assert_eq!(need, Need::All);
        let mut none = Need::No;
        none.and(Need::Part(a));
        assert!(matches!(none, Need::Part(_)));
        let off = TileSet::of(&rect([-50, -50], [-10, -10]), size);
        assert!(off.0.is_empty(), "off the canvas");
        let past = TileSet::of(&rect([97, 65], [500, 500]), size);
        assert_eq!(
            past.0.iter().copied().collect::<Vec<_>>(),
            [11],
            "cut to the canvas"
        );
    }
}
