//! Adjustment previews (S35). While a slider moves, every new value is a version that differs
//! from the one shown only in that adjustment's values (`retuned`): the scene keeps its items
//! and changes those ops in place, the plan stays, and the canvas evaluates through a snapshot
//! of the layer's pixels before the adjustment (`Snapshot`) instead of its ops before it, which
//! do not change while the slider moves. The snapshot holds exactly the floats those ops leave,
//! so a value previewed shows the pixels the committed adjustment shows evaluated cold.

use openpaint_core::{AdjustEdit, Artwork, RegionEdit, StrokeList};

use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::spread;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps, SnapshotUse};
use crate::protocol::{Op, TILE};
use crate::scene::{Item, Scene};
use crate::scene_adjust::EditPlace;
use crate::snapshot::Snapshot;

/// A preview's next value: the adjustment at `place` now has `edit`'s values.
#[derive(Debug, Clone)]
pub(crate) struct Retune {
    pub(crate) place: EditPlace,
    pub(crate) edit: AdjustEdit,
    /// How far either value changes pixels beyond the layer's content, in whole tiles.
    pub(crate) spread: u32,
}

/// How `after` differs from `before` when only one adjustment's values changed (its kind,
/// clip and place kept): everything else the same content, shared.
pub(crate) fn retuned(before: &Artwork, after: &Artwork) -> Option<Retune> {
    if before.same_version(after)
        || before.canvas() != after.canvas()
        || before.layers().len() != after.layers().len()
        || before.groups() != after.groups()
    {
        return None;
    }
    let mut found = Vec::new();
    for (old, new) in before.layers().iter().zip(after.layers()) {
        if old.id() != new.id()
            || old.props() != new.props()
            || !same_strokes(old.strokes(), new.strokes())
        {
            return None;
        }
        for (index, edit) in differing(old.edits(), new.edits())? {
            found.push(((new.id(), false, index), edit));
        }
        match (old.mask(), new.mask()) {
            (None, None) => {}
            (Some(a), Some(b))
                if (a.enabled(), a.inverted()) == (b.enabled(), b.inverted())
                    && same_strokes(a.strokes(), b.strokes()) =>
            {
                for (index, edit) in differing(a.edits(), b.edits())? {
                    found.push(((new.id(), true, index), edit));
                }
            }
            _ => return None,
        }
    }
    match found.as_slice() {
        [(place, [old, new])] => Some(Retune {
            place: *place,
            edit: (*new).clone(),
            spread: spread(old.adjustment()).max(spread(new.adjustment())),
        }),
        _ => None,
    }
}

/// The same strokes in both lists (they only grow, so sharing the last one's samples means
/// sharing all).
fn same_strokes(a: &StrokeList, b: &StrokeList) -> bool {
    a.len() == b.len()
        && match (a.last(), b.last()) {
            (Some(x), Some(y)) => x.shares_samples(y),
            _ => true,
        }
}

/// The edits of `b` that differ from `a`'s at their index in an adjustment's values only;
/// `None` when the lists differ otherwise.
fn differing<'a>(
    a: &'a [RegionEdit],
    b: &'a [RegionEdit],
) -> Option<Vec<(usize, [&'a AdjustEdit; 2])>> {
    if a.len() != b.len() {
        return None;
    }
    let mut out = Vec::new();
    for (index, (x, y)) in a.iter().zip(b).enumerate() {
        if x.same(y) {
            continue;
        }
        let (RegionEdit::Adjust(x), RegionEdit::Adjust(y)) = (x, y) else {
            return None;
        };
        let clip = match (x.clip(), y.clip()) {
            (Some(p), Some(q)) => p.shares(q),
            (None, None) => true,
            _ => false,
        };
        let kind = x.adjustment().kind() == y.adjustment().kind();
        if !clip || !kind || x.after() != y.after() || x.inverted() != y.inverted() {
            return None;
        }
        out.push((index, [x, y]));
    }
    Some(out)
}

/// A previewed layer's pixels before its adjustment at `place`: the scene's items `prefix`
/// evaluated once over the whole tiles they reach into a snapshot (`snapshot.rs`), one float
/// colour per pixel, premultiplied, unrounded.
pub(crate) struct PreviewSnapshot {
    pub(crate) place: EditPlace,
    pub(crate) prefix: std::ops::Range<usize>,
    tiles: Snapshot,
}

impl PreviewSnapshot {
    /// The snapshot of the adjustment at `place` in `scene`, evaluated now through the
    /// canvas's `bindings` (the target keeps its pixels: the evaluation writes the snapshot
    /// instead); `None` when the scene draws it in a way a snapshot cannot stand for, the layer
    /// holds nothing before it, or the snapshot would exceed the device's buffer limits.
    ///
    /// # Errors
    /// As the evaluator's dispatch.
    pub(crate) fn take(
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        (bindings, residuals): (&mut Bindings, [&wgpu::Buffer; 2]),
        place: EditPlace,
    ) -> Result<Option<Self>, RenderError> {
        let Some(prefix) = scene.prefix_of(place) else {
            return Ok(None);
        };
        let Some(rect) = tiles_reached(scene, prefix.clone()) else {
            return Ok(None);
        };
        let mut tiles = Snapshot::new(core, scene.canvas);
        let [min, max] = [rect.min, rect.max].map(|corner| corner.map(i32::unsigned_abs));
        let wanted: Vec<usize> = (min[1]..max[1])
            .step_by(TILE as usize)
            .flat_map(|y| (min[0]..max[0]).step_by(TILE as usize).map(move |x| [x, y]))
            .map(|pixel| tiles.tile_of(pixel))
            .collect();
        if !tiles.take(core, &wanted) {
            return Ok(None);
        }
        let snapshot = Self {
            place,
            prefix,
            tiles,
        };
        let plan = Plan::material(
            scene,
            (snapshot.prefix.clone(), Affine::IDENTITY),
            bindings.size(),
            &[rect],
        );
        core.evaluate.dispatch(
            &core.gpu,
            bindings,
            &Dispatch {
                ops,
                fresh: &[Op::end_layer(1.0)],
                plan: &plan,
                only: None,
                output: Output::CANVAS,
                background: [0.0; 4],
                reset: false,
                residuals,
                base: Affine::IDENTITY,
                snapshot: Some(snapshot.reading(true)),
            },
        )?;
        Ok(Some(snapshot))
    }

    /// The snapshot as a dispatch binds it, written (`writes`) or read.
    pub(crate) fn reading(&self, writes: bool) -> SnapshotUse<'_> {
        self.tiles.reading(writes)
    }
}

/// The whole tiles, inside the canvas, that the items `prefix` can reach; `None` when none.
fn tiles_reached(scene: &Scene, prefix: std::ops::Range<usize>) -> Option<PixelRect> {
    let canvas = PixelRect::full(scene.canvas);
    let reached = scene.items[prefix]
        .iter()
        .filter_map(Item::bounds)
        .filter(|bounds| !bounds.is_empty())
        .map(|bounds| bounds.pixels())
        .reduce(|a, b| a.union(&b))?;
    let side = TILE as i32;
    let aligned = PixelRect {
        min: reached.min.map(|v| v.max(0) / side * side),
        max: reached.max.map(|v| (v.max(0) + side - 1) / side * side),
    };
    let rect = aligned.intersect(&canvas);
    (!rect.is_empty()).then_some(rect)
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
