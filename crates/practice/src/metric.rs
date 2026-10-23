//! Every number a result holds, with its stable code: what is stored, summed up and, later,
//! followed over sessions. Values are kept in natural units: hue in degrees, saturation and value
//! as shares (0 to 1), colour differences in Oklab units (0 to about 1), counts as counts.
//!
//! Lines have no numbers (D99: a number that does not match what the eye sees misleads; the
//! overlay is the feedback). Codes 1 to 8 were 1.1.0's line metrics (key points, frame, shape,
//! speed, wobble, strokes, erased, seconds): retired, never reused, passed over when read.

/// One measured quantity of a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    /// Colour: the first colour laid against the original: Oklab difference, hue (degrees,
    /// signed, only when both have colour), saturation and value (signed shares).
    FirstDelta,
    FirstHue,
    FirstSaturation,
    FirstValue,
    /// Colour: the colour laid at the first Compare, as the first.
    FinalDelta,
    FinalHue,
    FinalSaturation,
    FinalValue,
    /// Colour: how often the colour laid changed after the first, before the first Compare.
    Corrections,
}

impl Metric {
    pub const ALL: [Self; 9] = [
        Self::FirstDelta,
        Self::FirstHue,
        Self::FirstSaturation,
        Self::FirstValue,
        Self::FinalDelta,
        Self::FinalHue,
        Self::FinalSaturation,
        Self::FinalValue,
        Self::Corrections,
    ];

    /// The code it is stored by; never reused.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::FirstDelta => 20,
            Self::FirstHue => 21,
            Self::FirstSaturation => 22,
            Self::FirstValue => 23,
            Self::FinalDelta => 24,
            Self::FinalHue => 25,
            Self::FinalSaturation => 26,
            Self::FinalValue => 27,
            Self::Corrections => 28,
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
    fn codes_are_distinct_and_read_back_and_the_retired_ones_are_unknown() {
        for metric in Metric::ALL {
            assert_eq!(Metric::of_code(metric.code()), Some(metric));
        }
        let mut codes: Vec<u8> = Metric::ALL.iter().map(|m| m.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Metric::ALL.len());
        for retired in 0..=8 {
            assert_eq!(Metric::of_code(retired), None, "code {retired}");
        }
    }
}
