//! Which grid pixels are the object's silhouette (`outline.rs`): over a clear ground the opaque
//! ones, on a plain ground what the flood gave the object (`outline_flood.rs`), specks left out;
//! everything the ground reaches from the picture's edge around them is ground, and the rest is
//! the silhouette. A pocket of ground the object encloses (a handle's loop) stays a hole when it
//! is clearly ground and large enough to draw; smaller or doubtful ones are filled. Then the
//! silhouette's parts: one dominant part on a plain ground, any number of real parts over a clear
//! ground.

use crate::outline::OutlineRefusal;
use crate::outline_flood::{Flooded, object};
use crate::outline_ground::{Difference, Ground};

/// Parts of the object smaller than this share of the grid are specks, left out.
const SPECK: f64 = 0.000_2;
/// A pocket of ground enclosed by the object is a hole when it is at least this share of the
/// silhouette and its pixels differ from the ground on the mean under `HOLE_GROUND` of `low`.
const LEAST_HOLE: f64 = 0.004;
const HOLE_GROUND: f32 = 0.5;
/// On a plain ground the largest part holds at least this share of the silhouette, and a part
/// beside it at most `STRAY`; over a clear ground parts of at least `LEAST_PART` are kept.
const DOMINANT: f64 = 0.88;
const STRAY: f64 = 0.02;
const LEAST_PART: f64 = 0.004;

/// The silhouette on the grid: true inside, holes and ground false.
pub(crate) fn silhouette(
    [width, height]: [usize; 2],
    difference: &Difference,
) -> Result<Vec<bool>, OutlineRefusal> {
    let found = match difference.ground {
        Ground::Clear => (difference.values.iter())
            .map(|v| *v >= difference.low)
            .collect(),
        Ground::Plain { noise } => object(&Flooded {
            size: [width, height],
            lab: &difference.lab,
            values: &difference.values,
            noise,
            high: difference.high,
        }),
    };
    let mut object = vec![false; found.len()];
    let speck = (SPECK * found.len() as f64).ceil() as usize;
    for part in parts([width, height], &found) {
        if part.len() >= speck {
            for i in part {
                object[i] = true;
            }
        }
    }
    if !object.contains(&true) {
        return Err(OutlineRefusal::Nothing);
    }
    let ground = reached_from_edge([width, height], &object);
    let mut inside: Vec<bool> = ground.iter().map(|g| !g).collect();
    let area = inside.iter().filter(|i| **i).count();
    let pockets: Vec<bool> = (0..inside.len()).map(|i| inside[i] && !object[i]).collect();
    for pocket in parts([width, height], &pockets) {
        let mean = pocket.iter().map(|&i| difference.values[i]).sum::<f32>() / pocket.len() as f32;
        let hole =
            pocket.len() as f64 >= LEAST_HOLE * area as f64 && mean < HOLE_GROUND * difference.low;
        if hole {
            for i in pocket {
                inside[i] = false;
            }
        }
    }
    kept_parts([width, height], inside, difference.ground)
}

/// `inside` with only the parts the ground allows: on a plain ground the dominant part, specks
/// beside it dropped; over a clear ground every part of some size.
fn kept_parts(
    size: [usize; 2],
    mut inside: Vec<bool>,
    ground: Ground,
) -> Result<Vec<bool>, OutlineRefusal> {
    let mut found = parts(size, &inside);
    found.sort_by_key(|part| std::cmp::Reverse(part.len()));
    let total: usize = found.iter().map(Vec::len).sum();
    let share = |part: &Vec<usize>| part.len() as f64 / total.max(1) as f64;
    let dropped: Vec<&Vec<usize>> = match ground {
        Ground::Plain { .. } => {
            let dominant = found.first().is_some_and(|part| share(part) >= DOMINANT);
            if !dominant || found.iter().skip(1).any(|part| share(part) > STRAY) {
                return Err(OutlineRefusal::Parts);
            }
            found.iter().skip(1).collect()
        }
        Ground::Clear => found
            .iter()
            .filter(|part| share(part) < LEAST_PART)
            .collect(),
    };
    for part in dropped {
        for &i in part {
            inside[i] = false;
        }
    }
    Ok(inside)
}

/// The 8-connected parts of the true pixels of `mask`, each by its pixels' indices.
pub(crate) fn parts([width, height]: [usize; 2], mask: &[bool]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; mask.len()];
    let mut found = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut part = Vec::new();
        while let Some(i) = stack.pop() {
            part.push(i);
            let (x, y) = (i % width, i / width);
            for (dx, dy) in NEIGHBOURS_8 {
                let (Some(nx), Some(ny)) = (x.checked_add_signed(dx), y.checked_add_signed(dy))
                else {
                    continue;
                };
                let n = ny * width + nx;
                if nx < width && ny < height && mask[n] && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        found.push(part);
    }
    found
}

const NEIGHBOURS_8: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// The pixels not in `object` that the edge reaches through others not in it (4-connected, so
/// the ground never slips between two object pixels meeting at a corner).
fn reached_from_edge([width, height]: [usize; 2], object: &[bool]) -> Vec<bool> {
    let mut reached = vec![false; object.len()];
    let mut stack: Vec<usize> = (0..object.len())
        .filter(|&i| {
            let (x, y) = (i % width, i / width);
            (x == 0 || y == 0 || x + 1 == width || y + 1 == height) && !object[i]
        })
        .collect();
    for &i in &stack {
        reached[i] = true;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % width, i / width);
        let neighbours = [
            (x > 0).then(|| i - 1),
            (x + 1 < width).then(|| i + 1),
            (y > 0).then(|| i - width),
            (y + 1 < height).then(|| i + width),
        ];
        for n in neighbours.into_iter().flatten() {
            if !object[n] && !reached[n] {
                reached[n] = true;
                stack.push(n);
            }
        }
    }
    reached
}
