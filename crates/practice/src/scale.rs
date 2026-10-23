//! Value scales (after Gurney's and Ctrl+Paint's value studies, beside V大's week 2 colour work):
//! a row of cells between a given dark and light of one hue and purity, the cells between them
//! filled in even steps of value. Generated the same for the same seed: five to seven cells, the
//! ends' values far apart, grey or of a hue.
//!
//! Checked on the colour wheel's value only, where the hand sets it: each cell's value read from
//! the copy's pixels (the median over the cell's middle, clear of its edges), against the even
//! step there, and the order (each cell lighter than the one before). A cell a quarter of a step
//! off or more is uneven: the steps either side of it then differ by half a step, which the eye
//! sees in a row.

use crate::colour::{Rgb8, from_hsv, hsv};
use crate::metric::{Metric, Metrics};
use crate::path::Point;
use crate::random::Random;

/// Where the row lies in the copy's frame and in the original's: left, top, right, bottom (frame
/// units).
pub const ROW: [f64; 4] = [0.06, 0.34, 0.94, 0.56];
/// Where the bars that show each cell's value lie under the row: top and bottom.
pub const BARS: [f64; 2] = [0.62, 0.9];
/// A cell's middle, which is read: the share of its width and height kept inside its edges.
const MIDDLE: f64 = 0.6;

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
    /// Each cell's value (the ends given); none for a cell not filled.
    pub values: Vec<Option<f64>>,
    /// Each cell's value on an even scale.
    pub targets: Vec<f64>,
    /// The cells (between the ends) a quarter of a step off or more.
    pub uneven: Vec<usize>,
    /// The cells no lighter than the one before them (both filled).
    pub reversed: Vec<usize>,
}

impl ValueScale {
    /// The scale `tone` and `seed` make.
    #[must_use]
    pub fn generate(tone: ScaleTone, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let cells = 5 + ((random.unit() * 3.0) as usize).min(2);
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

    /// The colour of `value` on this scale.
    #[must_use]
    pub fn colour(&self, value: f64) -> Rgb8 {
        from_hsv([self.hue, self.saturation, value])
    }

    /// The ends as the screen shows them: the dark and the light colour.
    #[must_use]
    pub fn ends(&self) -> [Rgb8; 2] {
        [self.colour(self.dark), self.colour(self.light)]
    }

    /// Cell `index`'s top left and bottom right corners, in frame units.
    #[must_use]
    pub fn cell(&self, index: usize) -> [Point; 2] {
        let [left, top, right, bottom] = ROW;
        let width = (right - left) / self.cells as f64;
        let x = left + width * index as f64;
        [[x, top], [x + width, bottom]]
    }

    /// The cell holding the frame point `p`, if any.
    #[must_use]
    pub fn cell_at(&self, p: Point) -> Option<usize> {
        (0..self.cells).find(|index| {
            let [[left, top], [right, bottom]] = self.cell(*index);
            (left..right).contains(&p[0]) && (top..bottom).contains(&p[1])
        })
    }

    /// Each cell's value on an even scale between the ends as shown.
    #[must_use]
    pub fn targets(&self) -> Vec<f64> {
        let [dark, light] = self.ends().map(|end| hsv(end)[2]);
        let steps = (self.cells - 1).max(1) as f64;
        (0..self.cells)
            .map(|index| {
                let t = index as f64 / steps;
                dark * (1.0 - t) + light * t
            })
            .collect()
    }

    /// The copy whose pixels `rgba` (8-bit RGBA, `size` pixels) cover its frame exactly, the
    /// sheet's paper `paper`: each cell between the ends its middle's median value, none while
    /// most of it is `paper`; the ends as given.
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

    /// Cell `index`'s value in the copy: the median over its middle.
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
                values.push(colour.into_iter().max().unwrap_or(0));
            }
        }
        if values.is_empty() || 2 * bare > values.len() {
            return None;
        }
        values.sort_unstable();
        Some(f64::from(values[values.len() / 2]) / 255.0)
    }
}

impl ScaleCheck {
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
