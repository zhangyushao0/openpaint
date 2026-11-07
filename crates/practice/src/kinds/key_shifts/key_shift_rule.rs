//! A key shift made by rule (S107): how a painter moves each colour of a picture toward a tendency
//! as V大 teaches 变调, in Oklab, so the How's example and the contact sheets' copies are shifted
//! as a painter would shift them. Every colour keeps its lightness; in the `a`, `b` plane:
//!
//! - a hue near the tendency's turns toward it the short way round (`TURN` of the way, fading
//!   from a quarter turn off to none at `OPPOSITE`), a little purer under a pure tendency;
//! - a hue across from it keeps its hue and turns greyer (`OPPOSITE_GREY` of its chroma at most):
//!   the opposites move only a little;
//! - a grey (chroma under `GREY`, fading to `COLOURED`) moves most: it takes the tendency's
//!   direction, `GREY_PUSH` far, gaining a little chroma;
//! - every coloured one is also pushed `PUSH` along the tendency's direction: the whole picture
//!   shifted; under a grey tendency everything is greyer (`GREYER` of its chroma);
//! - the lightest barely move: from `LIGHT` up to `LIGHTEST_SHOWN` the move fades to
//!   `1 - LIGHT_STAYS` of itself;
//! - the result is the hue and lightness asked at as much chroma as sRGB holds.
//!
//! Over a picture's areas (`shifted`), two neighbours apart in the picture that came within
//! `KEPT_APART` of each other move less, a step at a time, until they are apart again (V大:
//! after the shift the colours still separate).

use crate::colour_areas::areas::Areas;
use crate::kinds::key_shifts::key_shift::{Tendency, turn};
use crate::kinds::key_shifts::key_shift_check::APART;
use crate::maths::colour::{Rgb8, delta_e, in_gamut, oklab};

/// The share of the way a near hue turns toward the tendency's.
const TURN: f64 = 0.35;
/// Hues this far off the tendency's (degrees) turn the full share; beyond, less, none from
/// `OPPOSITE` on.
const NEAR_TURN: f64 = 90.0;
const OPPOSITE: f64 = 150.0;
/// How much greyer an opposite hue turns, at most, and how much purer a near hue under a pure
/// tendency (within `PURER_WITHIN` degrees).
const OPPOSITE_GREY: f64 = 0.3;
const PURER: f64 = 0.12;
const PURER_WITHIN: f64 = 60.0;
/// How much chroma a grey tendency leaves every colour.
const GREYER: f64 = 0.6;
/// How far every coloured area is pushed along the tendency's direction (a pure one's, a grey
/// one's), and a grey area.
const PUSH: [f64; 2] = [0.015, 0.015];
const GREY_PUSH: [f64; 2] = [0.05, 0.04];
/// A colour under `GREY` chroma is a grey, over `COLOURED` a colour, a blend between.
const GREY: f64 = 0.03;
const COLOURED: f64 = 0.08;
/// From `LIGHT` lightness up to `LIGHTEST_SHOWN` the move fades by `LIGHT_STAYS` of itself.
const LIGHT: f64 = 0.75;
const LIGHTEST_SHOWN: f64 = 0.95;
const LIGHT_STAYS: f64 = 0.8;
/// Neighbours apart in the picture are kept at least this far apart (Oklab), above Compare's
/// `MERGED`.
const KEPT_APART: f64 = 0.06;
/// Each step back toward their own colours keeps this share of two merging neighbours' moves.
const BACK: f64 = 0.7;
const STEPS_BACK: usize = 8;

impl Tendency {
    /// `colour` shifted toward the tendency by V大's rules, its lightness kept.
    #[must_use]
    pub fn shift(self, colour: Rgb8) -> Rgb8 {
        self.shift_by(colour, 1.0)
    }

    /// `colour` moved `share` of its shift's way in `a`, `b`, its lightness kept.
    fn shift_by(self, colour: Rgb8, share: f64) -> Rgb8 {
        let [l, a, b] = oklab(colour);
        let [x, y] = self.moved([a, b], l);
        let [x, y] = [a + share * (x - a), b + share * (y - b)];
        in_gamut([l, x.hypot(y), y.atan2(x).to_degrees()])
    }

    /// Where a colour of lightness `l` at `[a, b]` moves.
    fn moved(self, [a, b]: [f64; 2], l: f64) -> [f64; 2] {
        // The pushes' place: a pure tendency's first, a grey one's second.
        let lean = usize::from(!self.lean.pure());
        let [u, v] = self.direction();
        let chroma = a.hypot(b);
        let hue = b.atan2(a).to_degrees();
        let towards = turn(hue, f64::from(self.hue));
        let off = towards.abs();
        let turning = 1.0 - ((off - NEAR_TURN) / (OPPOSITE - NEAR_TURN)).clamp(0.0, 1.0);
        let mut c = chroma;
        if off > NEAR_TURN {
            c *= 1.0 - OPPOSITE_GREY * ((off - NEAR_TURN) / (OPPOSITE - NEAR_TURN)).min(1.0);
        } else if self.lean.pure() && off < PURER_WITHIN {
            c *= 1.0 + PURER * (1.0 - off / PURER_WITHIN);
        }
        if !self.lean.pure() {
            c *= GREYER;
        }
        let h = (hue + towards * TURN * turning).to_radians();
        let coloured = [c * h.cos() + PUSH[lean] * u, c * h.sin() + PUSH[lean] * v];
        let grey = [a + GREY_PUSH[lean] * u, b + GREY_PUSH[lean] * v];
        let g = 1.0 - smooth(GREY, COLOURED, chroma);
        let shifted = [0, 1].map(|k| coloured[k] + g * (grey[k] - coloured[k]));
        let light = 1.0 - LIGHT_STAYS * smooth(LIGHT, LIGHTEST_SHOWN, l);
        [a + light * (shifted[0] - a), b + light * (shifted[1] - b)]
    }

    /// Every area of `areas` shifted, two neighbours apart in the picture (`APART`) kept at least
    /// `KEPT_APART` apart by moving both less.
    #[must_use]
    pub fn shifted(self, areas: &Areas) -> Vec<Rgb8> {
        let mut shares = vec![1.0; areas.count()];
        let mut colours = self.at(areas, &shares);
        for _ in 0..STEPS_BACK {
            let merging: Vec<(usize, usize)> = (areas.neighbours.iter().copied())
                .filter(|(x, y)| delta_e(areas.colours[*x], areas.colours[*y]) >= APART)
                .filter(|(x, y)| delta_e(colours[*x], colours[*y]) < KEPT_APART)
                .collect();
            if merging.is_empty() {
                break;
            }
            for (x, y) in merging {
                shares[x] *= BACK;
                shares[y] *= BACK;
            }
            colours = self.at(areas, &shares);
        }
        colours
    }

    /// `areas`' colours each moved its `shares` of the way.
    fn at(self, areas: &Areas, shares: &[f64]) -> Vec<Rgb8> {
        (areas.colours.iter().zip(shares))
            .map(|(colour, share)| self.shift_by(*colour, *share))
            .collect()
    }
}

/// 0 at `low` and under, 1 at `high` and over, smooth between.
fn smooth(low: f64, high: f64, value: f64) -> f64 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
