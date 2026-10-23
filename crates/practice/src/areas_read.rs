//! A colour combination's copy read from its pixels, and checked as the eye compares it with its
//! original (`areas.rs`).
//!
//! - Each area's colour in the copy: the median, channel by channel, of the copy's pixels over
//!   the area's core. The core keeps clear of soft edges and of a copy's edges a little off, and
//!   the median of stray strokes and texture over less than half of it: the colour the eye
//!   gives the area. An area whose core still mostly shows the sheet's paper is not painted.
//! - Each area's difference: as a single colour's (`Difference`), on the colour wheel.
//! - Weak contrasts (V大's most common mistake, about four learners in five): for each pair of
//!   neighbours, the contrast is the Oklab difference between their colours, Oklab being made so
//!   equal differences look equally different (about 0.02 is just noticeable side by side). A
//!   pair is weak in the original at `WEAK` or less; the copy made it strong when its contrast
//!   is at least twice the original's and at least `MADE_STRONG` (two and a half just-noticeable
//!   steps) more: a step the eye sees plainly where there was barely one. A copy lighter or
//!   darker all over keeps its contrasts; a strong contrast made stronger is no weak one made
//!   strong.

use crate::areas::{Areas, GRID, WEAK};
use crate::colour::{Difference, Rgb8, delta_e};
use crate::metric::{Metric, Metrics};
use crate::path::Point;

/// How much more than the original's a weak contrast's copy must be, at least, to have been made
/// strong (Oklab: two and a half just-noticeable steps).
pub const MADE_STRONG: f64 = 0.05;

/// The contrast between two neighbouring areas, in the original and in the copy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contrast {
    pub areas: (usize, usize),
    /// The Oklab difference between their colours in the original.
    pub original: f64,
    /// And in the copy; none while either is not painted.
    pub copy: Option<f64>,
}

impl Contrast {
    /// Whether it is weak in the original.
    #[must_use]
    pub fn weak(&self) -> bool {
        self.original <= WEAK
    }

    /// Whether the copy made a weak contrast strong.
    #[must_use]
    pub fn made_strong(&self) -> bool {
        let least = (2.0 * self.original).max(self.original + MADE_STRONG);
        self.weak() && self.copy.is_some_and(|copy| copy >= least)
    }
}

/// A copy's colours in each area, and how it keeps the original's contrasts.
#[derive(Debug, Clone, PartialEq)]
pub struct AreasCopy {
    /// Each area's colour in the copy; none where it is not painted.
    pub colours: Vec<Option<Rgb8>>,
    /// Every pair of neighbours.
    pub contrasts: Vec<Contrast>,
}

impl AreasCopy {
    /// Each painted area's difference from its original colour.
    #[must_use]
    pub fn differences(&self, original: &Areas) -> Vec<Option<Difference>> {
        (original.colours.iter().zip(&self.colours))
            .map(|(original, copy)| copy.map(|copy| Difference::of(*original, copy)))
            .collect()
    }

    /// The numbers kept at the first Compare: the mean difference over the areas painted, the
    /// weak contrasts and how many the copy made strong.
    #[must_use]
    pub fn metrics(&self, original: &Areas) -> Metrics {
        let deltas: Vec<f64> = (self.differences(original).iter())
            .flatten()
            .map(|d| d.delta)
            .collect();
        let mut metrics = Vec::with_capacity(3);
        if !deltas.is_empty() {
            let mean = deltas.iter().sum::<f64>() / deltas.len() as f64;
            metrics.push((Metric::AreasDelta, mean));
        }
        let weak = self.contrasts.iter().filter(|c| c.weak()).count();
        let strong = self.contrasts.iter().filter(|c| c.made_strong()).count();
        metrics.push((Metric::AreasWeak, weak as f64));
        metrics.push((Metric::AreasMadeStrong, strong as f64));
        metrics
    }
}

impl Areas {
    /// The copy whose pixels `rgba` (8-bit RGBA, `size` pixels) cover its frame exactly, the
    /// sheet's paper `paper`.
    #[must_use]
    pub fn read(&self, size: [u32; 2], rgba: &[u8], paper: Rgb8) -> AreasCopy {
        let colours = self.copy_colours(size, rgba, paper);
        let contrasts = (self.neighbours.iter())
            .map(|&(a, b)| {
                let copy = colours[a].zip(colours[b]).map(|(x, y)| delta_e(x, y));
                Contrast {
                    areas: (a, b),
                    original: delta_e(self.colours[a], self.colours[b]),
                    copy,
                }
            })
            .collect();
        AreasCopy { colours, contrasts }
    }

    /// Each area's colour in the copy: its core's median, or none while most of the core is
    /// `paper`.
    fn copy_colours(
        &self,
        [width, height]: [u32; 2],
        rgba: &[u8],
        paper: Rgb8,
    ) -> Vec<Option<Rgb8>> {
        let mut core = vec![u8::MAX; GRID * GRID];
        for area in 0..self.count() {
            for cell in self.core(area) {
                core[cell] = u8::try_from(area).unwrap_or(u8::MAX);
            }
        }
        let mut samples: Vec<Vec<Rgb8>> = vec![Vec::new(); self.count()];
        let (w, h) = (width.max(1) as usize, height.max(1) as usize);
        for (index, pixel) in rgba.as_chunks::<4>().0.iter().take(w * h).enumerate() {
            let (x, y) = (index % w, index / w);
            let cell = (y * GRID / h) * GRID + x * GRID / w;
            if let Some(held) = samples.get_mut(usize::from(core[cell])) {
                held.push([pixel[0], pixel[1], pixel[2]]);
            }
        }
        samples
            .into_iter()
            .map(|mut held| {
                let bare = held.iter().filter(|c| **c == paper).count();
                if held.is_empty() || 2 * bare > held.len() {
                    return None;
                }
                Some([0, 1, 2].map(|k| {
                    held.sort_unstable_by_key(|c| c[k]);
                    held[held.len() / 2][k]
                }))
            })
            .collect()
    }

    /// The area at the frame point `p`, as the map holds it.
    #[must_use]
    pub fn area_at(&self, p: Point) -> usize {
        let cell = |v: f64| ((v * GRID as f64).floor().max(0.0) as usize).min(GRID - 1);
        self.cell([cell(p[0]), cell(p[1])])
    }
}
