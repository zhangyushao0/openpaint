//! A thumbnail's lightness steps (S154): abstract analysis's 明度 measure (`contrast.rs`) read on
//! the thumbnail's own pixels, as a combination's copy is read: the picture reduced to
//! `MASSES_SIDE` cells on its longer side, each the Oklab lightness of its pixels' mean light, its
//! rim held as value masses hold it, squinted by `BLUR`; at each cell the largest lightness step
//! between it and the cells within `CONTRAST_REACH` of it, and between each two of those across
//! it. What a thumbnail's three greys make of that measure: a value step is 0.25, dark against
//! light 0.5, a small accent squinted a little less.
//!
//! Its veil is the measure's own, made relative: the analysis's fixed thresholds clear every edge
//! between two of the three greys, so here a cell is clear within `CLEAR_WITHIN` of the copy's own
//! strongest step and veiled from `VEIL_BELOW` under it (about one value step): what stays clear is
//! where the copy is strongest, and anything as strong beside it.

use crate::compositions::value_masses::{
    BLUR, MASSES_SIDE, RIM, Value, blurred, cells_of, reduced, rimless,
};
use crate::kinds::abstract_analysis::contrast::{
    CONTRAST_REACH, LIGHTNESS_CONTRAST, Threshold, VEILED, around,
};
use crate::kinds::value_masses::value_masses_read::{VALUE_PEN, wider};
use crate::maths::colour::{lightness_of_linear, oklab};
use crate::maths::path::Point;

/// A step this close to the copy's strongest shows as clear as it, and one this far under it is
/// veiled (Oklab lightness): about one value step of the three greys.
pub const CLEAR_WITHIN: f64 = 0.06;
pub const VEIL_BELOW: f64 = 0.2;
/// How soft the veil's edge is, a share of the longer side (as the analysis's).
const FEATHER: f64 = 0.01;

/// A picture's lightness steps: a map of cells, each its largest step, and whether it was left
/// bare (nearer the paper's white than the light grey).
#[derive(Debug, Clone, PartialEq)]
pub struct Steps {
    /// Cells across and down.
    pub size: [u32; 2],
    /// Each cell's largest lightness step, row by row.
    pub steps: Vec<f64>,
    /// Each cell left bare, row by row.
    pub bare: Vec<bool>,
}

impl Steps {
    /// The steps of the picture `rgba` (8-bit sRGB with straight alpha, on white paper), `size`
    /// pixels.
    #[must_use]
    pub fn of(size: [u32; 2], rgba: &[u8]) -> Self {
        let cells = cells_of(size, MASSES_SIDE);
        let longer = f64::from(cells[0].max(cells[1]));
        let lightness: Vec<f64> = (reduced(size, rgba, cells).into_iter())
            .map(lightness_of_linear)
            .collect();
        let paper = f64::midpoint(oklab(Value::Light.grey())[0], 1.0);
        let bare = lightness.iter().map(|l| *l >= paper).collect();
        let held = rimless(&lightness, cells, (RIM * longer).round() as usize);
        let squinted = blurred(&held, cells, BLUR * longer);
        let reach = (CONTRAST_REACH * longer).round().max(1.0) as usize;
        Self {
            size: cells,
            steps: largest_steps(&squinted, cells, reach),
            bare,
        }
    }

    /// The share of the picture left bare over more than the grey pens' width.
    #[must_use]
    pub fn bare_share(&self) -> f64 {
        let [w, h] = self.size.map(|side| side as usize);
        let radius = VALUE_PEN / 2.0 * w.max(h) as f64;
        let wide = wider(&self.bare, [w, h], radius);
        wide.iter().filter(|bare| **bare).count() as f64 / (w * h).max(1) as f64
    }

    /// The strongest step among the cells whose middles `within` takes (shares of the picture's
    /// width and height), with that middle; none where it takes none.
    #[must_use]
    pub fn strongest(&self, within: impl Fn(Point) -> bool) -> Option<(f64, Point)> {
        let [w, h] = self.size.map(|side| side as usize);
        (0..w * h)
            .map(|at| {
                let middle = [
                    ((at % w) as f64 + 0.5) / w as f64,
                    ((at / w) as f64 + 0.5) / h as f64,
                ];
                (self.steps[at], middle)
            })
            .filter(|(_, middle)| within(*middle))
            .max_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// The veil, a pixel a cell (8-bit sRGB, straight alpha): paper white, `VEILED` opaque where a
    /// cell's step lies `VEIL_BELOW` under the strongest, clear within `CLEAR_WITHIN` of it; a
    /// picture whose strongest step is no contrast at all veiled whole.
    #[must_use]
    pub fn veil(&self) -> Vec<u8> {
        let most = self.steps.iter().copied().fold(0.0, f64::max);
        let threshold = Threshold {
            weak: (most - VEIL_BELOW).max(LIGHTNESS_CONTRAST.weak),
            strong: (most - CLEAR_WITHIN).max(LIGHTNESS_CONTRAST.strong),
        };
        let strengths: Vec<f64> = (self.steps.iter())
            .map(|step| threshold.strength(*step))
            .collect();
        let longer = f64::from(self.size[0].max(self.size[1]));
        (blurred(&strengths, self.size, FEATHER * longer).into_iter())
            .flat_map(|strength| {
                let alpha = (VEILED * (1.0 - strength.clamp(0.0, 1.0)) * 255.0).round() as u8;
                [255, 255, 255, alpha]
            })
            .collect()
    }
}

/// Each cell's largest step of `lightness` (cells `[w, h]`) within `reach` cells, between the cell
/// and another and between two across it, as the analysis's lightness band is found.
fn largest_steps(lightness: &[f64], [width, height]: [u32; 2], reach: usize) -> Vec<f64> {
    let around = around(reach);
    let (w, h) = (width as isize, height as isize);
    let mut out = Vec::with_capacity(lightness.len());
    for y in 0..h {
        for x in 0..w {
            let p = (y * w + x) as usize;
            let cell = |[dx, dy]: [isize; 2]| {
                ((y + dy).clamp(0, h - 1) * w + (x + dx).clamp(0, w - 1)) as usize
            };
            let most = (around.iter())
                .flat_map(|&[dx, dy]| {
                    let [q, r] = [cell([dx, dy]), cell([-dx, -dy])];
                    [(p, q), (r, q)]
                })
                .map(|(i, j)| (lightness[i] - lightness[j]).abs())
                .fold(0.0, f64::max);
            out.push(most);
        }
    }
    out
}
