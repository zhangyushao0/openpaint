//! A silhouette from a segmentation's matte (S81): how the pack builder outlines an official
//! picture whose ground is not clear. A salient-object model runs in the builder, never here and
//! never on the device; here only its matte (each place's chance of being the object) is cleaned,
//! by rules the contact sheets show, and traced as S80's outlines are (`outline.rs`):
//!
//! - the matte read at each grid pixel's centre, between its cells, the object where it is at
//!   least a half;
//! - the largest part kept, with the parts beside it (within `NEAR` of it, at least `LEAST_PART`
//!   of it) where the object comes in pieces; specks and far strays dropped, and a far part of
//!   some size refuses, since then the matte holds more than one object;
//! - what the ground does not reach from the picture's edge is the object's, but for pockets of
//!   ground large enough to draw (`LEAST_HOLE`): those stay holes (a handle's loop, a key's bow);
//! - the contour traced at the half (`outline_trace.rs`), its stairs smoothed by a grid pixel, so
//!   a real corner stays a corner; the same checks of size, cut and raggedness as S80's.
//!
//! A picture whose edge is clear (transparent) carries its silhouette in its own alpha, exactly:
//! S80's derivation is taken for it, whatever the matte says.

use std::cmp::Reverse;
use std::collections::VecDeque;

use crate::outlines::outline::{Outline, OutlineRefusal, check_silhouette, outline, traced};
use crate::outlines::outline_ground::{Grid, clear, grid};
use crate::outlines::outline_mask::{parts, reached_from_edge};

/// The matte's level at which a place is the object's.
const HALF: f32 = 0.5;
/// Parts kept beside the largest: within this share of the grid's long side of it, and at least
/// `LEAST_PART` of its size; a part dropped of at least `STRAY` of it refuses.
const NEAR: f64 = 0.03;
const LEAST_PART: f64 = 0.01;
const STRAY: f64 = 0.05;
/// A pocket of ground the object encloses stays a hole when it holds at least this share of
/// the silhouette; a smaller one is filled.
const LEAST_HOLE: f64 = 0.002;

/// A segmentation's matte over a whole picture: `size` cells at any resolution (the picture is
/// stretched onto them), rows top first, each the chance from 0 to 1 that the object covers it.
#[derive(Debug, Clone, PartialEq)]
pub struct Matte {
    pub size: [u32; 2],
    pub values: Vec<f32>,
}

impl Matte {
    /// The matte at `[u, v]`, the picture's width and height as 0 to 1, read between the four
    /// nearest cells' centres.
    fn at(&self, [u, v]: [f64; 2]) -> f32 {
        let [width, height] = self.size.map(|side| side as usize);
        let place = |t: f64, side: usize| (t * side as f64 - 0.5).clamp(0.0, (side - 1) as f64);
        let [x, y] = [place(u, width), place(v, height)];
        let [left, top] = [x.floor() as usize, y.floor() as usize];
        let [right, bottom] = [(left + 1).min(width - 1), (top + 1).min(height - 1)];
        let [fx, fy] = [(x - left as f64) as f32, (y - top as f64) as f32];
        let cell = |cx: usize, cy: usize| self.values[cy * width + cx];
        let upper = cell(left, top) + fx * (cell(right, top) - cell(left, top));
        let lower = cell(left, bottom) + fx * (cell(right, bottom) - cell(left, bottom));
        upper + fy * (lower - upper)
    }
}

/// The silhouette outline of the picture `rgba` (8-bit sRGB, straight alpha, `size` pixels) as
/// `matte` finds it; over a clear ground, the picture's own (`outline`).
///
/// # Errors
/// Why it has none (`OutlineRefusal`): `Pixels` also for a matte of no cells or not of its size.
pub fn outline_of_matte(
    size: [u32; 2],
    rgba: &[u8],
    matte: &Matte,
) -> Result<Outline, OutlineRefusal> {
    let grid = grid(size, rgba).ok_or(OutlineRefusal::Pixels)?;
    if clear(&grid) {
        return outline(size, rgba);
    }
    let [width, height] = matte.size.map(|side| side as usize);
    if width == 0 || height == 0 || matte.values.len() != width * height {
        return Err(OutlineRefusal::Pixels);
    }
    let cells = [grid.width, grid.height];
    let values = sampled(&grid, matte);
    let inside = cleaned(cells, &values)?;
    check_silhouette(cells, &inside, true)?;
    traced(size, &grid, &inside)
}

/// The matte at every grid pixel's centre.
fn sampled(grid: &Grid, matte: &Matte) -> Vec<f32> {
    let [width, height] = [grid.width, grid.height];
    (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let u = (x as f64 + 0.5) / width as f64;
            let v = (y as f64 + 0.5) / height as f64;
            matte.at([u, v])
        })
        .collect()
}

/// The silhouette the matte's `values` on the grid give: the object's parts as kept, and every
/// enclosed pocket but the clear holes.
fn cleaned(size: [usize; 2], values: &[f32]) -> Result<Vec<bool>, OutlineRefusal> {
    let found: Vec<bool> = values.iter().map(|v| *v >= HALF).collect();
    let mut pieces = parts(size, &found);
    pieces.sort_by_key(|piece| Reverse(piece.len()));
    let Some(largest) = pieces.first() else {
        return Err(OutlineRefusal::Nothing);
    };
    let reach = (NEAR * size[0].max(size[1]) as f64).round() as usize;
    let near = within(size, largest, reach);
    let mut object = vec![false; values.len()];
    for (index, piece) in pieces.iter().enumerate() {
        let share = piece.len() as f64 / largest.len() as f64;
        let beside = share >= LEAST_PART && piece.iter().any(|&i| near[i]);
        if index == 0 || beside {
            for &i in piece {
                object[i] = true;
            }
        } else if share >= STRAY {
            return Err(OutlineRefusal::Parts);
        }
    }
    let ground = reached_from_edge(size, &object);
    let mut inside: Vec<bool> = ground.iter().map(|g| !g).collect();
    let area = inside.iter().filter(|i| **i).count() as f64;
    let pockets: Vec<bool> = (0..inside.len()).map(|i| inside[i] && !object[i]).collect();
    for pocket in parts(size, &pockets) {
        if pocket.len() as f64 >= LEAST_HOLE * area {
            for i in pocket {
                inside[i] = false;
            }
        }
    }
    Ok(inside)
}

/// The grid pixels within `reach` steps (8-connected) of `part`'s.
fn within([width, height]: [usize; 2], part: &[usize], reach: usize) -> Vec<bool> {
    let mut steps = vec![usize::MAX; width * height];
    let mut queue: VecDeque<usize> = part.iter().copied().collect();
    for &i in part {
        steps[i] = 0;
    }
    while let Some(i) = queue.pop_front() {
        if steps[i] >= reach {
            continue;
        }
        let (x, y) = (i % width, i / width);
        for ny in y.saturating_sub(1)..(y + 2).min(height) {
            for nx in x.saturating_sub(1)..(x + 2).min(width) {
                let n = ny * width + nx;
                if steps[n] == usize::MAX {
                    steps[n] = steps[i] + 1;
                    queue.push_back(n);
                }
            }
        }
    }
    steps.into_iter().map(|s| s <= reach).collect()
}
