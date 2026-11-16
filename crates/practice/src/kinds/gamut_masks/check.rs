//! What Compare judges on a gamut mask (S153): each area's colour read as a combination's
//! (`areas_read.rs`: its core's median, none while mostly bare paper) and held to three rules, each
//! a direct measure (D123), so the app judges it:
//!
//! 1. Inside the mask: the area's Oklab `a`, `b` within `INSIDE` of the mask's shape (Gurney: only
//!    the colours inside it).
//! 2. Lightness kept: its Oklab `L` within `LIGHTNESS_OFF` of the original's, the key shift's band
//!    (a mask limits hue and purity, never value).
//! 3. Still apart: two neighbours `APART` or more in the original whose copies came within
//!    `MERGED` of each other are one block, the key shift's rule (V大: 偏完各色仍要分得开).
//!
//! Every rule held: clean. One finding (an area off, or one pair merged): near. More: again. The
//! reason (D121) names the worst finding by its sign: an area outside the mask too pure, too grey
//! or leaning toward a hue the mask leaves out; then one too light or too dark; then merged. The
//! areas outside are kept (code 61).

use super::mask::{INSIDE, Mask, distance, nearest, span};
use crate::colour_areas::areas::Areas;
use crate::colour_areas::main_hue::turn;
use crate::kinds::colours::marks::toward;
use crate::kinds::key_shifts::key_shift_check::{APART, KeyCheck, LIGHTNESS_OFF, MERGED};
use crate::marks::card::{Arg, Say};
use crate::marks::compared::Outcome;
use crate::maths::colour::{HUELESS, Rgb8, delta_e, oklab};
use crate::maths::path::Point;
use crate::records::metric::{Metric, Metrics};

/// A copy with this many findings at most is near.
pub const NEAR_FINDINGS: usize = 1;

/// A gamut mask's copy judged: what is off, area by area.
#[derive(Debug, Clone, PartialEq)]
pub struct MaskCheck {
    /// Each area's lightness in the copy less the original's (Oklab `L`); none where the copy has
    /// not painted it.
    pub lightness: Vec<Option<f64>>,
    /// The areas whose lightness left the original's by more than `LIGHTNESS_OFF`.
    pub off_lightness: Vec<usize>,
    /// The areas painted outside the mask, and how far outside each lies (Oklab `a`, `b`).
    pub outside: Vec<(usize, f64)>,
    /// The neighbours apart in the original whose copies merged, each pair once.
    pub merged: Vec<(usize, usize)>,
}

impl MaskCheck {
    /// `copy`'s colours (an area's none where not painted) against the original `areas` and
    /// `mask`.
    #[must_use]
    pub fn of(areas: &Areas, copy: &[Option<Rgb8>], mask: Mask) -> Self {
        let corners = mask.corners();
        let pairs = || (areas.colours.iter().zip(copy)).enumerate();
        let lightness: Vec<Option<f64>> = pairs()
            .map(|(_, (original, mine))| mine.map(|m| oklab(m)[0] - oklab(*original)[0]))
            .collect();
        let off_lightness = (lightness.iter().enumerate())
            .filter(|(_, off)| off.is_some_and(|off| off.abs() > LIGHTNESS_OFF))
            .map(|(area, _)| area)
            .collect();
        let outside = pairs()
            .filter_map(|(area, (_, mine))| Some((area, distance(&corners, ab((*mine)?)))))
            .filter(|(_, off)| *off > INSIDE)
            .collect();
        let merged = (areas.neighbours.iter().copied())
            .filter(|(a, b)| delta_e(areas.colours[*a], areas.colours[*b]) >= APART)
            .filter(|(a, b)| {
                let (Some(x), Some(y)) = (copy[*a], copy[*b]) else {
                    return false;
                };
                delta_e(x, y) < MERGED
            })
            .collect();
        Self {
            lightness,
            off_lightness,
            outside,
            merged,
        }
    }

    /// Whether `area` is painted outside the mask.
    #[must_use]
    pub fn is_outside(&self, area: usize) -> bool {
        self.outside.iter().any(|(held, _)| *held == area)
    }

    /// Whether `area` is off by any rule.
    #[must_use]
    pub fn marked(&self, area: usize) -> bool {
        self.is_outside(area)
            || self.off_lightness.contains(&area)
            || (self.merged.iter()).any(|(a, b)| *a == area || *b == area)
    }

    /// How many findings: the areas off (outside or in lightness, each once) and the pairs merged.
    #[must_use]
    pub fn findings(&self) -> usize {
        let mut off: Vec<usize> = (self.outside.iter().map(|(area, _)| *area))
            .chain(self.off_lightness.iter().copied())
            .collect();
        off.sort_unstable();
        off.dedup();
        off.len() + self.merged.len()
    }

    /// Clean with nothing found, near with `NEAR_FINDINGS`, else again.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        Outcome::counting(self.findings(), NEAR_FINDINGS)
    }

    /// The reason beside a result not clean (D121), with the area it names (from 0): the area
    /// farthest outside the mask and which way it left it (`way`); else the area's lightness
    /// furthest off, lighter or darker; else the merged blocks. No degree: the result counts the
    /// findings rather than how far one is (as a value scale's cell, S139).
    #[must_use]
    pub fn reason(&self, copy: &[Option<Rgb8>], mask: Mask) -> Option<(usize, Say)> {
        let farthest = (self.outside.iter()).max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((area, _)) = farthest {
            let mine = copy.get(*area).copied().flatten()?;
            let say = Say {
                key: "outside-mask",
                args: vec![
                    Arg::Word(way(&mask.corners(), ab(mine))),
                    Arg::Count(area + 1),
                ],
            };
            return Some((*area, say));
        }
        let off = |area: &usize| self.lightness[*area].map_or(0.0, f64::abs);
        let lightest = (self.off_lightness.iter()).max_by(|x, y| off(x).total_cmp(&off(y)));
        if let Some(area) = lightest {
            let lighter = self.lightness[*area].is_some_and(|l| l > 0.0);
            let way = if lighter { "light" } else { "dark" };
            return Some((*area, Say::way("colour", way).numbered(area + 1)));
        }
        let &(first, _) = self.merged.first()?;
        let pairs = KeyCheck {
            lightness: Vec::new(),
            off_lightness: Vec::new(),
            unmoved: Vec::new(),
            merged: self.merged.clone(),
        };
        let (areas, blocks) = pairs.merged_blocks();
        Some((first, Say::counting("merged", &[areas, blocks])))
    }

    /// The number the first Compare keeps (S153), shown nowhere: the areas painted outside the
    /// mask.
    #[must_use]
    pub fn metrics(&self) -> Metrics {
        vec![(Metric::MaskOutside, self.outside.len() as f64)]
    }
}

/// Which way the colour at `p` (Oklab `a`, `b`) left the mask `corners`: where the mask holds
/// some colour of its hue, beyond it, too pure ("pure"), or short of it, too grey ("grey"); where
/// it holds none (a small mask off the middle), round the wheel toward the named hue it turned
/// to from the mask's nearest point (as a colour's reason names it, `toward`).
#[must_use]
pub fn way(corners: &[Point], p: Point) -> &'static str {
    let long = p[0].hypot(p[1]);
    if long < HUELESS {
        return "grey";
    }
    if let Some((near, _)) = span(corners, [p[0] / long, p[1] / long]) {
        return if long < near { "grey" } else { "pure" };
    }
    let q = nearest(corners, p);
    let hue = |at: Point| at[1].atan2(at[0]).to_degrees().rem_euclid(360.0);
    toward(hue(q), turn(hue(q), hue(p)) > 0.0)
}

/// `colour`'s place in Oklab's `a`, `b` plane.
fn ab(colour: Rgb8) -> Point {
    let [_, a, b] = oklab(colour);
    [a, b]
}
