//! What Compare marks on a key shift (S107). V大: a key shift has no standard answer, so Compare
//! lays none over the copy and marks only what is off, each area's colour read as a combination's
//! (`areas_read.rs`: its core's median, none while mostly bare paper) and judged in Oklab by three
//! checks, each validated on contact sheets of shifts made by rule, made wrong on purpose and
//! painted unevenly by a simulated hand (`openpaint-harness key-shifts`):
//!
//! 1. Lightness kept (V大: lightness stays close to the original): an area whose Oklab `L` left
//!    the original's by more than `LIGHTNESS_OFF`, about a step of a nine-cell value scale, which
//!    the Grey view shows plainly side by side.
//! 2. Moved toward the tendency: an area whose colour came no nearer the tendency in Oklab's `a`,
//!    `b` plane by `MOVED` (a just-noticeable step), nearer measured both ways and the larger
//!    taken: how far it moved along the tendency's direction (a grey taking its hue, an opposite
//!    greying, a near hue turning or growing purer) and how much nearer the tendency's own colour
//!    it came (everything greying under a grey tendency). A grey area is held to it as any other:
//!    it should move most. The lightest (`LIGHTEST` up) barely move and are not held to it; an
//!    area already near the tendency need come only `NEAR_SHARE` of its way there, one at it (within
//!    `AT`, a just-noticeable step) not at all.
//! 3. Still apart (V大: 偏完各色仍要分得开): two neighbours `APART` or more in the original whose
//!    copies came within `MERGED` of each other, under two just-noticeable steps: one block.

use crate::colour_areas::areas::Areas;
use crate::kinds::key_shifts::key_shift::Tendency;
use crate::maths::colour::{Rgb8, delta_e, oklab};
use crate::records::metric::{Metric, Metrics};

/// How far an area's lightness may leave the original's (Oklab `L`).
pub const LIGHTNESS_OFF: f64 = 0.08;
/// How much nearer the tendency an area must come, at least (Oklab).
pub const MOVED: f64 = 0.02;
/// From this lightness up an area barely moves and is not held to the move.
pub const LIGHTEST: f64 = 0.85;
/// An area nearer the tendency (Oklab `a`, `b`) than `MOVED / NEAR_SHARE` need come only this
/// share of its way.
pub const NEAR_SHARE: f64 = 0.25;
/// An area this near the tendency (Oklab `a`, `b`) is at it already.
pub const AT: f64 = 0.02;
/// Neighbours this far apart in the original are clearly apart; their copies within `MERGED`
/// are one block.
pub const APART: f64 = 0.08;
pub const MERGED: f64 = 0.035;
/// A near key shift has at most this many areas marked by any of the three checks.
pub const NEAR_AREAS: usize = 1;

/// A key shift's copy judged: what is off, area by area.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyCheck {
    /// Each area's lightness in the copy less the original's (Oklab `L`); none where the copy has
    /// not painted it.
    pub lightness: Vec<Option<f64>>,
    /// The areas whose lightness left the original's by more than `LIGHTNESS_OFF`.
    pub off_lightness: Vec<usize>,
    /// The areas that did not move toward the tendency.
    pub unmoved: Vec<usize>,
    /// The neighbours apart in the original whose copies merged, each pair once.
    pub merged: Vec<(usize, usize)>,
}

impl KeyCheck {
    /// `copy`'s colours (an area's none where not painted) against the original `areas` shifted
    /// toward `tendency`.
    #[must_use]
    pub fn of(areas: &Areas, copy: &[Option<Rgb8>], tendency: Tendency) -> Self {
        let pairs = || (areas.colours.iter().zip(copy)).enumerate();
        let lightness: Vec<Option<f64>> = pairs()
            .map(|(_, (original, mine))| mine.map(|m| oklab(m)[0] - oklab(*original)[0]))
            .collect();
        let off_lightness = (lightness.iter().enumerate())
            .filter(|(_, off)| off.is_some_and(|off| off.abs() > LIGHTNESS_OFF))
            .map(|(area, _)| area)
            .collect();
        let unmoved = pairs()
            .filter(|(_, (original, mine))| {
                mine.is_some_and(|mine| !moved_enough(**original, mine, tendency))
            })
            .map(|(area, _)| area)
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
            unmoved,
            merged,
        }
    }

    /// The areas lighter than the original's past `LIGHTNESS_OFF`, and those darker.
    #[must_use]
    pub fn lighter_darker(&self) -> (usize, usize) {
        let off = |up: bool| {
            (self.off_lightness.iter())
                .filter(|area| self.lightness[**area].is_some_and(|l| (l > 0.0) == up))
                .count()
        };
        (off(true), off(false))
    }

    /// The areas the merged pairs hold, and the blocks they became (pairs joined through a shared
    /// area are one block).
    #[must_use]
    pub fn merged_blocks(&self) -> (usize, usize) {
        let mut blocks: Vec<Vec<usize>> = Vec::new();
        for &(a, b) in &self.merged {
            let found: Vec<usize> = (0..blocks.len())
                .filter(|k| blocks[*k].contains(&a) || blocks[*k].contains(&b))
                .collect();
            let mut joined = vec![a, b];
            for k in found.iter().rev() {
                joined.extend(blocks.remove(*k));
            }
            joined.sort_unstable();
            joined.dedup();
            blocks.push(joined);
        }
        (blocks.iter().map(Vec::len).sum(), blocks.len())
    }

    /// Whether `area` is off by any check.
    #[must_use]
    pub fn marked(&self, area: usize) -> bool {
        self.off_lightness.contains(&area)
            || self.unmoved.contains(&area)
            || (self.merged.iter()).any(|(a, b)| *a == area || *b == area)
    }

    /// The numbers the first Compare keeps (S107), shown nowhere: how many areas each check
    /// marked (pairs, for the merged).
    #[must_use]
    pub fn metrics(&self) -> Metrics {
        vec![
            (Metric::KeyLightness, self.off_lightness.len() as f64),
            (Metric::KeyUnmoved, self.unmoved.len() as f64),
            (Metric::KeyMerged, self.merged.len() as f64),
        ]
    }
}

/// How much nearer the tendency `copy` came than `original`, Oklab: the larger of its move along
/// the tendency's direction and how much nearer the tendency's own colour it lies.
#[must_use]
pub fn toward(original: Rgb8, copy: Rgb8, tendency: Tendency) -> f64 {
    let ([_, a0, b0], [_, a1, b1]) = (oklab(original), oklab(copy));
    let [u, v] = tendency.direction();
    let along = (a1 - a0) * u + (b1 - b0) * v;
    let [ta, tb] = tendency.ab();
    let nearer = (a0 - ta).hypot(b0 - tb) - (a1 - ta).hypot(b1 - tb);
    along.max(nearer)
}

/// Whether `copy` moved toward `tendency` enough from `original`: `MOVED`, or `NEAR_SHARE` of
/// its way where it lies nearer; the lightest and one at the tendency need not.
fn moved_enough(original: Rgb8, copy: Rgb8, tendency: Tendency) -> bool {
    let [l, a, b] = oklab(original);
    let [ta, tb] = tendency.ab();
    let least = MOVED.min(NEAR_SHARE * (a - ta).hypot(b - tb));
    let at = (a - ta).hypot(b - tb) <= AT;
    l >= LIGHTEST || at || toward(original, copy, tendency) >= least
}
