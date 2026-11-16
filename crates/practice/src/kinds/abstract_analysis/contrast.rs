//! Abstract analysis's contrasts (S112a): where in a composition a contrast of one kind is strong
//! and where weak, from its pixels as the screen shows them, for Compare's five views. Each but the
//! squint is a veil of paper white over the picture where its contrast is weak, so what stays
//! clear is where that contrast lives (V大's second step: by what contrast the place that draws the
//! eye draws it):
//!
//! - cells: the picture reduced to `MASSES_SIDE` cells on its longer side, each the mean of its
//!   pixels' light (linear sRGB, as an eye's blur mixes it) in Oklab, and the mean of its pixels'
//!   own Oklab chroma (purity is the paint's in each place: a red stroke beside a green one is two
//!   pure paints, not the grey their light mixes to); a scan's rim (`RIM`) taken off as value
//!   masses take it; each channel blurred by `BLUR`, the squint value masses squint by;
//! - 明度 (`Lightness`), 纯度 (`Chroma`), 色相 (`Hue`): at each cell the largest difference of
//!   that part (Oklab's `L`; chroma; the chord between the hues, `Difference`'s parts) between the
//!   cell and the cells within `CONTRAST_REACH` of it along sixteen directions, and between each
//!   two of those across it, so a contrast shows as a band along its edge; clear from its
//!   threshold's `strong` up, veiled under its `weak`, smoothly between. Absolute, not the
//!   picture's own: a picture without a hue contrast is veiled whole under 色相, which is true;
//! - 虚实 (`Edges`): the lightness band, where its step is hard: the steepest change of the
//!   unblurred lightness over two cells along the cells between the two the step was found
//!   between, as a share of the step. A hard edge carries it all there and is clear; a soft one
//!   spreads it over many cells and is veiled, as is a place with no edge;
//! - each veil's strengths feathered (`FEATHER`);
//! - 眯眼 (`Squint`): no veil, the picture squinted (`SQUINTED`).
//!
//! Size, isolation and the subject are not measured: the How says so. The thresholds were set on
//! the contact sheets of the 61 official compositions (`cargo run -p openpaint-harness --
//! contrasts`), each with its reason beside it. A view ships unless it veils a focus made by its
//! own contrast on more than about five of them (a tonal picture's focus veiled under 纯度 or 色相
//! is the truth, not a miss): the main session's reading of the bar on the sheets, 2026-11-15,
//! all five shipped (D122).

use crate::compositions::value_masses::{BLUR, MASSES_SIDE, RIM, blurred, cells_of, rimless};
use crate::maths::colour::{Rgb8, from_oklab, linear, oklab_of_linear};
use crate::maths::path::Point;

/// How far round a cell its contrasts are looked for, a share of the picture's longer side: a
/// contrast shows as a band twice as wide along its edge, wide enough to be seen at a frame's
/// size, narrow enough that a picture is not clear all over once it has a few edges.
pub const CONTRAST_REACH: f64 = 0.03;
/// The directions the reach is looked along.
const DIRECTIONS: u32 = 16;
/// The most a veil hides: the picture faintly there under it, so the eye still finds its place.
pub const VEILED: f64 = 0.8;
/// The squint 眯眼 shows: a Gaussian's standard deviation, a share of the longer side: a figure's
/// features and a picture's brushwork gone, its big masses and their values kept (`BLUR`, value
/// masses' squint before they are grouped, still shows the details at a frame's size).
pub const SQUINTED: f64 = 0.02;
/// How soft a veil's edge is: a Gaussian's standard deviation, a share of the longer side, so a
/// band fades in over a few of a frame's points and a cell's noise in the hardness of an edge is
/// evened out among its neighbours, never a cut line or a speckle.
const FEATHER: f64 = 0.01;

/// 明度: a lightness step (Oklab `L`): weak under three just-noticeable steps, strong at about a
/// sixth of the range a painting's lightness spans; a figure against its ground clears, a
/// painting's brushwork inside a mass does not.
pub const LIGHTNESS_CONTRAST: Threshold = Threshold {
    weak: 0.06,
    strong: 0.15,
};
/// 纯度: a step of the paints' chroma, a third of lightness's: a painting's chroma spans about a
/// quarter of the range its lightness does (a red shirt or a sun among earth colours clears; at
/// lightness's thresholds no picture but a cartoon's cleared anywhere).
pub const CHROMA_CONTRAST: Threshold = Threshold {
    weak: 0.02,
    strong: 0.05,
};
/// 色相: the chord between two hues at their chromas, weak just past a just-noticeable step, so
/// greyish hues apart never count and an orange stack against blue snow does.
pub const HUE_CONTRAST: Threshold = Threshold {
    weak: 0.025,
    strong: 0.06,
};
/// 虚实: the share of a lightness step two cells carry: all of it at a hard edge (a soft one over
/// the band's twelve cells carries a sixth); under 0.55 a step spread over about four cells, as a
/// painting's soft edge at a frame's size; brushwork's own steps inside a soft edge stay under it.
pub const HARD_EDGE: Threshold = Threshold {
    weak: 0.55,
    strong: 0.85,
};

/// Where a contrast starts to count and where it is strong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub weak: f64,
    pub strong: f64,
}

impl Threshold {
    /// How strong `contrast` is: 0 at `weak` and under, 1 at `strong` and over, smoothly between.
    #[must_use]
    pub fn strength(self, contrast: f64) -> f64 {
        let t = ((contrast - self.weak) / (self.strong - self.weak)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

/// One of Compare's views of the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContrastView {
    /// 明度.
    Lightness,
    /// 纯度.
    Chroma,
    /// 色相.
    Hue,
    /// 虚实: hard edges against soft ones.
    Edges,
    /// 眯眼.
    Squint,
}

impl ContrastView {
    /// In Compare's order.
    pub const ALL: [Self; 5] = [
        Self::Lightness,
        Self::Chroma,
        Self::Hue,
        Self::Edges,
        Self::Squint,
    ];

    /// Its place among the veils; none for the squint.
    const fn veil(self) -> Option<usize> {
        match self {
            Self::Lightness => Some(0),
            Self::Chroma => Some(1),
            Self::Hue => Some(2),
            Self::Edges => Some(3),
            Self::Squint => None,
        }
    }
}

/// A picture's contrasts: a map of cells, each with how strong each contrast is there, and the
/// picture squinted.
#[derive(Debug, Clone, PartialEq)]
pub struct Contrasts {
    /// Cells across and down.
    pub size: [u32; 2],
    /// Each cell's strength (0 weak to 1 strong) of 明度, 纯度, 色相 and 虚实, row by row.
    strengths: [Vec<f64>; 4],
    /// The picture squinted, a colour a cell.
    squinted: Vec<Rgb8>,
}

impl Contrasts {
    /// The contrasts of the picture `rgba` (8-bit sRGB with straight alpha, on white paper),
    /// `size` pixels.
    #[must_use]
    pub fn of(size: [u32; 2], rgba: &[u8]) -> Self {
        let cells = cells_of(size, MASSES_SIDE);
        let longer = f64::from(cells[0].max(cells[1]));
        let rim = (RIM * longer).round() as usize;
        let channels = channels(size, rgba, cells).map(|channel| rimless(&channel, cells, rim));
        let squint = |sigma: f64| -> [Vec<f64>; 4] {
            std::array::from_fn(|k| blurred(&channels[k], cells, sigma * longer))
        };
        let reach = (CONTRAST_REACH * longer).round().max(1.0) as usize;
        let [lightness, chroma, hue, hard] = strengths(&squint(BLUR), &channels[0], cells, reach)
            .map(|cells_strength| blurred(&cells_strength, cells, FEATHER * longer));
        let edges = lightness.iter().zip(&hard).map(|(l, h)| l * h).collect();
        let strengths = [lightness, chroma, hue, edges];
        let [l, a, b, _] = squint(SQUINTED);
        let squinted = (0..l.len()).map(|at| from_oklab([l[at], a[at], b[at]]));
        Self {
            size: cells,
            strengths,
            squinted: squinted.collect(),
        }
    }

    /// Each cell's strength of `view`'s contrast, 0 weak to 1 strong, row by row; none for the
    /// squint.
    #[must_use]
    pub fn strength(&self, view: ContrastView) -> Option<&[f64]> {
        view.veil().map(|veil| self.strengths[veil].as_slice())
    }

    /// The strength of `view`'s contrast at the picture's point `p`, given in shares of its width
    /// and height (0 to 1); none for the squint.
    #[must_use]
    pub fn at(&self, view: ContrastView, p: Point) -> Option<f64> {
        let cell = |share: f64, side: u32| {
            ((share * f64::from(side)).floor().max(0.0) as usize).min(side as usize - 1)
        };
        let at = cell(p[1], self.size[1]) * self.size[0] as usize + cell(p[0], self.size[0]);
        self.strength(view).and_then(|cells| cells.get(at).copied())
    }

    /// `view` as a picture of `size`, one pixel a cell (8-bit sRGB, straight alpha): a veil of
    /// paper white, `VEILED` opaque where its contrast is weak and clear where strong; the
    /// squint's colours, opaque.
    #[must_use]
    pub fn picture(&self, view: ContrastView) -> Vec<u8> {
        match self.strength(view) {
            Some(cells) => (cells.iter())
                .flat_map(|strength| {
                    let alpha = (VEILED * (1.0 - strength) * 255.0).round() as u8;
                    [255, 255, 255, alpha]
                })
                .collect(),
            None => (self.squinted.iter())
                .flat_map(|[r, g, b]| [*r, *g, *b, 255])
                .collect(),
        }
    }
}

/// Each cell's mean light in Oklab (`L`, `a`, `b`) and the mean of its pixels' own chroma, a
/// pixel's colour over white as much as it is transparent.
fn channels([width, height]: [u32; 2], rgba: &[u8], cells: [u32; 2]) -> [Vec<f64>; 4] {
    let table: Vec<f64> = (0..=255u8).map(|c| linear(f64::from(c) / 255.0)).collect();
    let span = |cell: u32, cells: u32, length: u32| {
        let from = (u64::from(cell) * u64::from(length) / u64::from(cells)) as usize;
        let to = (u64::from(cell + 1) * u64::from(length) / u64::from(cells)) as usize;
        from..to.max(from + 1).min(length as usize)
    };
    let mut out: [Vec<f64>; 4] = Default::default();
    for cy in 0..cells[1] {
        let rows = span(cy, cells[1], height);
        for cx in 0..cells[0] {
            let columns = span(cx, cells[0], width);
            let (mut sum, mut count) = ([0.0; 4], 0.0);
            for y in rows.clone() {
                for x in columns.clone() {
                    let at = 4 * (y * width as usize + x);
                    let Some(pixel) = rgba.get(at..at + 4) else {
                        continue;
                    };
                    let alpha = f64::from(pixel[3]) / 255.0;
                    let light: [f64; 3] =
                        std::array::from_fn(|k| table[usize::from(pixel[k])] * alpha + 1.0 - alpha);
                    let [_, a, b] = oklab_of_linear(light);
                    for (total, value) in sum.iter_mut().zip([light[0], light[1], light[2]]) {
                        *total += value;
                    }
                    sum[3] += a.hypot(b);
                    count += 1.0;
                }
            }
            let [r, g, b, chroma] = sum.map(|total| total / f64::max(count, 1.0));
            let [l, a, b] = oklab_of_linear([r, g, b]);
            for (channel, value) in out.iter_mut().zip([l, a, b, chroma]) {
                channel.push(value);
            }
        }
    }
    out
}

/// Each cell's strength of 明度, 纯度 and 色相 and how hard its lightness step is, from the
/// squinted channels (`L`, `a`, `b` and the paints' chroma) and the unblurred lightness `fine`,
/// its contrasts looked for within `reach` cells.
fn strengths(
    squinted: &[Vec<f64>; 4],
    fine: &[f64],
    cells: [u32; 2],
    reach: usize,
) -> [Vec<f64>; 4] {
    let [l, a, b, paint] = squinted;
    let hue_chroma: Vec<f64> = a.iter().zip(b).map(|(a, b)| a.hypot(*b)).collect();
    let chord = |p: usize, q: usize| {
        let squared = 2.0 * (hue_chroma[p] * hue_chroma[q] - a[p] * a[q] - b[p] * b[q]);
        squared.max(0.0).sqrt()
    };
    let around = around(reach);
    let (w, h) = (cells[0] as isize, cells[1] as isize);
    let mut out: [Vec<f64>; 4] = Default::default();
    for y in 0..h {
        for x in 0..w {
            let p = (y * w + x) as usize;
            let cell = |[dx, dy]: [isize; 2]| {
                ((y + dy).clamp(0, h - 1) * w + (x + dx).clamp(0, w - 1)) as usize
            };
            let (mut most, mut step) = ([0.0_f64; 3], (p, p));
            for &[dx, dy] in &around {
                let [q, r] = [cell([dx, dy]), cell([-dx, -dy])];
                for (i, j) in [(p, q), (r, q)] {
                    let lightness = (l[i] - l[j]).abs();
                    if lightness > most[0] {
                        (most[0], step) = (lightness, (i, j));
                    }
                    most[1] = most[1].max((paint[i] - paint[j]).abs());
                    most[2] = most[2].max(chord(i, j));
                }
            }
            let share = if most[0] > 0.0 {
                (steepest(fine, w as usize, step) / most[0]).min(1.0)
            } else {
                0.0
            };
            let strengths = [
                LIGHTNESS_CONTRAST.strength(most[0]),
                CHROMA_CONTRAST.strength(most[1]),
                HUE_CONTRAST.strength(most[2]),
                HARD_EDGE.strength(share),
            ];
            for (channel, strength) in out.iter_mut().zip(strengths) {
                channel.push(strength);
            }
        }
    }
    out
}

/// The steepest change of `fine` (cells `width` across) over two cells along the cells from the
/// first of `step`'s cells to the second: a hard edge's whole step, a soft one's share of it.
fn steepest(fine: &[f64], width: usize, (from, to): (usize, usize)) -> f64 {
    let [x0, y0, x1, y1] = [from % width, from / width, to % width, to / width].map(|v| v as f64);
    let count = (x1 - x0).abs().max((y1 - y0).abs()) as usize;
    let along: Vec<f64> = (0..=count)
        .map(|k| {
            let t = k as f64 / count.max(1) as f64;
            let [x, y] = [x0 + t * (x1 - x0), y0 + t * (y1 - y0)].map(|v| v.round() as usize);
            fine[y * width + x]
        })
        .collect();
    (along.windows(3.min(along.len())))
        .map(|run| (run[run.len() - 1] - run[0]).abs())
        .fold(0.0, f64::max)
}

/// The cells within `reach` of a cell along `DIRECTIONS` directions, as offsets, each once.
pub(crate) fn around(reach: usize) -> Vec<[isize; 2]> {
    let mut offsets: Vec<[isize; 2]> = (0..DIRECTIONS)
        .flat_map(|k| {
            let turn = std::f64::consts::TAU * f64::from(k) / f64::from(DIRECTIONS);
            (1..=reach).map(move |step| {
                let length = step as f64;
                [turn.cos(), turn.sin()].map(|c| (c * length).round() as isize)
            })
        })
        .collect();
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}
