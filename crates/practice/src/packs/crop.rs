//! Crops for colour combinations (S101, the pack builder's; V大's 色彩组合: a corner of an abstract
//! oil painting or of a rusted old thing, never a concrete object). A picture gives candidate
//! rectangles, square or a third longer one way, three sizes, at its corners (a composition's
//! paintings) or anywhere on a grid of nine places (a texture's photograph); each candidate's areas
//! are found (`areas_found.rs`) and held to the rules a combination keeps: `FEWEST_AREAS` to
//! `MOST_AREAS` areas, each thick enough to read, at least one weak contrast between neighbours
//! and one pair plainly apart (`DISTINCT`), at least one area low in saturation, and areas that
//! say the picture (`BUSIEST`). The best are kept, none overlapping another by more than a
//! quarter: the most varied colours and the flattest areas first, four or five before others.
//! Whether a crop shows a recognisable object no rule can tell: every crop is judged by eye on the
//! builder's contact sheets, and one that does is listed out with its reason.

use std::fmt;

use crate::areas::{Areas, LEAST_CORE, STRONG};
use crate::areas_found::Found;
use crate::colour::{delta_e, hsv};
use crate::pack::Tier;

/// The fewest areas a crop holds.
pub const FEWEST_AREAS: usize = 3;
/// A crop whose squinted cells lie farther than this from their area's colour on average (Oklab)
/// is texture more than areas: its answer would leave too much unsaid.
pub const BUSIEST: f64 = 0.065;
/// An area at most this saturated (HSV) is low in saturation.
pub const GREYISH: f64 = 0.3;
/// Two neighbours at least this far apart (Oklab, five just-noticeable steps) are plainly
/// distinct colours; a crop holds such a pair.
pub const DISTINCT: f64 = 0.1;
/// The least longer side of a crop, in pixels of the picture it is cut from (S101b): the pack
/// holds a crop at most 1,024 pixels long and the tablet shows it about 770 device pixels tall,
/// so a crop cut smaller would be enlarged there and read blurred. A size a picture cannot cut so
/// large is not offered on it.
pub const LEAST_LONGER: u32 = 768;
/// A crop's longer side as a share of the picture's shorter side, and its shapes (width to
/// height).
const SIDES: [f64; 3] = [0.25, 0.4, 0.55];
const SHAPES: [f64; 3] = [1.0, 4.0 / 3.0, 3.0 / 4.0];
/// How much of the smaller of two crops kept may lie in the other.
const OVERLAP: f64 = 0.25;

/// Where a picture's crops are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Places {
    /// Its four corners: a painting's, whose middle holds its subject.
    Corners,
    /// Nine places, its corners, the middles of its sides and its middle: a texture's.
    Anywhere,
}

/// A crop: where it lies in its picture (pixels) and its areas.
#[derive(Debug, Clone, PartialEq)]
pub struct Crop {
    pub at: [u32; 2],
    pub size: [u32; 2],
    pub found: Found,
}

/// The rule a crop breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropRefusal {
    /// Under `FEWEST_AREAS` areas.
    FewAreas,
    /// No neighbours whose contrast is weak.
    NoWeakContrast,
    /// No area low in saturation.
    NothingGrey,
    /// More texture than areas (`BUSIEST`).
    Busy,
    /// An area too thin for a copy of it to be read (no core of `LEAST_CORE` cells).
    ThinArea,
    /// No neighbours `DISTINCT` apart: one surface's shades, not colours side by side.
    AllClose,
}

impl fmt::Display for CropRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::FewAreas => "fewer than three areas",
            Self::NoWeakContrast => "no weak contrast",
            Self::NothingGrey => "no area low in saturation",
            Self::Busy => "more texture than areas",
            Self::ThinArea => "an area too thin to read",
            Self::AllClose => "no colours plainly apart",
        })
    }
}

/// The rule `found`'s areas break as a crop's, the first; none when it keeps them all.
#[must_use]
pub fn refusal(found: &Found) -> Option<CropRefusal> {
    let areas = &found.areas;
    if areas.count() < FEWEST_AREAS {
        return Some(CropRefusal::FewAreas);
    }
    if (0..areas.count()).any(|area| areas.core(area).len() < LEAST_CORE) {
        return Some(CropRefusal::ThinArea);
    }
    if areas.weak_pairs().is_empty() {
        return Some(CropRefusal::NoWeakContrast);
    }
    if !areas.colours.iter().any(|c| hsv(*c)[1] <= GREYISH) {
        return Some(CropRefusal::NothingGrey);
    }
    let distinct = (areas.neighbours.iter())
        .any(|(a, b)| delta_e(areas.colours[*a], areas.colours[*b]) >= DISTINCT);
    if !distinct {
        return Some(CropRefusal::AllClose);
    }
    (found.busy > BUSIEST).then_some(CropRefusal::Busy)
}

/// A combination's tier by its areas: three or four simple, five medium, six or seven hard.
#[must_use]
pub fn tier_of(areas: &Areas) -> Tier {
    match areas.count() {
        0..=4 => Tier::Simple,
        5 => Tier::Medium,
        _ => Tier::Hard,
    }
}

impl Crop {
    /// How good a crop it is: the more its neighbours differ (the most, up to `STRONG` twice, and
    /// on average) and the flatter its areas the better, four or five of them before others.
    #[must_use]
    pub fn score(&self) -> f64 {
        let areas = &self.found.areas;
        let contrasts: Vec<f64> = (areas.neighbours.iter())
            .map(|(a, b)| delta_e(areas.colours[*a], areas.colours[*b]))
            .collect();
        let most = contrasts
            .iter()
            .copied()
            .fold(0.0, f64::max)
            .min(2.0 * STRONG);
        let mean = contrasts.iter().sum::<f64>() / contrasts.len().max(1) as f64;
        let off = if (4..=5).contains(&areas.count()) {
            0.0
        } else {
            0.03
        };
        most + 0.5 * mean - 2.0 * self.found.busy - off
    }

    /// The share of the smaller of it and `other` lying in both.
    fn overlap(&self, other: &Self) -> f64 {
        let along = |axis: usize| {
            let from = self.at[axis].max(other.at[axis]);
            let to = (self.at[axis] + self.size[axis]).min(other.at[axis] + other.size[axis]);
            f64::from(to.saturating_sub(from))
        };
        let area = |crop: &Self| f64::from(crop.size[0]) * f64::from(crop.size[1]);
        along(0) * along(1) / area(self).min(area(other)).max(1.0)
    }
}

/// The pixels of the rectangle at `at` of `size` cut from `rgba` (8-bit RGBA, `picture` pixels).
#[must_use]
pub fn cut(picture: [u32; 2], rgba: &[u8], at: [u32; 2], size: [u32; 2]) -> Vec<u8> {
    let width = picture[0] as usize;
    let mut pixels = Vec::with_capacity(size[0] as usize * size[1] as usize * 4);
    for row in at[1]..at[1] + size[1] {
        let from = 4 * (row as usize * width + at[0] as usize);
        let to = from + 4 * size[0] as usize;
        pixels.extend_from_slice(rgba.get(from..to).unwrap_or_default());
    }
    pixels
}

/// Every candidate crop of the picture `rgba` (`size` pixels) at `places`, with its areas found.
#[must_use]
pub fn candidates(size: [u32; 2], rgba: &[u8], places: Places) -> Vec<Crop> {
    (rectangles(size, places).into_iter())
        .map(|(at, crop)| candidate(size, rgba, (at, crop)))
        .collect()
}

/// Where the candidate crops of a picture of `size` lie at `places`, in `candidates`' order: each
/// one's top left corner and size (so a builder may find their areas side by side).
#[must_use]
pub fn rectangles(size: [u32; 2], places: Places) -> Vec<([u32; 2], [u32; 2])> {
    let shorter = f64::from(size[0].min(size[1]));
    let mut found = Vec::new();
    for side in SIDES {
        for shape in SHAPES {
            let longer = (side * shorter).round();
            let [w, h] = if shape >= 1.0 {
                [longer, (longer / shape).round()]
            } else {
                [(longer * shape).round(), longer]
            }
            .map(|v| v as u32);
            if w.max(h) < LEAST_LONGER || w > size[0] || h > size[1] {
                continue;
            }
            found.extend(
                spots(size, [w, h], places)
                    .into_iter()
                    .map(|at| (at, [w, h])),
            );
        }
    }
    found
}

/// The candidate crop of the picture `rgba` (`size` pixels) at `at` of `crop` pixels, its areas
/// found.
#[must_use]
pub fn candidate(size: [u32; 2], rgba: &[u8], (at, crop): ([u32; 2], [u32; 2])) -> Crop {
    let pixels = cut(size, rgba, at, crop);
    Crop {
        at,
        size: crop,
        found: Areas::found(crop, &pixels),
    }
}

/// Where a crop of `crop` pixels lies in a picture of `size` at `places`: its top left corners.
fn spots(size: [u32; 2], crop: [u32; 2], places: Places) -> Vec<[u32; 2]> {
    let room = [size[0] - crop[0], size[1] - crop[1]];
    let steps: &[u32] = match places {
        Places::Corners => &[0, 2],
        Places::Anywhere => &[0, 1, 2],
    };
    (steps.iter())
        .flat_map(|y| {
            steps
                .iter()
                .map(move |x| [room[0] * x / 2, room[1] * y / 2])
        })
        .collect()
}

/// The best `count` of `candidates` keeping every rule, best first, none overlapping a better
/// one by more than `OVERLAP`.
#[must_use]
pub fn best(mut candidates: Vec<Crop>, count: usize) -> Vec<Crop> {
    candidates.retain(|crop| refusal(&crop.found).is_none());
    candidates.sort_by(|a, b| b.score().total_cmp(&a.score()));
    let mut kept: Vec<Crop> = Vec::new();
    for crop in candidates {
        if kept.len() < count && kept.iter().all(|k| k.overlap(&crop) <= OVERLAP) {
            kept.push(crop);
        }
    }
    kept
}
