//! A cold-warm copy's two tendencies (S154, V大's 整图变调 toward two colours at once; with W5-6's
//! rule that a picture shifted as a whole keeps its warm against its cool): the lights lean toward
//! a warm hue, the shadows toward a cool one about opposite it. A pair is kept in its kind's
//! variant, so a copy is shown the tendencies it was painted to; the pairs turn from copy to copy
//! (`nth`), as key shifts' leans do, so no colour habit forms.

use crate::colour_areas::main_hue::HUE_STEP;
use crate::maths::colour::{Rgb8, in_gamut, oklab};
use crate::maths::path::Point;

/// Warm hues from magenta-red round to yellow, cool ones from green round to violet-blue (Oklab
/// hue, degrees, the first to the last in steps of `HUE_STEP`): key shifts' own.
const WARM: (i32, i32) = (-10, 110);
const COOL: (i32, i32) = (150, 290);
/// The lights' tendency's lightness and chroma as its swatch shows it, and the shadows'
/// (Oklab): a light warm and a deep cool, each plainly coloured.
const LIGHTS: [f64; 2] = [0.78, 0.12];
const SHADOWS: [f64; 2] = [0.5, 0.08];
/// The step the warm hue turns by from one copy to the next, in its hues: prime to their count,
/// so every one comes before any comes again.
const TURN: usize = 5;
/// The first copy's warm hue's place: an orange (60 degrees), the warm a painter knows first.
const FIRST: usize = 7;

/// The hues of `(first, last)` in steps of `HUE_STEP`, degrees `0..360`.
fn hues((first, last): (i32, i32)) -> Vec<u16> {
    (first..=last)
        .step_by(usize::from(HUE_STEP))
        .map(|hue| u16::try_from(hue.rem_euclid(360)).unwrap_or(0))
        .collect()
}

/// A cold-warm copy's tendencies: the lights' warm hue and the shadows' cool one (Oklab, degrees,
/// multiples of `HUE_STEP`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Leans {
    pub warm: u16,
    pub cool: u16,
}

impl Leans {
    /// The pair of the `n`th cold-warm copy (from 0): its warm hue `TURN` steps on from the last
    /// one's, its cool one the cool hue nearest the opposite, a step either way in turn.
    #[must_use]
    pub fn nth(n: usize) -> Self {
        let (warm, cool) = (hues(WARM), hues(COOL));
        let hue = warm[(FIRST + n * TURN) % warm.len()];
        let opposite = (i32::from(hue) + 180).rem_euclid(360);
        let nearest = (0..cool.len())
            .min_by_key(|at| (i32::from(cool[*at]) - opposite).abs())
            .unwrap_or(0);
        let step = (n % 3) as isize - 1;
        let at = (nearest as isize + step).clamp(0, cool.len() as isize - 1) as usize;
        Self {
            warm: hue,
            cool: cool[at],
        }
    }

    /// As its kind's variant stores it: the warm hue's place times the cool hues, then the cool
    /// hue's place.
    #[must_use]
    pub fn code(self) -> u8 {
        let (warm, cool) = (hues(WARM), hues(COOL));
        let place = |all: &[u16], hue: u16| all.iter().position(|held| *held == hue).unwrap_or(0);
        u8::try_from(place(&warm, self.warm) * cool.len() + place(&cool, self.cool)).unwrap_or(0)
    }

    /// The pair stored as `code`; none past the pairs there are.
    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        let (warm, cool) = (hues(WARM), hues(COOL));
        let at = usize::from(code);
        Some(Self {
            warm: *warm.get(at / cool.len())?,
            cool: cool[at % cool.len()],
        })
    }

    /// The lights' tendency as its swatch shows it: the warm hue light.
    #[must_use]
    pub fn lights(self) -> Rgb8 {
        in_gamut([LIGHTS[0], LIGHTS[1], f64::from(self.warm)])
    }

    /// The shadows' tendency as its swatch shows it: the cool hue deep.
    #[must_use]
    pub fn shadows(self) -> Rgb8 {
        in_gamut([SHADOWS[0], SHADOWS[1], f64::from(self.cool)])
    }

    /// Where each tendency lies on the hue and chroma disc (Oklab `a`, `b`) with its colour:
    /// the lights', then the shadows'.
    #[must_use]
    pub fn on_disc(self) -> [(Point, Rgb8); 2] {
        [self.lights(), self.shadows()].map(|colour| {
            let [_, a, b] = oklab(colour);
            ([a, b], colour)
        })
    }
}
