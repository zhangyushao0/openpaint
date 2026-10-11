//! The in-progress stroke's layer kept as a snapshot (S48, DECISIONS D75; D61's mechanism for
//! the active stroke). While a stroke is drawn, every frame evaluates the tiles its tail
//! reaches; without this, each of them walks every finished op of every layer there, so a frame
//! costs more with every stroke painted over the place. Instead the layer's items before the
//! stroke's place are evaluated once per tile into a float snapshot (`snapshot.rs`), the first
//! time a frame reaches the tile, and every frame resumes the layer from it (`OP_RESUME`) and
//! adds only the stroke: the same floats, so every frame equals the stroke evaluated cold.
//!
//! The snapshot outlives the stroke: strokes committed to the layer are laid over a tile it holds
//! (`Plan::appended`) when a later frame needs that tile, so painting on keeps every tile taken
//! and no frame lays a whole stroke at once. Any other change to the layer's content (undo, an
//! edit, another layer or the mask) starts it over, empty.

use std::ops::Range;

use openpaint_core::{Artwork, LayerId, RegionEdit, StrokeList};

use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps, SnapshotUse};
use crate::protocol::{Op, TILE};
use crate::scene::{ActivePlace, Item, Scene};
use crate::snapshot::Snapshot;

/// The snapshot of one layer's paint or mask before the in-progress stroke.
pub(crate) struct ActiveSnapshot {
    layer: LayerId,
    on_mask: bool,
    /// The version whose content the tiles hold (it keeps that content's buffers alive).
    artwork: Artwork,
    /// The scene items that content made (counted from its first), and the last of them.
    items: usize,
    last: Option<Item>,
    /// Per canvas tile held, how many of those items its pixels hold: fewer while it lacks
    /// strokes committed after a frame last needed it.
    applied: Vec<u32>,
    tiles: Snapshot,
}

/// How the content now relates to the content the snapshot holds.
enum Since {
    Same,
    /// It gained this many strokes, the rest the same.
    Appended(usize),
    Other,
}

/// What a scene, its GPU ops and the canvas's bindings give a snapshot's evaluations.
pub(crate) type Evaluating<'a, 'b> = (
    (&'a Scene, &'a SceneOps),
    (&'b mut Bindings, [&'a wgpu::Buffer; 2]),
);

/// What one frame did to the snapshot: the tiles it took (evaluating the layer's items into
/// them), and the tiles it laid strokes committed since into.
pub(crate) type Kept = [usize; 2];

/// What a frame resumes from: the layer's items the snapshot stands for, and the snapshot.
pub(crate) type Resumed<'h> = Option<(Range<usize>, SnapshotUse<'h>)>;

/// Brings `held` up to date with `artwork` and the in-progress stroke's `place` in `scene`, and
/// makes it hold every tile of `regions`; returns the items the frame resumes from it and the
/// snapshot, or `None` when the frame evaluates every op (the layer holds nothing before the
/// stroke, or the device's limits refuse more tiles), and what it did.
///
/// # Errors
/// As the evaluator's dispatch; `held` is emptied then, as its tiles may be half written.
pub(crate) fn prepare<'h>(
    held: &'h mut Option<ActiveSnapshot>,
    core: &Core,
    evaluating: Evaluating<'_, '_>,
    (artwork, place): (&Artwork, &ActivePlace),
    regions: &[PixelRect],
) -> Result<(Resumed<'h>, Kept), RenderError> {
    let prefix = match evaluating.0.0.active_before.clone() {
        Some(prefix) if !prefix.is_empty() => prefix,
        _ => return Ok((None, [0; 2])),
    };
    let result = bring_up(held, core, evaluating, (artwork, place), (&prefix, regions));
    match result {
        Ok(Some(kept)) => {
            let resumed = held
                .as_ref()
                .map(|snapshot| (prefix, snapshot.tiles.reading(false)));
            Ok((resumed, kept))
        }
        Ok(None) => Ok((None, [0; 2])),
        Err(error) => {
            *held = None;
            Err(error)
        }
    }
}

/// `prepare`'s work; `None` when the tiles cannot all be held.
fn bring_up(
    held: &mut Option<ActiveSnapshot>,
    core: &Core,
    ((scene, ops), (bindings, residuals)): Evaluating<'_, '_>,
    (artwork, place): (&Artwork, &ActivePlace),
    (prefix, regions): (&Range<usize>, &[PixelRect]),
) -> Result<Option<Kept>, RenderError> {
    let kept = held
        .take()
        .filter(|s| s.layer == place.layer && s.on_mask == place.on_mask);
    let mut snapshot = match kept {
        Some(snapshot) => match snapshot.since(artwork, scene, prefix) {
            Since::Same => snapshot,
            // Its tiles take the strokes committed since when a frame needs them.
            Since::Appended(strokes) if snapshot.appended(scene, prefix, strokes) => snapshot,
            _ => snapshot.emptied(),
        },
        None => {
            let Some(tiles) = Snapshot::new(core, scene.canvas, bindings.snapshot_room) else {
                return Ok(None);
            };
            ActiveSnapshot {
                layer: place.layer,
                on_mask: place.on_mask,
                artwork: artwork.clone(),
                items: 0,
                last: None,
                applied: vec![0; tiles.count()],
                tiles,
            }
        }
    };
    snapshot.artwork = artwork.clone();
    snapshot.items = prefix.len();
    snapshot.last = scene.items.get(prefix.end.wrapping_sub(1)).cloned();
    let evaluating = ((scene, ops), (bindings, residuals));
    let kept = snapshot.take(core, evaluating, (prefix, regions));
    *held = Some(snapshot);
    kept
}

impl ActiveSnapshot {
    /// Bytes its buffer holds.
    pub(crate) fn bytes(&self) -> u64 {
        self.tiles.bytes()
    }

    /// How the layer's content in `artwork` relates to the content held; the same only when its
    /// items in `scene`, `prefix`, are as many as before and end alike.
    fn since(&self, artwork: &Artwork, scene: &Scene, prefix: &Range<usize>) -> Since {
        let (Some(held), Some(now)) = (
            content(&self.artwork, self.layer, self.on_mask),
            content(artwork, self.layer, self.on_mask),
        ) else {
            return Since::Other;
        };
        let edits = std::ptr::eq(held.1.as_ptr(), now.1.as_ptr()) && held.1.len() == now.1.len();
        if !edits {
            return Since::Other;
        }
        if held.0.same(now.0) {
            let alike =
                self.items == prefix.len() && self.last.as_ref() == scene.items.get(prefix.end - 1);
            return if alike { Since::Same } else { Since::Other };
        }
        let grew = now.0.len() > held.0.len()
            && held.0.last().is_none_or(|last| {
                now.0
                    .get(held.0.len() - 1)
                    .is_some_and(|stroke| stroke.shares_samples(last))
            });
        if grew {
            Since::Appended(now.0.len() - held.0.len())
        } else {
            Since::Other
        }
    }

    /// Whether the items of `prefix` after those held are exactly `strokes` whole strokes and
    /// those held end as they did: what `take` can lay over the tiles.
    fn appended(&self, scene: &Scene, prefix: &Range<usize>, strokes: usize) -> bool {
        if self.items == 0 || self.items >= prefix.len() {
            return false;
        }
        let boundary = prefix.start + self.items;
        if self.last.as_ref() != scene.items.get(boundary - 1) {
            return false;
        }
        let gained = &scene.items[boundary..prefix.end];
        let strokes_only = gained.iter().all(|item| {
            matches!(
                item,
                Item::Segment { .. } | Item::EndStroke(_) | Item::Reach(_) | Item::Smudge(_)
            )
        });
        let ends = gained
            .iter()
            .filter(|item| matches!(item, Item::EndStroke(_) | Item::Smudge(_)))
            .count();
        strokes_only
            && ends == strokes
            && matches!(gained.last(), Some(Item::EndStroke(_) | Item::Smudge(_)))
    }

    /// The same layer's snapshot holding nothing (its buffer kept for the tiles taken next).
    fn emptied(mut self) -> Self {
        self.tiles.clear();
        self.items = 0;
        self.last = None;
        self
    }

    /// Brings the tiles of `regions` up to date with the layer's items `prefix`: a tile held
    /// that lacks the strokes committed since it was last brought up gets them laid over it
    /// (`Plan::appended`), a tile not held yet is taken and gets every item evaluated into it.
    /// Returns how many of each, or `None` when the device's limits refuse the new tiles.
    fn take(
        &mut self,
        core: &Core,
        ((scene, ops), (bindings, residuals)): Evaluating<'_, '_>,
        (prefix, regions): (&Range<usize>, &[PixelRect]),
    ) -> Result<Option<Kept>, RenderError> {
        let items = u32::try_from(prefix.len()).unwrap_or(u32::MAX);
        let behind = |tile: usize| self.tiles.holds(tile) && self.applied[tile] < items;
        let stale = self.tiles_of(regions, behind);
        let mut starts: Vec<u32> = stale.iter().map(|(tile, _)| self.applied[*tile]).collect();
        starts.sort_unstable();
        starts.dedup();
        let resumed = [Op::end_layer(1.0), Op::resume()];
        for start in starts {
            let group: Vec<PixelRect> = (stale.iter())
                .filter(|(tile, _)| self.applied[*tile] == start)
                .map(|(_, rect)| *rect)
                .collect();
            let gained = prefix.start + start as usize..prefix.end;
            let plan = Plan::appended(scene, gained, bindings.size(), &group);
            self.write(core, (ops, &mut *bindings, residuals), (&plan, &resumed))?;
        }
        let wanted = self.tiles_of(regions, |tile| !self.tiles.holds(tile));
        let numbers: Vec<usize> = wanted.iter().map(|(tile, _)| *tile).collect();
        if !wanted.is_empty() {
            if !self.tiles.take(core, &numbers, bindings.snapshot_room) {
                return Ok(None);
            }
            let rects: Vec<PixelRect> = wanted.iter().map(|(_, rect)| *rect).collect();
            let size = bindings.size();
            let plan = Plan::material(scene, (prefix.clone(), Affine::IDENTITY), size, &rects);
            self.write(
                core,
                (ops, bindings, residuals),
                (&plan, &[Op::end_layer(1.0)]),
            )?;
        }
        for tile in stale.iter().map(|(tile, _)| *tile).chain(numbers) {
            self.applied[tile] = items;
        }
        Ok(Some([wanted.len(), stale.len()]))
    }

    /// The canvas tiles meeting `regions` that `keep` keeps (by number, row major), and their
    /// rectangles.
    fn tiles_of(
        &self,
        regions: &[PixelRect],
        keep: impl Fn(usize) -> bool,
    ) -> Vec<(usize, PixelRect)> {
        let side = TILE as i32;
        let size = self.artwork.canvas();
        let canvas = PixelRect::full([size.width(), size.height()]);
        let mut seen = std::collections::BTreeMap::new();
        for region in regions {
            let cut = region.intersect(&canvas);
            if cut.is_empty() {
                continue;
            }
            for y in (cut.min[1] / side)..(cut.max[1] + side - 1) / side {
                for x in (cut.min[0] / side)..(cut.max[0] + side - 1) / side {
                    let tile = self
                        .tiles
                        .tile_of([x * side, y * side].map(i32::unsigned_abs));
                    if keep(tile) {
                        let rect = PixelRect {
                            min: [x * side, y * side],
                            max: [(x + 1) * side, (y + 1) * side],
                        };
                        seen.insert(tile, rect);
                    }
                }
            }
        }
        seen.into_iter().collect()
    }

    /// Evaluates `plan` with `fresh` into the snapshot's tiles instead of the target.
    fn write(
        &self,
        core: &Core,
        (ops, bindings, residuals): (&SceneOps, &mut Bindings, [&wgpu::Buffer; 2]),
        (plan, fresh): (&Plan, &[Op]),
    ) -> Result<(), RenderError> {
        core.evaluate.dispatch(
            &core.gpu,
            bindings,
            &Dispatch {
                ops,
                fresh,
                plan,
                only: None,
                output: Output::CANVAS,
                background: [0.0; 4],
                reset: false,
                residuals,
                base: Affine::IDENTITY,
                snapshot: Some(self.tiles.reading(true)),
            },
        )
    }
}

/// The strokes and edits of `layer`'s paint, or (`on_mask`) its mask, in `artwork`.
fn content(
    artwork: &Artwork,
    layer: LayerId,
    on_mask: bool,
) -> Option<(&StrokeList, &[RegionEdit])> {
    let layer = artwork.layer(layer)?;
    if on_mask {
        let mask = layer.mask()?;
        return Some((mask.strokes(), mask.edits()));
    }
    Some((layer.strokes(), layer.edits()))
}
