//! Every number a result holds, with its stable code: what is stored, summed up and, later,
//! followed over sessions. Values are kept in natural units: distances in frame sides, speed in
//! frame sides a second, times in seconds, hue in degrees, saturation and value as shares
//! (0 to 1), colour differences in Oklab units (0 to about 1), counts as counts.

/// One measured quantity of a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    /// Line: the mean distance of the original's key points (its ends and most bulging points)
    /// from the nearest point of the copy.
    KeyPoints,
    /// Line: the symmetric mean distance between copy and original as placed in their frames.
    Frame,
    /// Line: the same after the copy is moved and sized (never turned) onto the original.
    Shape,
    /// Line: the paint strokes' length over the time the pen was down drawing them.
    Speed,
    /// Line: the root mean square distance of the copy from its own local smoothing.
    Wobble,
    /// Line: paint strokes drawn in the frame.
    Strokes,
    /// Line: eraser strokes in the frame.
    Erased,
    /// Line: seconds the pen was down drawing the paint strokes.
    Seconds,
    /// Colour: the first colour laid against the original: Oklab difference, hue (degrees,
    /// signed, only when both have colour), saturation and value (signed shares).
    FirstDelta,
    FirstHue,
    FirstSaturation,
    FirstValue,
    /// Colour: the colour laid last, as the first.
    FinalDelta,
    FinalHue,
    FinalSaturation,
    FinalValue,
    /// Colour: how often the colour laid changed after the first.
    Corrections,
}

impl Metric {
    pub const ALL: [Self; 17] = [
        Self::KeyPoints,
        Self::Frame,
        Self::Shape,
        Self::Speed,
        Self::Wobble,
        Self::Strokes,
        Self::Erased,
        Self::Seconds,
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
            Self::KeyPoints => 1,
            Self::Frame => 2,
            Self::Shape => 3,
            Self::Speed => 4,
            Self::Wobble => 5,
            Self::Strokes => 6,
            Self::Erased => 7,
            Self::Seconds => 8,
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
    fn codes_are_distinct_and_read_back() {
        for metric in Metric::ALL {
            assert_eq!(Metric::of_code(metric.code()), Some(metric));
        }
        let mut codes: Vec<u8> = Metric::ALL.iter().map(|m| m.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Metric::ALL.len());
        assert_eq!(Metric::of_code(0), None);
    }
}
