//! Key shifts (V大's 变调, week 5; S107): a crop of a real picture (`crop.rs`, its areas from its
//! pack) painted again as if its whole key had shifted toward a tendency colour the exercise gives
//! (倾向色), warm or cool, pure or grey. The tendencies come in turn (`Lean`), each a hue of its
//! warmth never near the crop's own main hue (`AWAY`; far from it at the third level,
//! `FAR_AWAY`): a shift toward one's own colour teaches nothing. A tendency is kept in its kind's
//! variant, so a record's copy is judged against the tendency it was painted to whatever the pack
//! becomes. How a painter shifts is `key_shift_rule.rs`; what Compare marks,
//! `key_shift_check.rs`: a key shift has no answer.

use crate::colour_areas::main_hue::{HUE_STEP, turn};
use crate::maths::colour::{Rgb8, in_gamut, most_chroma};
use crate::maths::random::Random;

/// A pure tendency's chroma and a grey one's (Oklab): a pure one about an orange's or a sky's, a
/// grey one a warm or cool grey, plainly not neutral (two and a half just-noticeable steps).
pub const PURE_CHROMA: f64 = 0.13;
pub const GREY_CHROMA: f64 = 0.05;
/// Gurney's mask's apex past the tendency's chroma, its base past grey and its base's half width,
/// as shares of the apex's chroma.
const APEX_PAST: f64 = 0.05;
const BASE_PAST: f64 = 0.4;
const BASE_HALF: f64 = 0.6;
/// The lightnesses its disc may be shown at: its hue's most colourful between them.
const SHOWN: (f64, f64) = (0.55, 0.88);
/// The least turn between a tendency's hue and the crop's main hue, degrees: `AWAY` at the first
/// two levels, `FAR_AWAY` at the third (S140: toward the crop's opposite, where its colours move
/// only a little and grey, V大's harder shift), the lean's farthest hues where none lies so far.
pub const AWAY: f64 = 60.0;
pub const FAR_AWAY: f64 = 120.0;
/// Warm hues from magenta-red round to yellow, cool ones from green round to violet-blue (Oklab
/// hue, degrees, the first to the last in steps of `HUE_STEP`).
const WARM: (i32, i32) = (-10, 110);
const COOL: (i32, i32) = (150, 290);

/// Which way a tendency leans: warm or cool, pure or grey; in a sheet's turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lean {
    WarmPure,
    CoolGrey,
    WarmGrey,
    CoolPure,
}

impl Lean {
    /// In turn: the warmth changes every exercise, the purity every second, so any three in a
    /// row hold both of each.
    pub const ALL: [Self; 4] = [
        Self::WarmPure,
        Self::CoolGrey,
        Self::WarmGrey,
        Self::CoolPure,
    ];

    #[must_use]
    pub const fn warm(self) -> bool {
        matches!(self, Self::WarmPure | Self::WarmGrey)
    }

    #[must_use]
    pub const fn pure(self) -> bool {
        matches!(self, Self::WarmPure | Self::CoolPure)
    }

    /// Its hues, degrees `0..360`.
    fn hues(self) -> Vec<u16> {
        let (first, last) = if self.warm() { WARM } else { COOL };
        (first..=last)
            .step_by(usize::from(HUE_STEP))
            .map(|hue| u16::try_from(hue.rem_euclid(360)).unwrap_or(0))
            .collect()
    }

    fn index(self) -> u8 {
        let at = Self::ALL.iter().position(|lean| *lean == self).unwrap_or(0);
        u8::try_from(at).unwrap_or(0)
    }
}

/// A key shift's tendency: its lean and its hue (Oklab, degrees, a multiple of `HUE_STEP`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tendency {
    pub lean: Lean,
    pub hue: u16,
}

/// Hues on the circle per turn: `360 / HUE_STEP`.
const STEPS: u8 = 36;

impl Tendency {
    /// A tendency of `lean` picked by `random` among its hues at least `least` degrees from the
    /// crop's `main` hue (any of them without one), among its farthest where none lies so far
    /// (`AWAY` always finds some: a lean's hues span 120 degrees and more).
    #[must_use]
    pub fn chosen(lean: Lean, main: Option<f64>, least: f64, random: &mut Random) -> Self {
        let all = lean.hues();
        let off = |hue: u16| main.map_or(180.0, |main| turn(f64::from(hue), main).abs());
        let farthest = all.iter().map(|hue| off(*hue)).fold(0.0, f64::max);
        let least = least.min(farthest);
        let from: Vec<u16> = (all.iter().copied())
            .filter(|hue| off(*hue) >= least)
            .collect();
        let at = ((random.unit() * from.len() as f64) as usize).min(from.len() - 1);
        Self {
            lean,
            hue: from[at],
        }
    }

    /// As its kind's variant stores it: its lean's place times the hues a turn, then its hue's
    /// step.
    #[must_use]
    pub fn code(self) -> u8 {
        let step = u8::try_from(self.hue / HUE_STEP).unwrap_or(0) % STEPS;
        self.lean.index() * STEPS + step
    }

    /// The tendency stored as `code`: none unless its hue is one of its lean's.
    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        let lean = *Lean::ALL.get(usize::from(code / STEPS))?;
        let hue = u16::from(code % STEPS) * HUE_STEP;
        lean.hues().contains(&hue).then_some(Self { lean, hue })
    }

    /// Its chroma: a pure tendency's or a grey one's.
    #[must_use]
    pub const fn chroma(self) -> f64 {
        if self.lean.pure() {
            PURE_CHROMA
        } else {
            GREY_CHROMA
        }
    }

    /// Its direction in Oklab's `a`, `b` plane, a unit long.
    #[must_use]
    pub fn direction(self) -> [f64; 2] {
        let (sin, cos) = f64::from(self.hue).to_radians().sin_cos();
        [cos, sin]
    }

    /// Its place in Oklab's `a`, `b` plane: its direction at its chroma.
    #[must_use]
    pub fn ab(self) -> [f64; 2] {
        self.direction().map(|v| v * self.chroma())
    }

    /// The colour its disc shows: its hue at its chroma, or as much as sRGB holds there, at the
    /// lightness within `SHOWN` where sRGB holds most of its hue (a yellow light, a blue deep).
    #[must_use]
    pub fn colour(self) -> Rgb8 {
        let hue = f64::from(self.hue);
        let lightness = (0..=33)
            .map(|step| SHOWN.0 + (SHOWN.1 - SHOWN.0) * f64::from(step) / 33.0)
            .max_by(|x, y| most_chroma(*x, hue, 1.0).total_cmp(&most_chroma(*y, hue, 1.0)))
            .unwrap_or(SHOWN.0);
        in_gamut([lightness, self.chroma(), hue])
    }

    /// Gurney's gamut mask for it (Oklab `a`, `b`): a triangle whose apex lies along its
    /// direction past its chroma and whose base lies across the other side past grey, so its
    /// opposites are allowed only greyed.
    #[must_use]
    pub fn mask(self) -> [[f64; 2]; 3] {
        let [u, v] = self.direction();
        let apex = self.chroma() + APEX_PAST;
        let (base, half) = (-BASE_PAST * apex, BASE_HALF * apex);
        [
            [u * apex, v * apex],
            [u * base - v * half, v * base + u * half],
            [u * base + v * half, v * base - u * half],
        ]
    }
}
