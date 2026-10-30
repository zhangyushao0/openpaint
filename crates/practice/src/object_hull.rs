//! An object's parts built (S94, `object_body.rs`): the box's blocks left once the changes took
//! theirs out, each cut by the changes' planes, as convex hulls with their corners and edges (in
//! the box's own coordinates), and a round part in its block.

use crate::object_body::{Hull, Placed, Round};
use crate::object_design::{Block, Local, SAME, Turned};
use crate::solid_answer::Square;
use crate::vector::{add, cross, dot, norm, sub, times, unit};

/// `whole` with `out` taken out: up to six blocks, the lowest whole first (along height, then
/// width, then depth).
pub(crate) fn minus(whole: Block, out: &Block) -> Vec<Block> {
    let overlaps =
        (0..3).all(|k| out.low[k] < whole.high[k] - SAME && out.high[k] > whole.low[k] + SAME);
    if !overlaps {
        return vec![whole];
    }
    let mut rest = whole;
    let mut pieces = Vec::new();
    for k in [1, 0, 2] {
        if out.low[k] > rest.low[k] + SAME {
            let mut piece = rest;
            piece.high[k] = out.low[k];
            pieces.push(piece);
            rest.low[k] = out.low[k];
        }
        if out.high[k] < rest.high[k] - SAME {
            let mut piece = rest;
            piece.low[k] = out.high[k];
            pieces.push(piece);
            rest.high[k] = out.high[k];
        }
    }
    pieces
}

/// `block` cut by `cuts` (planes in the box's own coordinates) as a hull in the world; none when
/// nothing of it is left.
pub(crate) fn hull(block: &Block, cuts: &[(Local, f64)], placed: Placed) -> Option<Hull> {
    let mut own: Vec<(Local, f64)> = Vec::new();
    for k in 0..3 {
        let mut axis = [0.0; 3];
        axis[k] = 1.0;
        own.push((axis, block.high[k]));
        own.push((axis.map(|v| -v), -block.low[k]));
    }
    own.extend_from_slice(cuts);
    let corners = corners_of(&own);
    if corners.len() < 4 {
        return None;
    }
    let edges = edges_of(&own, &corners);
    let planes = own.into_iter().map(|plane| placed.plane(plane)).collect();
    Some(Hull {
        planes,
        corners,
        edges,
        surfaces: Vec::new(),
    })
}

/// Where three of `own`'s planes meet inside all of them, each once.
fn corners_of(own: &[(Local, f64)]) -> Vec<Local> {
    let keeps = |p: Local| (own.iter()).all(|(n, d)| dot(*n, p) - d <= SAME * norm(*n).max(1.0));
    let mut corners: Vec<Local> = Vec::new();
    for i in 0..own.len() {
        for j in (i + 1)..own.len() {
            for k in (j + 1)..own.len() {
                let Some(p) = meeting([own[i], own[j], own[k]]) else {
                    continue;
                };
                if keeps(p) && !corners.iter().any(|q| norm(sub(*q, p)) <= 1e-9) {
                    corners.push(p);
                }
            }
        }
    }
    corners
}

/// The edges of the hull of `own`'s planes with `corners`: two planes' shared corners, the two
/// farthest apart along where they meet, each edge once.
fn edges_of(own: &[(Local, f64)], corners: &[Local]) -> Vec<[Local; 2]> {
    let on = |(n, d): (Local, f64), p: Local| (dot(n, p) - d).abs() <= 1e-9 * norm(n).max(1.0);
    let mut edges = Vec::new();
    for i in 0..own.len() {
        for j in (i + 1)..own.len() {
            let along = cross(own[i].0, own[j].0);
            if norm(along) <= 1e-12 {
                continue;
            }
            let shared: Vec<Local> = (corners.iter().copied())
                .filter(|p| on(own[i], *p) && on(own[j], *p))
                .collect();
            let by = |p: &Local| dot(*p, along);
            let low = shared.iter().min_by(|a, b| by(a).total_cmp(&by(b)));
            let high = shared.iter().max_by(|a, b| by(a).total_cmp(&by(b)));
            if let (Some(low), Some(high)) = (low, high)
                && norm(sub(*high, *low)) > 1e-9
                && !edges
                    .iter()
                    .any(|e: &[Local; 2]| same_edge(e, [*low, *high]))
            {
                edges.push([*low, *high]);
            }
        }
    }
    edges
}

fn same_edge(e: &[Local; 2], f: [Local; 2]) -> bool {
    let near = |a: Local, b: Local| norm(sub(a, b)) <= 1e-9;
    (near(e[0], f[0]) && near(e[1], f[1])) || (near(e[0], f[1]) && near(e[1], f[0]))
}

/// Where three planes meet; none when two are parallel.
fn meeting(planes: [(Local, f64); 3]) -> Option<Local> {
    let [(a, da), (b, db), (c, dc)] = planes;
    let det = dot(a, cross(b, c));
    if det.abs() < 1e-12 {
        return None;
    }
    let sum = add(
        add(times(cross(b, c), da), times(cross(c, a), db)),
        times(cross(a, b), dc),
    );
    Some(times(sum, 1.0 / det))
}

/// `turned` in the world: its block's face at the axis's low end its base.
pub(crate) fn round(turned: &Turned, placed: Placed) -> Round {
    let Block { low, high } = turned.block;
    let [first, second] = match turned.axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    let edge = |k: usize| times(placed.axes[k], high[k] - low[k]);
    let along = edge(turned.axis);
    Round {
        base: Square {
            corner: placed.world(low),
            u: edge(first),
            v: edge(second),
        },
        axis: unit(along),
        length: norm(along),
        cone: turned.cone,
    }
}
