//! A real picture's colour areas, found as a squinting eye groups them (S101, the pack builder's;
//! V大's 色彩组合: squint, set the overall tone, lay the big blocks, keep weak contrasts weak):
//!
//! - the picture reduced to `GRID` cells on its longer side, each the mean of its pixels' light
//!   (linear sRGB, as light mixes), blurred by a Gaussian of `SQUINT` cells and taken into Oklab;
//! - the cells' colours clustered (k-means from the colours the most cells crowd round, `CLUSTERS`
//!   at most, more than any combination holds, so two colours a step apart never share one), each
//!   cell then given the cluster most cells round it hold (a thin line goes, an edge is smoothed);
//! - the cells of one cluster touching side by side made one region, then grouped
//!   (`areas_regions.rs`): bands and specks dissolved into the regions beside them, then regions
//!   too small; neighbours a step apart merged, the nearest while more than `MOST_AREAS` are left;
//! - each area's colour its core's median over the picture's own pixels, as a copy is read.
//!
//! `busy` says how far the squinted cells lie from their area's colour on average: how much of the
//! picture the areas leave unsaid.

use std::collections::BTreeMap;

use crate::areas::{Areas, GRID};
use crate::areas_read::median;
use crate::areas_regions::{Lab, distance, grouped};
use crate::colour::{Rgb8, from_oklab, oklab, oklab_of_linear};
use crate::value_masses::{blurred, cells_of, reduced};
use crate::value_masses_islands::labelled;

/// The squint: the blur's standard deviation, in cells.
pub const SQUINT: f64 = 1.5;
/// Colours the cells are first clustered into at most.
const CLUSTERS: usize = 16;
/// The bins colours are counted in (Oklab), and how far apart the colours clustering starts from
/// lie at least.
const BIN: f64 = 0.01;
const SEPARATE: f64 = 0.015;
/// Rounds of k-means at most.
const ROUNDS: usize = 24;
/// How far round a cell its cluster is taken from the most there, in cells.
const ROUND: isize = 2;

/// A picture's areas as found, and how far its squinted cells lie from their area's colour on
/// average (Oklab).
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub areas: Areas,
    pub busy: f64,
}

impl Areas {
    /// The areas of the picture `rgba` (8-bit sRGB with straight alpha, on white), `size` pixels.
    #[must_use]
    pub fn found(size: [u32; 2], rgba: &[u8]) -> Found {
        let cells = cells_of(size, GRID as u32);
        let grid = cells.map(|side| side as usize);
        let light = reduced(size, rgba, cells);
        let channels: [Vec<f64>; 3] = std::array::from_fn(|k| {
            let channel: Vec<f64> = light.iter().map(|c| c[k]).collect();
            blurred(&channel, cells, SQUINT)
        });
        let lab: Vec<Lab> = (0..light.len())
            .map(|i| oklab_of_linear([channels[0][i], channels[1][i], channels[2][i]]))
            .collect();
        let labels = most_round(&clustered(&lab), grid);
        let regions = labelled(&labels, grid[0], grid[1]);
        let (of, count) = grouped(regions, &lab, grid);
        coloured((size, rgba), &of, count, (&lab, grid))
    }
}

/// The answer of the regions `of` (`count` of them, over `grid` cells whose squinted colours are
/// `lab`), numbered in reading order, each coloured by its core's median over `rgba`.
fn coloured(
    (size, rgba): ([u32; 2], &[u8]),
    of: &[usize],
    count: usize,
    (lab, grid): (&[Lab], [usize; 2]),
) -> Found {
    let mut order: Vec<Option<u8>> = vec![None; count];
    let mut next = 0u8;
    let map: Vec<u8> = (of.iter())
        .map(|region| {
            *order[*region].get_or_insert_with(|| {
                next += 1;
                next - 1
            })
        })
        .collect();
    let areas = usize::from(next);
    let means = means(
        &map.iter().map(|a| usize::from(*a)).collect::<Vec<_>>(),
        areas,
        lab,
    );
    let blank = Areas::laid(grid, map.clone(), vec![[0; 3]; areas], vec![0.0; areas]);
    let colours: Vec<Rgb8> = (blank.core_samples(size, rgba).into_iter().zip(&means))
        .map(|(held, mean)| median(held).unwrap_or_else(|| from_oklab(*mean)))
        .collect();
    let shown: Vec<Lab> = colours.iter().map(|c| oklab(*c)).collect();
    let off: f64 = (map.iter().zip(lab))
        .map(|(area, cell)| distance(&shown[usize::from(*area)], cell))
        .sum();
    Found {
        busy: off / lab.len().max(1) as f64,
        areas: Areas::laid(grid, map, colours, vec![0.0; areas]),
    }
}

/// The colours the cells crowd round: the cells binned by `BIN` of Oklab, the fullest bins first,
/// each bin's mean taken while it lies `SEPARATE` from every one taken before, `CLUSTERS` at
/// most. An area of a colour fills a bin of its own; the blends along its edges spread thin.
fn crowded(lab: &[Lab]) -> Vec<Lab> {
    let mut bins: BTreeMap<[i64; 3], (Lab, usize)> = BTreeMap::new();
    for cell in lab {
        let key = cell.map(|v| (v / BIN).round() as i64);
        let (sum, held) = bins.entry(key).or_insert(([0.0; 3], 0));
        (0..3).for_each(|k| sum[k] += cell[k]);
        *held += 1;
    }
    let mut fullest: Vec<(Lab, usize)> = (bins.into_values())
        .map(|(sum, held)| (sum.map(|v| v / held as f64), held))
        .collect();
    fullest.sort_by_key(|a| std::cmp::Reverse(a.1));
    let mut centres: Vec<Lab> = Vec::with_capacity(CLUSTERS);
    for (colour, _) in fullest {
        if centres.len() == CLUSTERS {
            break;
        }
        if centres.iter().all(|c| distance(c, &colour) >= SEPARATE) {
            centres.push(colour);
        }
    }
    centres
}

/// Each cell's cluster: k-means over `lab`, started from the colours the cells crowd round.
fn clustered(lab: &[Lab]) -> Vec<u8> {
    let mut centres = crowded(lab);
    let mut labels = vec![u8::MAX; lab.len()];
    for _ in 0..ROUNDS {
        let mut changed = false;
        for (label, cell) in labels.iter_mut().zip(lab) {
            let best = (0..centres.len())
                .min_by(|a, b| {
                    distance(cell, &centres[*a]).total_cmp(&distance(cell, &centres[*b]))
                })
                .unwrap_or(0) as u8;
            changed |= *label != best;
            *label = best;
        }
        if !changed {
            break;
        }
        let of: Vec<usize> = labels.iter().map(|l| usize::from(*l)).collect();
        for (centre, mean) in centres.iter_mut().zip(means(&of, CLUSTERS, lab)) {
            if mean.iter().all(|v| v.is_finite()) {
                *centre = mean;
            }
        }
    }
    labels
}

/// The mean colour of each of `count` regions (`of` each cell's); not a number for one empty.
fn means(of: &[usize], count: usize, lab: &[Lab]) -> Vec<Lab> {
    let mut sums = vec![([0.0; 3], 0usize); count];
    for (region, cell) in of.iter().zip(lab) {
        if let Some((sum, held)) = sums.get_mut(*region) {
            (0..3).for_each(|k| sum[k] += cell[k]);
            *held += 1;
        }
    }
    (sums.into_iter())
        .map(|(sum, held)| sum.map(|v| v / held as f64))
        .collect()
}

/// Each cell given the label most cells within `ROUND` of it hold (its own on a tie).
fn most_round(labels: &[u8], [w, h]: [usize; 2]) -> Vec<u8> {
    let disc: Vec<[isize; 2]> = (-ROUND..=ROUND)
        .flat_map(|dy| (-ROUND..=ROUND).map(move |dx| [dx, dy]))
        .filter(|[dx, dy]| dx * dx + dy * dy <= ROUND * ROUND)
        .collect();
    let (wi, hi) = (w as isize, h as isize);
    let mut out = Vec::with_capacity(labels.len());
    for y in 0..hi {
        for x in 0..wi {
            let own = labels[(y * wi + x) as usize];
            let mut counts = [0usize; CLUSTERS];
            for [dx, dy] in &disc {
                let [nx, ny] = [(x + dx).clamp(0, wi - 1), (y + dy).clamp(0, hi - 1)];
                let label = usize::from(labels[(ny * wi + nx) as usize]);
                counts[label.min(CLUSTERS - 1)] += 1;
            }
            let most = (0..CLUSTERS as u8).max_by_key(|l| (counts[usize::from(*l)], *l == own));
            out.push(most.unwrap_or(own));
        }
    }
    out
}
