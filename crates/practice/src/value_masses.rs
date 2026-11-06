//! Value masses (S96; V大's week 2 调性, value judged high, middle or low and the picture grouped
//! by it into big blocks; weeks 3 and 4's colour blocks grouped by their attributes, a shadow and
//! a dark ground one block; Notan): a picture squinted at until only its few big masses are left,
//! each dark, middle or light, by value and not by object. The answer is made once when a pack is
//! built and judged by eye on its contact sheet (a picture whose masses do not read as a squinting
//! eye sees them is listed `no_masses` and never offered), and for a How's example as it plays:
//!
//! - the picture reduced to `MASSES_SIDE` cells on its longer side, each the mean of its pixels'
//!   light (linear sRGB, as light mixes), taken as Oklab lightness (as the eye orders it), its
//!   outermost `RIM` the lightness just inside it (a scan's frame edge or margin is no mass), then
//!   blurred by a Gaussian of `BLUR` of that side: what a squinting eye sees, a mass's edge kept
//!   where the eye puts it between a dark and a light;
//! - the cells' lightness split in three by the two thresholds that best separate them (Otsu's
//!   method for three classes: the largest variance between the classes, over `BINS` steps);
//! - each cell given the value most cells round it hold (`ROUND`): a thin line goes and an edge is
//!   smoothed, as squinting loses them;
//! - every island under `ISLAND` of the picture merged into the mass it borders most
//!   (`value_masses_islands.rs`): a detail is no mass.
//!
//! Its edges are `value_masses_edges.rs`; a copy painted in the three greys is read against it in
//! `value_masses_read.rs`.

use crate::colour::{Rgb8, lightness_of_linear, linear, oklab};
use crate::path::Point;
use crate::value_masses_islands::merge_islands;

/// The cells of an answer on the picture's longer side.
pub const MASSES_SIDE: u32 = 200;
/// The squint: the blur's standard deviation, a share of the picture's longer side.
pub const BLUR: f64 = 0.008;
/// The least a mass holds, a share of the picture: a smaller island is a detail.
pub const ISLAND: f64 = 0.01;
/// A scan's rim, a share of the picture's longer side: no mass.
pub const RIM: f64 = 0.015;
/// Steps of lightness the thresholds are found between.
const BINS: usize = 256;
/// How far round a cell its value is taken from the most there, in cells.
const ROUND: usize = 2;

/// A mass's value, and the grey a copy paints it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    Dark,
    Middle,
    Light,
}

impl Value {
    pub const ALL: [Self; 3] = [Self::Dark, Self::Middle, Self::Light];

    /// Its grey, evenly apart in Oklab lightness: 0.35, 0.6 and 0.85.
    #[must_use]
    pub const fn grey(self) -> Rgb8 {
        match self {
            Self::Dark => [58; 3],
            Self::Middle => [128; 3],
            Self::Light => [206; 3],
        }
    }

    /// Its place in `ALL`, as an answer's cell holds it.
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Dark => 0,
            Self::Middle => 1,
            Self::Light => 2,
        }
    }

    #[must_use]
    pub fn of_index(index: u8) -> Option<Self> {
        Self::ALL.get(usize::from(index)).copied()
    }

    /// The value a copy's pixel `rgb` was painted in: the grey nearest its lightness; none where
    /// it lies nearer the paper's white (left bare).
    #[must_use]
    pub fn painted(rgb: Rgb8) -> Option<Self> {
        let lightness = oklab(rgb)[0];
        let grey = |value: Self| oklab(value.grey())[0];
        let paper = f64::midpoint(grey(Self::Light), 1.0);
        if lightness >= paper {
            return None;
        }
        Self::ALL.into_iter().min_by(|a, b| {
            let off = |value: &Self| (grey(*value) - lightness).abs();
            off(a).total_cmp(&off(b))
        })
    }
}

/// A picture's value masses: a map of cells, each a mass's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueMasses {
    /// Cells across and down.
    pub size: [u32; 2],
    /// Each cell's value's index, row by row.
    pub values: Vec<u8>,
}

impl ValueMasses {
    /// The masses of the picture `rgba` (8-bit sRGB with straight alpha, on white paper), `size`
    /// pixels.
    #[must_use]
    pub fn of(size: [u32; 2], rgba: &[u8]) -> Self {
        let cells = cells_of(size, MASSES_SIDE);
        let light = reduced(size, rgba, cells);
        let lightness: Vec<f64> = light.iter().map(|c| lightness_of_linear(*c)).collect();
        let longer = f64::from(cells[0].max(cells[1]));
        let lightness = rimless(&lightness, cells, (RIM * longer).round() as usize);
        let blurred = blurred(&lightness, cells, BLUR * longer);
        let bins: Vec<usize> = blurred.iter().map(|l| bin(*l)).collect();
        let [low, high] = thresholds(&bins);
        let values: Vec<u8> = (bins.iter())
            .map(|b| u8::from(*b >= low) + u8::from(*b >= high))
            .collect();
        let mut values = most_round(&values, cells);
        let least = (ISLAND * f64::from(cells[0]) * f64::from(cells[1])).ceil() as usize;
        merge_islands(&mut values, cells, least);
        Self {
            size: cells,
            values,
        }
    }

    /// The masses `values` holds, if it is a map of `size` (each side 1 to `MASSES_SIDE`) whose
    /// every cell is a value: what a pack's head is read into.
    #[must_use]
    pub fn checked(size: [u32; 2], values: Vec<u8>) -> Option<Self> {
        let masses = Self { size, values };
        masses.valid().then_some(masses)
    }

    /// Whether it is a map of its size (each side 1 to `MASSES_SIDE`) whose every cell is a value.
    #[must_use]
    pub fn valid(&self) -> bool {
        let sides = (self.size.iter()).all(|side| (1..=MASSES_SIDE).contains(side));
        let cells = self.size[0] as usize * self.size[1] as usize;
        let valued = (self.values.iter()).all(|v| usize::from(*v) < Value::ALL.len());
        sides && self.values.len() == cells && valued
    }

    /// The value of the cell `[x, y]`.
    #[must_use]
    pub fn value(&self, [x, y]: [usize; 2]) -> Value {
        let at = y * self.size[0] as usize + x;
        (self.values.get(at).copied())
            .and_then(Value::of_index)
            .unwrap_or(Value::Middle)
    }

    /// The value at the picture's point `p`, given in shares of its width and height (0 to 1).
    #[must_use]
    pub fn at(&self, p: Point) -> Value {
        let cell = |share: f64, side: u32| {
            ((share * f64::from(side)).floor().max(0.0) as usize).min(side as usize - 1)
        };
        self.value([cell(p[0], self.size[0]), cell(p[1], self.size[1])])
    }

    /// The masses as a picture of their greys, one pixel a cell (8-bit sRGB, opaque).
    #[must_use]
    pub fn picture(&self) -> Vec<u8> {
        (self.values.iter())
            .flat_map(|v| {
                let [r, g, b] = Value::of_index(*v).unwrap_or(Value::Middle).grey();
                [r, g, b, 255]
            })
            .collect()
    }

    /// How much of the picture each value holds, dark, middle and light.
    #[must_use]
    pub fn shares(&self) -> [f64; 3] {
        let all = self.values.len().max(1) as f64;
        [0, 1, 2].map(|k| self.values.iter().filter(|v| **v == k).count() as f64 / all)
    }
}

/// An answer's cells for a picture of `size` pixels: `most` on the longer side (`MASSES_SIDE`,
/// or a combination's `GRID`; the picture's own where it is smaller), the other in proportion.
pub(crate) fn cells_of([width, height]: [u32; 2], most: u32) -> [u32; 2] {
    let (width, height) = (width.max(1), height.max(1));
    let longer = width.max(height);
    let side = most.min(longer);
    let scaled = |length: u32| {
        let cells = (f64::from(length) * f64::from(side) / f64::from(longer)).round() as u32;
        cells.clamp(1, side)
    };
    [scaled(width), scaled(height)]
}

/// Each cell's mean light (linear sRGB) over the pixels it covers, a pixel's colour over white
/// as much as it is transparent.
pub(crate) fn reduced([width, height]: [u32; 2], rgba: &[u8], cells: [u32; 2]) -> Vec<[f64; 3]> {
    let table: Vec<f64> = (0..=255u8).map(|c| linear(f64::from(c) / 255.0)).collect();
    let span = |cell: u32, cells: u32, length: u32| {
        let from = (u64::from(cell) * u64::from(length) / u64::from(cells)) as usize;
        let to = (u64::from(cell + 1) * u64::from(length) / u64::from(cells)) as usize;
        from..to.max(from + 1).min(length as usize)
    };
    let mut light = Vec::with_capacity(cells[0] as usize * cells[1] as usize);
    for cy in 0..cells[1] {
        let rows = span(cy, cells[1], height);
        for cx in 0..cells[0] {
            let columns = span(cx, cells[0], width);
            let mut sum = [0.0; 3];
            let mut count = 0.0;
            for y in rows.clone() {
                for x in columns.clone() {
                    let at = 4 * (y * width as usize + x);
                    let Some(pixel) = rgba.get(at..at + 4) else {
                        continue;
                    };
                    let alpha = f64::from(pixel[3]) / 255.0;
                    for (total, channel) in sum.iter_mut().zip(&pixel[..3]) {
                        *total += table[usize::from(*channel)] * alpha + (1.0 - alpha);
                    }
                    count += 1.0;
                }
            }
            light.push(sum.map(|total| total / f64::max(count, 1.0)));
        }
    }
    light
}

/// `cells` with each of the outermost `rim` cells round the picture taking the value of the cell
/// just inside them: a scan's rim (a frame's edge, an unpainted margin) is no mass.
fn rimless(cells: &[f64], [width, height]: [u32; 2], rim: usize) -> Vec<f64> {
    let (w, h) = (width as usize, height as usize);
    let inside = |at: usize, length: usize| {
        if length > 2 * rim {
            at.clamp(rim, length - 1 - rim)
        } else {
            at
        }
    };
    (0..w * h)
        .map(|at| cells[inside(at / w, h) * w + inside(at % w, w)])
        .collect()
}

/// `cells` blurred by a Gaussian of standard deviation `sigma` cells, across then down, the
/// edges held.
pub(crate) fn blurred(cells: &[f64], [width, height]: [u32; 2], sigma: f64) -> Vec<f64> {
    let reach = (3.0 * sigma).ceil().max(1.0) as isize;
    let weights: Vec<f64> = (-reach..=reach)
        .map(|k| (-(k as f64).powi(2) / (2.0 * sigma.max(1e-6).powi(2))).exp())
        .collect();
    let total: f64 = weights.iter().sum();
    let (w, h) = (width as isize, height as isize);
    let pass = |source: &[f64], across: bool| -> Vec<f64> {
        let mut out = vec![0.0; source.len()];
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0;
                for (k, weight) in (-reach..=reach).zip(&weights) {
                    let [sx, sy] = if across {
                        [(x + k).clamp(0, w - 1), y]
                    } else {
                        [x, (y + k).clamp(0, h - 1)]
                    };
                    sum += weight * source[(sy * w + sx) as usize];
                }
                out[(y * w + x) as usize] = sum / total;
            }
        }
        out
    };
    pass(&pass(cells, true), false)
}

/// Each cell given the value most cells hold within `ROUND` cells of it (its own on a tie): a
/// line thinner than that (a frame's rim, a mast, a stray stroke of light) goes, as squinting
/// loses it, and a mass's edge is smoothed.
fn most_round(values: &[u8], [width, height]: [u32; 2]) -> Vec<u8> {
    let reach = ROUND as isize;
    let disc: Vec<[isize; 2]> = (-reach..=reach)
        .flat_map(|dy| (-reach..=reach).map(move |dx| [dx, dy]))
        .filter(|[dx, dy]| dx * dx + dy * dy <= reach * reach)
        .collect();
    let (w, h) = (width as isize, height as isize);
    let mut out = Vec::with_capacity(values.len());
    for y in 0..h {
        for x in 0..w {
            let own = values[(y * w + x) as usize];
            let mut counts = [0_usize; 3];
            for [dx, dy] in &disc {
                let [nx, ny] = [(x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1)];
                counts[usize::from(values[(ny * w + nx) as usize]).min(2)] += 1;
            }
            let most = (0..3_u8).max_by_key(|v| (counts[usize::from(*v)], *v == own));
            out.push(most.unwrap_or(own));
        }
    }
    out
}

/// The step of lightness `l` falls in.
fn bin(l: f64) -> usize {
    ((l.clamp(0.0, 1.0) * BINS as f64) as usize).min(BINS - 1)
}

/// The two steps that split `bins` (each cell's step of lightness) into three classes with the
/// largest variance between them: a cell under the first is dark, under the second middle.
fn thresholds(bins: &[usize]) -> [usize; 2] {
    let mut counts = [0.0_f64; BINS];
    for b in bins {
        counts[*b] += 1.0;
    }
    // Cumulative weight and first moment before each step.
    let mut weight = [0.0; BINS + 1];
    let mut moment = [0.0; BINS + 1];
    for k in 0..BINS {
        weight[k + 1] = weight[k] + counts[k];
        moment[k + 1] = moment[k] + counts[k] * k as f64;
    }
    let part = |from: usize, to: usize| {
        let w = weight[to] - weight[from];
        if w > 0.0 {
            (moment[to] - moment[from]).powi(2) / w
        } else {
            0.0
        }
    };
    let between = |low: usize, high: usize| part(0, low) + part(low, high) + part(high, BINS);
    let pairs = || (1..BINS - 1).flat_map(|low| (low + 1..BINS).map(move |high| (low, high)));
    let best = pairs()
        .map(|(low, high)| between(low, high))
        .fold(f64::MIN, f64::max);
    // Every pair as good splits the classes alike (steps no cell holds lie between them): the
    // middle of them, so a threshold lies midway between the values it parts.
    let (mut sum, mut count) = ([0_usize; 2], 0_usize);
    for (low, high) in pairs().filter(|(low, high)| between(*low, *high) >= best - 1e-9 * best) {
        sum = [sum[0] + low, sum[1] + high];
        count += 1;
    }
    sum.map(|total| (total + count / 2) / count.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_clusters_of_lightness_split_between_them() {
        // Cells round 0.3, 0.6 and 0.9 lightness, a little spread.
        let bins: Vec<usize> = [0.3, 0.6, 0.9]
            .iter()
            .flat_map(|l| (0..40).map(move |k| bin(l + (f64::from(k) - 20.0) * 0.002)))
            .collect();
        let [low, high] = thresholds(&bins);
        assert!((bin(0.35)..bin(0.55)).contains(&low), "{low}");
        assert!((bin(0.65)..bin(0.85)).contains(&high), "{high}");
    }

    #[test]
    fn a_painted_grey_reads_as_its_value_and_white_as_bare() {
        for value in Value::ALL {
            assert_eq!(Value::painted(value.grey()), Some(value));
        }
        assert_eq!(Value::painted([255; 3]), None);
        assert_eq!(Value::painted([60, 50, 50]), Some(Value::Dark));
        assert_eq!(Value::painted([226; 3]), Some(Value::Light));
    }

    #[test]
    fn a_picture_of_three_bands_is_three_masses() {
        let (width, height) = (300_u32, 200_u32);
        let rgba: Vec<u8> = (0..width * height)
            .flat_map(|at| {
                let y = at / width;
                let grey = [40, 128, 230][(3 * y / height) as usize];
                [grey, grey, grey, 255]
            })
            .collect();
        let masses = ValueMasses::of([width, height], &rgba);
        assert_eq!(masses.size, [200, 133]);
        assert_eq!(masses.at([0.5, 0.1]), Value::Dark);
        assert_eq!(masses.at([0.5, 0.5]), Value::Middle);
        assert_eq!(masses.at([0.5, 0.9]), Value::Light);
        let shares = masses.shares();
        assert!(shares.iter().all(|share| (share - 1.0 / 3.0).abs() < 0.02));
        assert_eq!(
            ValueMasses::checked(masses.size, masses.values.clone()),
            Some(masses)
        );
        assert_eq!(ValueMasses::checked([2, 2], vec![0, 1, 2, 3]), None);
    }
}
