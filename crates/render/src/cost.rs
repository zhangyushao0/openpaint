//! What evaluating tiles costs the GPU (S53): the frame budget (`pace.rs`) takes background tiles
//! by these estimates. Each op listed in a tile costs, at each of the tile's 256 output pixels,
//! what its kind costs the evaluator on the tablet's GPU (an Adreno 735: measured S55 with the
//! benchmark, `Core::time_evaluation`, at 1x), and the tile itself its loop and its write. A
//! stamped span costs by the stamps a pixel tests (twice its reach over its spacing, at most what
//! it holds). The budget corrects the whole table by the measured time of earlier frames'
//! background work, so the table need only rank the kinds right.

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

/// S55's benchmark after its levers (ns a pixel per op): continuous spans 2.0 to 3.4, stamped
/// spans about 3 plus 1 a disc or square stamp and 2 a texture's (Speckle 4.6 at 1.7 stamps,
/// Flat 10.2 at 6.6, Textured Round 6.2 at 2.5, Opaque Paint 10.1 at 3.1), fills 7 to 24 (their
/// edges), patches 10 to 11, smudges 2.9 to 3.6, filters 1.2 to 3.4, adjustments and ends 1.4
/// to 2.7.
const TABLET: Table = Table {
    tile: 1.0,
    span: 2.5,
    piece: 0.5,
    stamped: 3.0,
    stamp: 1.0,
    texture_stamp: 2.0,
    fill: 10.0,
    patch: 10.0,
    smudge: 3.0,
    filter: 3.0,
    other: 1.5,
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
    item_kinds(scene).map(|kind| TABLET.of(kind)).collect()
}

/// Per item of `scene`, its work at a pixel (a stamped span's from its stroke's paint op).
pub(crate) fn item_kinds(scene: &Scene) -> impl Iterator<Item = OpKind> + '_ {
    let items = &scene.items;
    (0..items.len()).map(|index| {
        let op = items[index].op();
        let paint = op.stamp_paint().and_then(|offset| {
            let at = isize::try_from(index).ok()? + isize::try_from(offset).ok()?;
            items
                .get(usize::try_from(at).ok()?)
                .map(crate::scene::Item::op)
        });
        op.kind(paint.as_ref())
    })
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
