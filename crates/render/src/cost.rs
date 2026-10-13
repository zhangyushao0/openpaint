//! What evaluating tiles costs the GPU (S53): the frame budget (`pace.rs`) takes background tiles
//! by these estimates. Each op listed in a tile costs, at each of the tile's 256 output pixels,
//! what its kind costs the evaluator on the tablet's GPU (an Adreno 735; measured S53), and the
//! tile itself its loop and its write. A stamped span costs by the stamps a pixel tests (twice its
//! reach over its spacing, at most what it holds): S49 costed every op as the costliest, a
//! Textured Round span, about 33 ns a pixel. The budget corrects the whole table by the measured
//! time of earlier frames' background work, so the table need only rank the kinds right.

use crate::plan::Plan;
use crate::protocol::{FRESH, OpKind, TILE, TileRecord};
use crate::scene::Scene;

/// Nanoseconds at one output pixel on the tablet, by what is evaluated there.
#[derive(Debug, Clone, Copy)]
struct Table {
    tile: f32,
    span: f32,
    piece: f32,
    stamped: f32,
    stamp: f32,
    texture_stamp: f32,
    fill: f32,
    patch: f32,
    smudge: f32,
    filter: f32,
    other: f32,
}

const TABLET: Table = Table {
    tile: 1.0,
    span: 3.0,
    piece: 0.5,
    stamped: 4.0,
    stamp: 1.0,
    texture_stamp: 2.0,
    fill: 3.0,
    patch: 3.0,
    smudge: 6.0,
    filter: 3.0,
    other: 0.3,
};

/// Output pixels in a plan tile.
const PIXELS: f64 = (TILE * TILE) as f64;

impl Table {
    fn of(&self, kind: OpKind) -> f32 {
        match kind {
            OpKind::Span { pieces } => self.piece.mul_add(pieces, self.span),
            OpKind::Stamped { stamps, texture } => {
                let each = if texture {
                    self.texture_stamp
                } else {
                    self.stamp
                };
                each.mul_add(stamps, self.stamped)
            }
            OpKind::Fill => self.fill,
            OpKind::Patch => self.patch,
            OpKind::Smudge => self.smudge,
            OpKind::Filter => self.filter,
            OpKind::Other => self.other,
        }
    }
}

/// Per item of `scene`, what it costs at one output pixel of a tile that lists it, in ns.
pub(crate) fn item_costs(scene: &Scene) -> Vec<f32> {
    let items = &scene.items;
    (0..items.len())
        .map(|index| {
            let op = items[index].op();
            let paint = op.stamp_paint().and_then(|offset| {
                let at = isize::try_from(index).ok()? + isize::try_from(offset).ok()?;
                items
                    .get(usize::try_from(at).ok()?)
                    .map(crate::scene::Item::op)
            });
            TABLET.of(op.kind(paint.as_ref()))
        })
        .collect()
}

/// What evaluating `tile` of `plan` costs, in ns: its ops costed by `items` (a fresh op, the
/// in-progress stroke's, as a plain span), and the tile itself.
pub(crate) fn tile_cost(plan: &Plan, tile: &TileRecord, items: &[f32]) -> f64 {
    let start = tile.start as usize;
    let listed = &plan.indices[start..start + tile.count as usize];
    let ops: f32 = listed
        .iter()
        .map(|&entry| {
            if entry >= FRESH {
                TABLET.span
            } else {
                items.get(entry as usize).copied().unwrap_or(TABLET.span)
            }
        })
        .sum();
    f64::from(ops + TABLET.tile) * PIXELS
}
