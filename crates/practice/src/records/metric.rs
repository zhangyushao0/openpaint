//! Every number a result holds, with its stable code: what is stored, summed up and, later,
//! followed over lessons. Values are kept in natural units: colour differences in Oklab units
//! (0 to about 1), shares as shares, counts as counts.
//!
//! Lines and shapes have no numbers (D99: a number that does not match what the eye sees misleads;
//! the overlay is the feedback). Colours, colour combinations and value scales do (D105, D108);
//! estimates and points in space keep their exact errors for the trajectories, never shown
//! (D107); a division its lines' counts and lengths per level, for its card's table and later
//! trajectories (S95); value masses the share painted at the answer's value (S96); a key shift how
//! many areas each of its checks marked (S107); abstract analysis what its card's choices held
//! (S112). A code is never given to another metric; one this version does not know is refused.

use crate::compositions::levels::Level;

/// One measured quantity of a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    /// Colour: the first colour laid against the original: the Oklab difference and its parts,
    /// lightness, chroma and hue, signed (`Difference`).
    FirstDelta,
    FirstLightness,
    FirstChroma,
    FirstHue,
    /// Colour: the colour laid at the first Compare, as the first.
    FinalDelta,
    FinalLightness,
    FinalChroma,
    FinalHue,
    /// Colour: how often the colour laid changed after the first, before the first Compare.
    Corrections,
    /// Colour combination: the mean Oklab difference over the areas painted, the weak
    /// contrasts the original holds and how many of them the copy made strong (`areas_read.rs`).
    AreasDelta,
    AreasWeak,
    AreasMadeStrong,
    /// Value scale: the mean lightness (Oklab `L`) off the even scale over the cells filled, the
    /// cells a quarter of a step off or more and the cells out of order (`scale.rs`).
    ScaleOff,
    ScaleUneven,
    ScaleReversed,
    /// A point placed: the copy's mark off the answer across and down (frame units, signed).
    PointAcross,
    PointDown,
    /// A proportion: the mark off the division along its segment (a share of it, signed).
    ProportionOff,
    /// An angle: the line drawn turned off the original's (degrees, clockwise positive).
    AngleOff,
    /// A point in space: the copy off the answer, a share of the face (`space_check.rs`).
    SpaceOff,
    /// A division's level (S95, a whole copy's own division too): its lines drawn, and how far
    /// they run on the mean, a share of the picture's longer side (`division.rs`).
    DivisionLines(Level),
    DivisionLength(Level),
    /// Value masses (S96): the share of the picture painted at the answer's value
    /// (`value_masses_read.rs`).
    MassesMatched,
    /// A key shift (S107): the areas whose lightness left the original's, those that did not move
    /// toward the tendency and the neighbours apart that merged (`key_shift_check.rs`).
    KeyLightness,
    KeyUnmoved,
    KeyMerged,
    /// Abstract analysis (S112): what the card's choices held at the first Compare, the eye before
    /// the answer: the contrasts chosen as a bit set (`Choices::chips`) and the distribution
    /// chosen (`Choices::shape`, 0 to 3).
    ChosenChips,
    ChosenShape,
}

impl Metric {
    pub const ALL: [Self; 32] = [
        Self::FirstDelta,
        Self::FirstLightness,
        Self::FirstChroma,
        Self::FirstHue,
        Self::FinalDelta,
        Self::FinalLightness,
        Self::FinalChroma,
        Self::FinalHue,
        Self::Corrections,
        Self::AreasDelta,
        Self::AreasWeak,
        Self::AreasMadeStrong,
        Self::ScaleOff,
        Self::ScaleUneven,
        Self::ScaleReversed,
        Self::PointAcross,
        Self::PointDown,
        Self::ProportionOff,
        Self::AngleOff,
        Self::SpaceOff,
        Self::DivisionLines(Level::Main),
        Self::DivisionLength(Level::Main),
        Self::DivisionLines(Level::Second),
        Self::DivisionLength(Level::Second),
        Self::DivisionLines(Level::Third),
        Self::DivisionLength(Level::Third),
        Self::MassesMatched,
        Self::KeyLightness,
        Self::KeyUnmoved,
        Self::KeyMerged,
        Self::ChosenChips,
        Self::ChosenShape,
    ];

    /// The code it is stored by; never reused.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::FirstDelta => 20,
            Self::FirstLightness => 36,
            Self::FirstChroma => 37,
            Self::FirstHue => 38,
            Self::FinalDelta => 24,
            Self::FinalLightness => 39,
            Self::FinalChroma => 40,
            Self::FinalHue => 41,
            Self::Corrections => 28,
            Self::AreasDelta => 29,
            Self::AreasWeak => 30,
            Self::AreasMadeStrong => 31,
            Self::ScaleOff => 35,
            Self::ScaleUneven => 33,
            Self::ScaleReversed => 34,
            Self::PointAcross => 42,
            Self::PointDown => 43,
            Self::ProportionOff => 44,
            Self::AngleOff => 45,
            Self::SpaceOff => 46,
            Self::DivisionLines(Level::Main) => 49,
            Self::DivisionLength(Level::Main) => 50,
            Self::DivisionLines(Level::Second) => 51,
            Self::DivisionLength(Level::Second) => 52,
            Self::DivisionLines(Level::Third) => 53,
            Self::DivisionLength(Level::Third) => 54,
            Self::MassesMatched => 55,
            Self::KeyLightness => 56,
            Self::KeyUnmoved => 57,
            Self::KeyMerged => 58,
            Self::ChosenChips => 59,
            Self::ChosenShape => 60,
        }
    }

    /// The metric stored as `code`, if this version knows it.
    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|metric| metric.code() == code)
    }
}

/// A result's numbers, each metric at most once.
pub type Metrics = Vec<(Metric, f64)>;

/// The value of `metric` in `metrics`.
#[must_use]
pub fn value(metrics: &[(Metric, f64)], metric: Metric) -> Option<f64> {
    metrics
        .iter()
        .find(|(held, _)| *held == metric)
        .map(|(_, value)| *value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_distinct_and_read_back_and_others_are_unknown() {
        for metric in Metric::ALL {
            assert_eq!(Metric::of_code(metric.code()), Some(metric));
        }
        let mut codes: Vec<u8> = Metric::ALL.iter().map(|m| m.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Metric::ALL.len());
        let unknown = (0..=u8::MAX).filter(|code| !codes.contains(code));
        for code in unknown {
            assert_eq!(Metric::of_code(code), None, "code {code}");
        }
    }
}
