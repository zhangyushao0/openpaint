//! Single colours to match (V大's W1 single colours): a swatch of a generated colour, matched by
//! filling the empty frame beside it, picked once by eye and corrected two or three times. Most
//! swatches are low in saturation, which V大 stresses; every hue comes up.
//!
//! A match is judged by its first pick and its final colour, each against the original as an
//! Oklab difference (one number for how different they look) and its lightness, chroma and hue
//! parts (which way to correct: lighter or darker, purer or greyer, towards which neighbouring
//! hue), and by how many corrections it took.

use crate::maths::colour::{Difference, Rgb8, from_hsv};
use crate::maths::random::Random;
use crate::records::metric::{Metric, Metrics};

/// How pure a swatch is: its saturation band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purity {
    /// Nearly grey, 0.04 to 0.14.
    Grey,
    /// 0.14 to 0.34.
    Muted,
    /// 0.34 to 0.62.
    Middle,
    /// 0.62 to 0.95.
    Vivid,
}

/// How light a swatch is: its value band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lightness {
    /// 0.18 to 0.42.
    Dark,
    /// 0.42 to 0.70.
    Middle,
    /// 0.70 to 0.95.
    Light,
}

/// The band a swatch's colour is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tone {
    pub purity: Purity,
    pub lightness: Lightness,
}

impl Tone {
    pub const PURITIES: [Purity; 4] = [Purity::Grey, Purity::Muted, Purity::Middle, Purity::Vivid];
    pub const LIGHTNESSES: [Lightness; 3] = [Lightness::Dark, Lightness::Middle, Lightness::Light];

    /// One byte for storing: purity times three plus lightness.
    #[must_use]
    pub fn code(self) -> u8 {
        let purity = Self::PURITIES.iter().position(|p| *p == self.purity);
        let lightness = Self::LIGHTNESSES.iter().position(|l| *l == self.lightness);
        (purity.unwrap_or(0) * 3 + lightness.unwrap_or(0)) as u8
    }

    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        let code = usize::from(code);
        Some(Self {
            purity: *Self::PURITIES.get(code / 3)?,
            lightness: *Self::LIGHTNESSES.get(code % 3)?,
        })
    }

    /// The swatch's colour of this tone that `seed` makes: any hue, saturation and value in the
    /// tone's bands.
    #[must_use]
    pub fn colour(self, seed: u64) -> Rgb8 {
        let mut random = Random::new(seed);
        let hue = random.range(0.0, 360.0);
        let [low, high] = match self.purity {
            Purity::Grey => [0.04, 0.14],
            Purity::Muted => [0.14, 0.34],
            Purity::Middle => [0.34, 0.62],
            Purity::Vivid => [0.62, 0.95],
        };
        let saturation = random.range(low, high);
        let [low, high] = match self.lightness {
            Lightness::Dark => [0.18, 0.42],
            Lightness::Middle => [0.42, 0.70],
            Lightness::Light => [0.70, 0.95],
        };
        from_hsv([hue, saturation, random.range(low, high)])
    }
}

/// The colours laid in the empty frame, oldest first: each fill's colour and each paint
/// stroke's colour, as chosen (not as mixed with what lies under a transparent stroke).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColourCopy {
    pub picks: Vec<Rgb8>,
}

/// The numbers of one match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourScore {
    pub first: Difference,
    pub last: Difference,
    /// How often the colour laid changed from one pick to the next.
    pub corrections: u32,
}

impl ColourScore {
    /// The score as stored.
    #[must_use]
    pub fn metrics(&self) -> Metrics {
        let mut metrics = Vec::with_capacity(9);
        let firsts = [
            Metric::FirstDelta,
            Metric::FirstLightness,
            Metric::FirstChroma,
            Metric::FirstHue,
        ];
        let finals = [
            Metric::FinalDelta,
            Metric::FinalLightness,
            Metric::FinalChroma,
            Metric::FinalHue,
        ];
        for (names, d) in [(firsts, self.first), (finals, self.last)] {
            let values = [d.delta, d.lightness, d.chroma, d.hue];
            metrics.extend(names.into_iter().zip(values));
        }
        metrics.push((Metric::Corrections, f64::from(self.corrections)));
        metrics
    }
}

/// `copy` against the swatch `original`; `None` when nothing was laid.
#[must_use]
pub fn score_colour(original: Rgb8, copy: &ColourCopy) -> Option<ColourScore> {
    let (&first, &last) = (copy.picks.first()?, copy.picks.last()?);
    let corrections = copy
        .picks
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .count();
    Some(ColourScore {
        first: Difference::of(original, first),
        last: Difference::of(original, last),
        corrections: u32::try_from(corrections).unwrap_or(u32::MAX),
    })
}
