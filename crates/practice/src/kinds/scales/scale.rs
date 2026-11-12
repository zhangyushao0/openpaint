//! Value scales (after Gurney's and Ctrl+Paint's value studies, beside V大's week 2 colour work):
//! a row of cells between a given dark and light of one hue and purity, the cells between them
//! filled in even steps of lightness. Nine cells (S97: halved and halved again, the middle, the
//! halves' middles, then the rest, `halving`), five at a lesson's first level, seven at its second
//! (`FIRST_CELLS`, `scales/mod.rs`); generated the same for the same seed: the ends far apart (HSV
//! value 0.12 to 0.3 and 0.78 to 0.95 as the wheel sets them), grey or of a hue (in turn by
//! session). Compare lays the even scale as a row under the copy's (`EVEN_ROW`), cell under cell.
//!
//! Checked on perceptual lightness, Oklab's `L`, as the eye sees it (the main session, S83: the
//! wheel's value steps unevenly to the eye, the more so on a hue): each cell's lightness read
//! from the copy's pixels (the median over the cell's middle, clear of its edges), against the
//! even step there, and the order (each cell lighter than the one before). A cell a quarter of a
//! step off or more is uneven: the steps either side of it then differ by half a step, which the
//! eye sees in a row. A coloured scale's ends lie on one ray from black in Oklab (the wheel's
//! value scales sRGB's channels together, so hue and purity hold), and so does the even scale
//! between them: a hue held, its lightness stepped.

use crate::marks::compared::{Outcome, Verdict};
use crate::maths::colour::{Rgb8, from_hsv, from_oklab, oklab};
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::records::metric::{Metric, Metrics};

/// Where the row lies in the copy's frame: left, top, right, bottom (frame units).
pub const ROW: [f64; 4] = [0.06, 0.34, 0.94, 0.56];
/// Where Compare lays the even scale, cell under cell under the row: its top and bottom.
pub const EVEN_ROW: [f64; 2] = [0.58, 0.8];
/// A scale's cells, the ends among them; a beginner's first scales' (`FIRST_SCALES` of them).
pub const CELLS: usize = 9;
pub const FIRST_CELLS: usize = 5;
pub const FIRST_SCALES: usize = 2;
/// A cell's middle, which is read: the share of its width and height kept inside its edges.
const MIDDLE: f64 = 0.6;
/// A near scale has none out of order and at most this many cells uneven: of a beginner's five
/// cells one, of more cells (seven or nine) `NEAR_UNEVEN_LONG`.
pub const NEAR_UNEVEN: usize = 1;
pub const NEAR_UNEVEN_LONG: usize = 2;

/// Whether a value scale is grey or of a hue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleTone {
    Grey,
    Hue,
}

impl ScaleTone {
    pub const ALL: [Self; 2] = [Self::Grey, Self::Hue];
}

/// A value scale: its cells and the colours of its ends, one hue and saturation (HSV).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueScale {
    pub cells: usize,
    pub hue: f64,
    pub saturation: f64,
    /// The ends' values, as the wheel shows them.
    pub dark: f64,
    pub light: f64,
}

/// A value scale's copy as checked.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleCheck {
    /// Each cell's lightness (Oklab `L`, 0 to 1; the ends given); none for a cell not filled.
    pub values: Vec<Option<f64>>,
    /// Each cell's lightness on an even scale.
    pub targets: Vec<f64>,
    /// The cells (between the ends) a quarter of a step off or more.
    pub uneven: Vec<usize>,
    /// The cells no lighter than the one before them (both filled).
    pub reversed: Vec<usize>,
}

impl ValueScale {
    /// The scale of `cells` cells `tone` and `seed` make.
    #[must_use]
    pub fn generate(tone: ScaleTone, cells: usize, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let cells = cells.max(3);
        let (hue, saturation) = match tone {
            ScaleTone::Grey => (0.0, 0.0),
            ScaleTone::Hue => (random.range(0.0, 360.0), random.range(0.2, 0.55)),
        };
        Self {
            cells,
            hue,
            saturation,
            dark: random.range(0.12, 0.3),
            light: random.range(0.78, 0.95),
        }
    }

    /// The colour of lightness `lightness` (Oklab `L`) on this scale: the point of that lightness
    /// on the Oklab line through its ends (beyond them, the line drawn on), clipped to sRGB.
    #[must_use]
    pub fn colour(&self, lightness: f64) -> Rgb8 {
        let [dark, light] = self.ends().map(oklab);
        let span = light[0] - dark[0];
        let t = if span.abs() < f64::EPSILON {
            0.0
        } else {
            (lightness - dark[0]) / span
        };
        from_oklab([0, 1, 2].map(|axis| dark[axis] + (light[axis] - dark[axis]) * t))
    }

    /// The ends as the screen shows them: the dark and the light colour.
    #[must_use]
    pub fn ends(&self) -> [Rgb8; 2] {
        [self.dark, self.light].map(|value| from_hsv([self.hue, self.saturation, value]))
    }

    /// Cell `index`'s top left and bottom right corners, in frame units.
    #[must_use]
    pub fn cell(&self, index: usize) -> [Point; 2] {
        let [left, top, right, bottom] = ROW;
        let width = (right - left) / self.cells as f64;
        let x = left + width * index as f64;
        [[x, top], [x + width, bottom]]
    }

    /// The even scale's cell `index` as Compare lays it, under the copy's cell: its top left and
    /// bottom right corners.
    #[must_use]
    pub fn even_cell(&self, index: usize) -> [Point; 2] {
        let [[left, _], [right, _]] = self.cell(index);
        let [top, bottom] = EVEN_ROW;
        [[left, top], [right, bottom]]
    }

    /// The cell holding the frame point `p`, if any.
    #[must_use]
    pub fn cell_at(&self, p: Point) -> Option<usize> {
        (0..self.cells).find(|index| {
            let [[left, top], [right, bottom]] = self.cell(*index);
            (left..right).contains(&p[0]) && (top..bottom).contains(&p[1])
        })
    }

    /// Each cell's lightness on an even scale between the ends as shown.
    #[must_use]
    pub fn targets(&self) -> Vec<f64> {
        let [dark, light] = self.ends().map(|end| oklab(end)[0]);
        let steps = (self.cells - 1).max(1) as f64;
        (0..self.cells)
            .map(|index| {
                let t = index as f64 / steps;
                dark * (1.0 - t) + light * t
            })
            .collect()
    }

    /// The copy whose pixels `rgba` (8-bit RGBA, `size` pixels) cover its frame exactly, the
    /// sheet's paper `paper`: each cell between the ends its middle's median lightness, none
    /// while most of it is `paper`; the ends as given.
    #[must_use]
    pub fn read(&self, size: [u32; 2], rgba: &[u8], paper: Rgb8) -> ScaleCheck {
        let targets = self.targets();
        let values: Vec<Option<f64>> = (0..self.cells)
            .map(|index| {
                if index == 0 || index + 1 == self.cells {
                    return Some(targets[index]);
                }
                self.cell_value(index, (size, rgba), paper)
            })
            .collect();
        self.check(values)
    }

    /// `values` (one a cell) checked against the even scale.
    #[must_use]
    pub fn check(&self, values: Vec<Option<f64>>) -> ScaleCheck {
        let targets = self.targets();
        let step = (targets[self.cells - 1] - targets[0]).abs() / (self.cells - 1).max(1) as f64;
        let uneven = (1..self.cells.saturating_sub(1))
            .filter(|index| {
                values[*index].is_some_and(|value| (value - targets[*index]).abs() >= step / 4.0)
            })
            .collect();
        let reversed = (1..self.cells)
            .filter(|index| match (values[index - 1], values[*index]) {
                (Some(before), Some(value)) => value <= before,
                _ => false,
            })
            .collect();
        ScaleCheck {
            values,
            targets,
            uneven,
            reversed,
        }
    }

    /// Cell `index`'s lightness in the copy: the median over its middle.
    fn cell_value(
        &self,
        index: usize,
        ([w, h], rgba): ([u32; 2], &[u8]),
        paper: Rgb8,
    ) -> Option<f64> {
        let [[left, top], [right, bottom]] = self.cell(index);
        let inset = |low: f64, high: f64| {
            let keep = (high - low) * (1.0 - MIDDLE) / 2.0;
            (low + keep, high - keep)
        };
        let ((left, right), (top, bottom)) = (inset(left, right), inset(top, bottom));
        let (w, h) = (w.max(1) as usize, h.max(1) as usize);
        let pixel = |v: f64, side: usize| ((v * side as f64).round() as usize).min(side);
        let mut values = Vec::new();
        let mut bare = 0;
        for y in pixel(top, h)..pixel(bottom, h) {
            for x in pixel(left, w)..pixel(right, w) {
                let at = 4 * (y * w + x);
                let Some(texel) = rgba.get(at..at + 3) else {
                    continue;
                };
                let colour = [texel[0], texel[1], texel[2]];
                bare += usize::from(colour == paper);
                values.push(oklab(colour)[0]);
            }
        }
        if values.is_empty() || 2 * bare > values.len() {
            return None;
        }
        values.sort_unstable_by(f64::total_cmp);
        Some(values[values.len() / 2])
    }
}

/// The cells between the ends of a scale of `cells` in the order to paint them, a round at a time
/// (S97, V大's way of halving): the middle; each half's middle; each quarter's, and so on. Of
/// five: the 3rd; the 2nd and 4th. Of nine: the 5th; the 3rd and 7th; the 2nd, 4th, 6th and 8th
/// (indices from 0).
#[must_use]
pub fn halving(cells: usize) -> Vec<Vec<usize>> {
    let mut rounds = Vec::new();
    let mut spans = vec![(0, cells.max(1) - 1)];
    while !spans.is_empty() {
        let mut round = Vec::new();
        let mut next = Vec::new();
        for (low, high) in spans.into_iter().filter(|(low, high)| high - low >= 2) {
            let middle = (low + high) / 2;
            round.push(middle);
            next.extend([(low, middle), (middle, high)]);
        }
        if !round.is_empty() {
            rounds.push(round);
        }
        spans = next;
    }
    rounds
}

impl ScaleCheck {
    /// Incomplete while a cell is not filled; clean when no cell is uneven or reversed; near when
    /// none is reversed and at most `NEAR_UNEVEN` are uneven (of five cells; `NEAR_UNEVEN_LONG` of
    /// more); else redo.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        if self.values.iter().any(Option::is_none) {
            return Verdict::Incomplete;
        }
        let near = if self.values.len() <= FIRST_CELLS {
            NEAR_UNEVEN
        } else {
            NEAR_UNEVEN_LONG
        };
        Verdict::App(match (self.uneven.len(), self.reversed.len()) {
            (0, 0) => Outcome::Clean,
            (uneven, 0) if uneven <= near => Outcome::Near,
            _ => Outcome::Redo,
        })
    }

    /// The numbers kept at the first Compare: how far the cells filled are off on average,
    /// how many are uneven and how many out of order.
    #[must_use]
    pub fn metrics(&self) -> Metrics {
        let off: Vec<f64> = (1..self.values.len().saturating_sub(1))
            .filter_map(|index| Some((self.values[index]? - self.targets[index]).abs()))
            .collect();
        let mut metrics = Vec::with_capacity(3);
        if !off.is_empty() {
            metrics.push((Metric::ScaleOff, off.iter().sum::<f64>() / off.len() as f64));
        }
        metrics.push((Metric::ScaleUneven, self.uneven.len() as f64));
        metrics.push((Metric::ScaleReversed, self.reversed.len() as f64));
        metrics
    }
}
