//! The regions of a picture's cells as a found answer groups them into areas
//! (`areas_found.rs`, S101): each cell's region (`of`), numbered from 0, over a grid of cells
//! whose squinted colours are given (Oklab). Bands and specks dissolved into the regions beside
//! them, neighbours a step apart merged.

use std::collections::{BTreeMap, BTreeSet};

use crate::areas::{CORE, LEAST_CORE, LEAST_SHARE, MOST_AREAS, depth};

/// Neighbouring areas nearer than this in Oklab are one: under a just-noticeable step.
pub const ONE_STEP: f64 = 0.02;
/// A region whose cells all lie within this many cells of its edge, but for fewer than
/// `LEAST_CORE`, is a band along an edge (a soft edge's blend, a seam), not an area.
const BAND: usize = 3;

/// How far either side of an edge its step is read, in cells (about a tenth of the picture), and
/// the least share of two regions' difference their edge must step for the eye to see an edge.
const REACH: usize = 8;
const SHARP: f64 = 0.35;

/// A colour in Oklab.
pub(crate) type Lab = [f64; 3];

/// How far apart two colours are in Oklab.
pub(crate) fn distance(a: &Lab, b: &Lab) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The regions `of` (`count` of them, over `grid` cells whose colours are `lab`) with every
/// band and every region too small dissolved and every pair of neighbours a step apart merged,
/// until none is left.
pub(crate) fn grouped(
    (mut of, mut count): (Vec<usize>, usize),
    lab: &[Lab],
    grid: [usize; 2],
) -> (Vec<usize>, usize) {
    loop {
        let dissolved = dissolved(&of, count, lab, grid);
        if let Some(regions) = &dissolved {
            (of, count) = regions.clone();
        }
        let merged = merged(&of, count, lab, grid);
        if let Some(regions) = &merged {
            (of, count) = regions.clone();
        }
        if dissolved.is_none() && merged.is_none() {
            return (of, count);
        }
    }
}

/// The colour of each of `count` regions (`of` each cell's, over `grid`) as a squinting eye
/// gives it: the median, channel by channel, of its cells deeper than `CORE`, past any soft
/// edge's blend (of all its cells while it has no such cell).
fn core_colours(of: &[usize], count: usize, (lab, grid): (&[Lab], [usize; 2])) -> Vec<Lab> {
    let deep = depth(of, grid);
    let mut inside: Vec<Vec<Lab>> = vec![Vec::new(); count];
    let mut all: Vec<Vec<Lab>> = vec![Vec::new(); count];
    for ((region, cell), depth) in of.iter().zip(lab).zip(&deep) {
        all[*region].push(*cell);
        if usize::from(*depth) > CORE {
            inside[*region].push(*cell);
        }
    }
    (inside.into_iter().zip(all))
        .map(|(inside, all)| {
            let mut cells = if inside.is_empty() { all } else { inside };
            std::array::from_fn(|k| {
                cells.sort_unstable_by(|a, b| a[k].total_cmp(&b[k]));
                cells.get(cells.len() / 2).map_or(0.0, |c| c[k])
            })
        })
        .collect()
}

/// The cells beside `cell` across and down, of a grid of `[w, h]`.
fn beside(cell: usize, [w, h]: [usize; 2]) -> impl Iterator<Item = usize> {
    let (x, y) = (cell % w, cell / w);
    [
        (x > 0).then(|| cell - 1),
        (x + 1 < w).then_some(cell + 1),
        (y > 0).then(|| cell - w),
        (y + 1 < h).then_some(cell + w),
    ]
    .into_iter()
    .flatten()
}

/// How many cells each region holds, how many lie deeper than `BAND` and than `CORE`.
fn depths(of: &[usize], count: usize, grid: [usize; 2]) -> [Vec<usize>; 3] {
    let mut counts = [
        vec![0usize; count],
        vec![0usize; count],
        vec![0usize; count],
    ];
    for (region, depth) in of.iter().zip(depth(of, grid)) {
        counts[0][*region] += 1;
        counts[1][*region] += usize::from(usize::from(depth) > BAND);
        counts[2][*region] += usize::from(usize::from(depth) > CORE);
    }
    counts
}

/// Each region's neighbours (sharing a side of a cell), in order.
fn adjacent(of: &[usize], count: usize, grid: [usize; 2]) -> Vec<BTreeSet<usize>> {
    let mut meet = vec![BTreeSet::new(); count];
    for cell in 0..of.len() {
        for other in beside(cell, grid).filter(|n| of[*n] != of[cell]) {
            meet[of[cell]].insert(of[other]);
        }
    }
    meet
}

/// Whether `colour` lies between `a` and `b`, as a soft edge's blend of them does: along the
/// line from one to the other, well inside it, nearer it than a sixth of their distance (half a
/// step at least: sRGB's blend bows a little off Oklab's line).
fn between(colour: &Lab, [a, b]: [&Lab; 2]) -> bool {
    let along: Lab = std::array::from_fn(|k| b[k] - a[k]);
    let length = distance(a, b);
    if length < ONE_STEP {
        return false;
    }
    let t = (0..3).map(|k| (colour[k] - a[k]) * along[k]).sum::<f64>() / (length * length);
    let on: Lab = std::array::from_fn(|k| a[k] + t * along[k]);
    (0.15..=0.85).contains(&t) && distance(colour, &on) <= (length / 6.0).max(ONE_STEP / 2.0)
}

/// The regions `of` with every band dissolved into the regions beside it (one with no inside, or
/// without a core whose colour lies between two of its neighbours': a soft edge's blend), or once
/// none is left every region too small (whose bands came back to it first); none when every
/// region keeps (the largest keeps when none would).
fn dissolved(
    of: &[usize],
    count: usize,
    lab: &[Lab],
    grid: [usize; 2],
) -> Option<(Vec<usize>, usize)> {
    let [sizes, inner, cores] = depths(of, count, grid);
    let colours = core_colours(of, count, (lab, grid));
    let meet = adjacent(of, count, grid);
    let blend = |r: usize| {
        let near: Vec<&usize> = meet[r].iter().collect();
        let pairs = (0..near.len()).flat_map(|i| (i + 1..near.len()).map(move |j| (i, j)));
        cores[r] < LEAST_CORE
            && pairs
                .into_iter()
                .any(|(i, j)| between(&colours[r], [&colours[*near[i]], &colours[*near[j]]]))
    };
    let mut kept: Vec<bool> = (0..count)
        .map(|r| inner[r] >= LEAST_CORE && !blend(r))
        .collect();
    if kept.iter().all(|k| *k) {
        let least = (LEAST_SHARE * of.len() as f64).ceil() as usize;
        kept = (0..count).map(|r| sizes[r] >= least).collect();
    }
    if kept.iter().all(|k| *k) {
        return None;
    }
    if !kept.iter().any(|k| *k) {
        let largest = (0..count)
            .max_by_key(|r| (sizes[*r], usize::MAX - r))
            .unwrap_or(0);
        kept[largest] = true;
    }
    let given = reassigned(of, &kept, (&colours, lab), grid);
    Some(compact(&given, count))
}

/// Each cell of a region not `kept`, from the outside in, given to the kept region beside it
/// whose colour (`colours`) lies nearest its own (`lab`).
fn reassigned(
    of: &[usize],
    kept: &[bool],
    (colours, lab): (&[Lab], &[Lab]),
    grid: [usize; 2],
) -> Vec<usize> {
    let mut given: Vec<Option<usize>> = of.iter().map(|r| kept[*r].then_some(*r)).collect();
    loop {
        let mut taken = Vec::new();
        for cell in (0..of.len()).filter(|c| given[*c].is_none()) {
            let nearest = (beside(cell, grid).filter_map(|n| given[n])).min_by(|a, b| {
                let off = |r: &usize| distance(&lab[cell], &colours[*r]);
                off(a).total_cmp(&off(b)).then(a.cmp(b))
            });
            if let Some(region) = nearest {
                taken.push((cell, region));
            }
        }
        if taken.is_empty() {
            break;
        }
        for (cell, region) in taken {
            given[cell] = Some(region);
        }
    }
    given.into_iter().map(|r| r.unwrap_or(0)).collect()
}

/// The regions `of` numbered from 0 without gaps, and how many there are.
fn compact(of: &[usize], count: usize) -> (Vec<usize>, usize) {
    let mut number = vec![None; count];
    let mut next = 0;
    let of = (of.iter())
        .map(|region| {
            *number[*region].get_or_insert_with(|| {
                next += 1;
                next - 1
            })
        })
        .collect();
    (of, next)
}

/// The regions `of` with neighbours merged, their colours their cores' (each merge read anew):
/// the nearest pair under `ONE_STEP` apart, else the pair whose shared edge is softest while
/// under `SHARP` (a shading across one surface, no edge the eye sees), else the nearest while more
/// than `MOST_AREAS` are left; none when no pair merges.
fn merged(
    of: &[usize],
    count: usize,
    lab: &[Lab],
    grid: [usize; 2],
) -> Option<(Vec<usize>, usize)> {
    let (mut of, mut count) = (of.to_vec(), count);
    let mut changed = false;
    loop {
        let colours = core_colours(&of, count, (lab, grid));
        let steps = edge_steps(&of, lab, grid);
        let apart = |(a, b): (usize, usize)| distance(&colours[a], &colours[b]);
        let sharpness = |pair: (usize, usize)| steps[&pair] / apart(pair).max(f64::EPSILON);
        let by = |key: &dyn Fn((usize, usize)) -> f64| {
            (steps.keys().copied()).min_by(|x, y| key(*x).total_cmp(&key(*y)))
        };
        let close = by(&apart).filter(|pair| apart(*pair) < ONE_STEP);
        let shading = || by(&sharpness).filter(|pair| sharpness(*pair) < SHARP);
        let crowded = || by(&apart).filter(|_| count > MOST_AREAS);
        let Some((a, b)) = close.or_else(shading).or_else(crowded) else {
            break;
        };
        let joined: Vec<usize> = of.iter().map(|r| if *r == b { a } else { *r }).collect();
        (of, count) = compact(&joined, count);
        changed = true;
    }
    changed.then_some((of, count))
}

/// How far apart each pair of neighbouring regions of `of` look across their shared edge: the
/// mean distance, along every side of a cell they share, between the squinted colours `REACH`
/// cells either side of it. A sharp edge or a soft one parts their colours almost wholly within
/// that reach; a shading across one surface parts them only a little of the way.
fn edge_steps(of: &[usize], lab: &[Lab], [w, h]: [usize; 2]) -> BTreeMap<(usize, usize), f64> {
    let mut sums: BTreeMap<(usize, usize), (f64, f64)> = BTreeMap::new();
    let reach = REACH as isize - 1;
    let at = |x: isize, y: isize| {
        let [x, y] = [x.clamp(0, w as isize - 1), y.clamp(0, h as isize - 1)];
        y as usize * w + x as usize
    };
    for cell in 0..of.len() {
        let [x, y] = [(cell % w) as isize, (cell / w) as isize];
        for [dx, dy] in [[1, 0], [0, 1]] {
            let [nx, ny] = [x + dx, y + dy];
            if nx >= w as isize || ny >= h as isize || of[at(nx, ny)] == of[cell] {
                continue;
            }
            let [here, there] = [of[cell], of[at(nx, ny)]];
            let behind = lab[at(x - dx * reach, y - dy * reach)];
            let beyond = lab[at(nx + dx * reach, ny + dy * reach)];
            let sum = sums
                .entry((here.min(there), here.max(there)))
                .or_insert((0.0, 0.0));
            *sum = (sum.0 + distance(&behind, &beyond), sum.1 + 1.0);
        }
    }
    (sums.into_iter())
        .map(|(pair, (sum, sides))| (pair, sum / sides))
        .collect()
}
